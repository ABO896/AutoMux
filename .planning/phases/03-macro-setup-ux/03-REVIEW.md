---
phase: 03-macro-setup-ux
reviewed: 2026-05-30T12:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/App.tsx
  - src/keymap.ts
findings:
  critical: 1
  warning: 2
  info: 0
  total: 3
status: issues_found
---

# Phase 3 (Gap-Closure 03-04): Code Review Report

**Reviewed:** 2026-05-30T12:00:00Z
**Depth:** standard
**Files Reviewed:** 2 (`src/App.tsx`, `src/keymap.ts`)
**Status:** issues_found

## Summary

This re-review covers only the gap-closure changes introduced by plan 03-04. The three targeted fixes (CR-02, CR-03, WR-02) were applied correctly in structure — `domKeycodeToNative` now returns `number | null`, `handleCardSetTriggerKey` on macOS now calls `set_macro_trigger_key` as the third invoke, and the outer `Show` fallback renders a "Set key…" placeholder. However, one critical regression was introduced: the `=== 0` half of the null guard in `startCapture` blocks the letter A (CGKeyCode 0 on macOS) from ever being used as a trigger key. Two warnings cover missing capture-active feedback in the fallback branch and a residual signal contamination risk between card and form capture.

---

## Critical Issues

### CR-01: `nativeCode === 0` guard permanently blocks KeyA as a trigger key on macOS

**File:** `src/App.tsx:270`

**Issue:** The guard added to fix CR-03 reads:

```typescript
if (nativeCode === null || nativeCode === 0) return;
```

The `=== null` check is correct — it rejects keys not present in the lookup table. The `=== 0` check is a regression. On macOS, `DOM_KEYCODE_TO_CGKEYCODE["KeyA"]` is `0` (CGKeyCode 0 = A per `HIToolbox/Events.h`). The guard fires for KeyA, silently drops it, and keeps capture active — making it impossible for a user to bind the letter A as a trigger key on macOS.

Before this gap-closure, CR-03 described the problem as: "pressing an unmapped key silently assigns CGKeyCode 0." The intended fix was to return `null` for unmapped keys, which was correctly done in `keymap.ts`. The `=== 0` guard was added as belt-and-suspenders, but now that unmapped keys return `null` (not `0`), the only path that can reach `=== 0` is a legitimate KeyA press.

The `=== 0` clause is also dead code on Windows because `DOM_KEYCODE_TO_VK["KeyA"]` is `0x41` (65), and no entry in `DOM_KEYCODE_TO_VK` maps to `0`.

**Fix:** Remove the `=== 0` clause. The `null` check alone is sufficient now that `domKeycodeToNative` returns `null` for misses:

```typescript
const nativeCode = domKeycodeToNative(e.code);
if (nativeCode === null) return;
onCommit(nativeCode);
```

---

## Warnings

### WR-01: "Set key…" fallback shows no capture-active visual state and has no cancel control

**File:** `src/App.tsx:781–792`

**Issue:** When a user clicks the "Set key…" placeholder on a key-less macro card, `startCapture` attaches the keydown listener and sets `setTriggerKeyRecording(true)`, but the fallback branch has no conditional rendering driven by `editingCardId()`/`editingField()`. The card continues to display the static "Set key…" text with no indication that capture is active. There is also no cancel button (✕) in the fallback branch — the only way to abort capture is to press Escape.

The `editingCardId` and `editingField` signals are correctly set, but neither is used inside the fallback branch's JSX. The "Press…" / ✕ indicator exists only in the `when=true` branch (lines 810–824), which is unreachable when `trigger_key` is `null`.

Additionally, `startCapture` sets the shared `triggerKeyRecording` signal to `true`. If the "New Macro" form is open simultaneously, the form's key button will flip to "Press a key…" text even though the form capture was not initiated — a cross-contamination of visual state between the card fallback and the new-macro form.

**Fix:** Add a capture-in-progress inner Show to the fallback branch, driven by `editingCardId() === macro.id && editingField() === "key"`:

```tsx
<Show when={macro.trigger_key !== null} fallback={
  <div class="flex items-center gap-1">
    <Show
      when={editingCardId() === macro.id && editingField() === "key"}
      fallback={
        <span
          class="px-1.5 py-0.5 rounded border border-dashed border-border text-[10px] font-mono text-text-dim cursor-pointer hover:border-accent/40 hover:text-text-main"
          onClick={() => {
            setEditingCardId(macro.id);
            setEditingField("key");
            startCapture((nativeCode) => handleCardSetTriggerKey(macro.id, nativeCode));
          }}
        >Set key…</span>
      }
    >
      <span class="px-1.5 py-0.5 rounded border border-accent text-[10px] font-mono text-accent shadow-[0_0_4px_var(--color-accent-glow)] flex items-center gap-1">
        Press…
        <span
          class="text-text-dim hover:text-text-main leading-none cursor-pointer"
          onClick={() => {
            setEditingCardId(null);
            setEditingField(null);
            if (_keyCaptureListener) {
              document.removeEventListener("keydown", _keyCaptureListener, true);
              _keyCaptureListener = null;
            }
            setTriggerKeyRecording(false);
          }}
        >✕</span>
      </span>
    </Show>
    <span class="text-[10px] text-text-muted">({macro.trigger_mode})</span>
  </div>
}>
```

---

### WR-02: macOS `handleCardSetTriggerKey` is non-atomic — failed `bind_hotkey` leaves macro with orphaned `trigger_key` and no active binding

**File:** `src/App.tsx:283–286`

**Issue:** The three sequential awaits in the macOS branch are not rolled back on partial failure:

```typescript
await invoke("unbind_hotkey", { macro_id: id });           // (1) old binding removed
await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers: 0 }); // (2) may fail
await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode });           // (3) skipped if (2) throws
```

If `bind_hotkey` at step (2) fails (e.g., key already claimed by the OS or another app), the `catch` block fires. At that point: the old hotkey binding has been removed from `HOTKEY_BINDINGS` (step 1), the new binding was never added, and `MacroConfig.trigger_key` still holds the old key value (step 3 was skipped). The result is a macro whose UI badge shows the old key name (because trigger_key is unchanged in AppState), but the key does nothing when pressed (because HOTKEY_BINDINGS no longer contains it). There is no error surfaced to the user — `console.error` writes to DevTools only.

This non-atomicity existed before plan 03-04 (only two invokes were there previously), and the plan adding the third invoke made the failure window larger. It is not a new regression per se, but it is the first time the gap-closure review can flag it clearly.

**Fix (minimal):** On `bind_hotkey` failure, attempt to re-bind the original key to restore the previous state, and surface an error message to the user:

```typescript
async function handleCardSetTriggerKey(id: string, nativeCode: number) {
  const previousKey = state()?.macros[id]?.trigger_key ?? null;
  try {
    if (IS_MACOS) {
      await invoke("unbind_hotkey", { macro_id: id });
      try {
        await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers: 0 });
      } catch (bindErr) {
        // Restore the previous binding to avoid orphaning trigger_key
        if (previousKey !== null) {
          await invoke("bind_hotkey", { macro_id: id, keycode: previousKey, modifiers: 0 }).catch(() => {});
        }
        throw bindErr;
      }
      await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode });
    } else {
      await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode });
    }
    setEditingCardId(null);
    setEditingField(null);
  } catch (e) {
    console.error("Card trigger key update failed:", e);
    // Surface error to user (e.g., via a transient toast signal)
  }
}
```

---

_Reviewed: 2026-05-30T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
