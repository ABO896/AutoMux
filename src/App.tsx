import { createSignal, createEffect, onCleanup, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import { domKeycodeToNative, resolveKeyName } from "./keymap";
import { getStoredPreference, applyTheme, type ThemePreference } from "./theme";
import Sidebar from "./components/Sidebar";
import ThemeToggle from "./components/ThemeToggle";
import MacroForm, { type MacroFormSubmitValues } from "./components/MacroForm";
import MacroCard from "./components/MacroCard";
import "./App.css";

// ── Types (mirrors Rust state) ──────────────────────────────────

export type TriggerMode = "Pulse" | "Hold";

export interface MacroConfig {
  id: string;
  name: string;
  interval_ms: number;
  enabled: boolean;
  target_app: string | null;
  sequence: ActionSequence;
  trigger_key: number | null;
  // UX-13: Platform-native modifier bitmask paired with trigger_key. Values
  // are CGEventFlags bits on macOS and MOD_* on Windows — see computeModifiers
  // in this file for the exact mapping. Mirrors Rust MacroConfig.trigger_modifiers.
  trigger_modifiers: number;
  trigger_mode: TriggerMode;
}

type ActionStep =
  | { SustainedHold: { input: InputEvent } }
  | { InterleavedInterval: { input: InputEvent; interval_ms: number } };

// 10-04 Task 2: named alias for MacroConfig's inline `{ steps: ActionStep[] }`
// shape — exported so MacroForm.tsx can build/type the value it assembles
// and passes back via its onSubmit callback, without duplicating the shape.
export type ActionSequence = { steps: ActionStep[] };

export type InputEvent =
  | { MouseButton: "Left" | "Right" | "Middle" }
  | { Key: number };

interface AppState {
  macros: Record<string, MacroConfig>;
  emergency_stop_active: boolean;
  active_app: string | null;
  engine_active: boolean;
  // UX-12: derived list of input-event conflicts between currently enabled
  // macros. Populated by `recompute_conflicts()` after every state-mutating
  // intent. Mirrors Rust AppState.conflicts: Vec<InputConflict>.
  conflicts: Array<{ macros: string[]; input: InputEvent }>;
}

interface ProfileSummary {
  name: string;
  macro_count: number;
}

interface ProfileData {
  name: string;
  macros: Record<string, MacroConfig>;
  engine_active: boolean;
}

export interface RunningApp {
  display_name: string;
  identifier: string;
}

// ── Helpers ─────────────────────────────────────────────────────

function formatInputEvent(ev: InputEvent): string {
  if ("MouseButton" in ev) return `🖱 ${ev.MouseButton}`;
  if ("Key" in ev) return `⌨ Key(${ev.Key})`;
  return "?";
}

export function formatStep(step: ActionStep): string {
  if ("SustainedHold" in step)
    return `Hold ${formatInputEvent(step.SustainedHold.input)}`;
  if ("InterleavedInterval" in step)
    return `${formatInputEvent(step.InterleavedInterval.input)} every ${step.InterleavedInterval.interval_ms}ms`;
  return "?";
}

export type RunningState = "firing" | "held" | "combined" | "waiting" | "disabled";

/**
 * EXEC-01/EXEC-02 (D-01–D-04): Pure derivation of a macro's visible running
 * state. Mirrors the 3 gates in Rust `StateActor::handle_action`
 * (src-tauri/src/state/mod.rs:373-394) — engine active + not
 * emergency-stopped; macro enabled; target app matches active app (or is
 * Global) — plus a 4th SustainedHold/InterleavedInterval distinction so the
 * card can show held vs firing vs combined. Frontend-only derivation: no
 * new backend field, no new signal (D-04).
 */
function computeRunningState(macro: MacroConfig, state: AppState): RunningState {
  if (!macro.enabled) return "disabled";
  if (!state.engine_active || state.emergency_stop_active) return "disabled";

  const matchesTarget =
    macro.target_app == null || state.active_app === macro.target_app;
  if (!matchesTarget) return "waiting";

  // CR-01 fix: the scheduler force-converts EVERY step to SustainedHold when
  // trigger_mode is Hold (src-tauri/src/scheduler/mod.rs:287-294), and the
  // empty-sequence Hold fallback also yields a SustainedHold
  // (scheduler/mod.rs:279-281) — so a Hold-mode macro is entirely held at
  // runtime regardless of what sequence.steps persists. handleCreateMacro
  // always persists a single InterleavedInterval step (src/App.tsx), so the
  // persisted step shape alone must not be the sole discriminant here.
  if (macro.trigger_mode === "Hold") return "held";

  // Legacy empty-sequence macros fall back to a single interval click
  // (see src-tauri/src/scheduler/mod.rs:257-270) — treat as "firing".
  if (macro.sequence.steps.length === 0) return "firing";

  const hasHold = macro.sequence.steps.some((s) => "SustainedHold" in s);
  const hasInterval = macro.sequence.steps.some((s) => "InterleavedInterval" in s);
  if (hasHold && hasInterval) return "combined";
  if (hasHold) return "held";
  return "firing";
}

/**
 * UX-13: Extract a platform-native modifier bitmask from a KeyboardEvent.
 *
 * macOS: CGEventFlags bits (matching what the CGEventTap callback stores
 *   in `flags.bits()`). Values pinned by `cg_event_flag_constants` test in
 *   `src-tauri/src/platform/macos/observer.rs` (08-01 Task 2).
 *
 *   Shift   = 0x020000
 *   Control = 0x040000
 *   Option  = 0x080000
 *   Command = 0x100000
 *
 * Windows: Win32 MOD_* values. Values pinned by `windows_mod_constants`
 *   test in `src-tauri/src/platform/windows/mod.rs` (08-01 Task 2). The
 *   Windows `hook_callback` synthesizes this same bitmask from
 *   `GetAsyncKeyState` via `build_mod_mask()` (08-03 Task 3).
 *
 *   MOD_ALT     = 0x0001
 *   MOD_CONTROL = 0x0002
 *   MOD_SHIFT   = 0x0004
 *   MOD_WIN     = 0x0008
 *
 * Module-level so the helper is callable from outside the App() closure.
 * The inline `IS_MACOS` check mirrors the constant declared inside App()
 * (line 131) — same detection, different scope, identical bit output.
 */
function computeModifiers(e: KeyboardEvent): number {
  const IS_MACOS = navigator.userAgent.toLowerCase().includes("mac");
  if (IS_MACOS) {
    let m = 0;
    if (e.shiftKey) m |= 0x020000;
    if (e.ctrlKey)  m |= 0x040000;
    if (e.altKey)   m |= 0x080000;
    if (e.metaKey)  m |= 0x100000;
    return m;
  } else {
    let m = 0;
    if (e.shiftKey) m |= 0x0004;
    if (e.ctrlKey)  m |= 0x0002;
    if (e.altKey)   m |= 0x0001;
    if (e.metaKey)  m |= 0x0008; // Win key
    return m;
  }
}

/**
 * UX-13 (C-5): Map a platform-native modifier bitmask to the list of label
 * strings to render as preview chips. Order is SEMANTIC (Shift → Ctrl → Alt →
 * Cmd/Win), independent of press order. Bits not present in `bits` are
 * omitted from the result.
 *
 * Bit values mirror computeModifiers above and the Rust constants pinned
 * by `cg_event_flag_constants` and `windows_mod_constants` tests in 08-01.
 */
function modifierChips(bits: number): string[] {
  const IS_MACOS = navigator.userAgent.toLowerCase().includes("mac");
  if (IS_MACOS) {
    const order: Array<[number, string]> = [
      [0x020000, "Shift"],
      [0x040000, "Ctrl"],
      [0x080000, "Option"],
      [0x100000, "⌘"],
    ];
    return order.filter(([bit]) => (bits & bit) === bit).map(([, label]) => label);
  } else {
    const order: Array<[number, string]> = [
      [0x0004, "Shift"],
      [0x0002, "Ctrl"],
      [0x0001, "Alt"],
      [0x0008, "Win"],
    ];
    return order.filter(([bit]) => (bits & bit) === bit).map(([, label]) => label);
  }
}

// ── App ─────────────────────────────────────────────────────────

export type Tab = "dashboard" | "profiles";

// Module-level listener ref — survives across renders; prevents double-attach (T-03-08)
let _keyCaptureListener: ((e: KeyboardEvent) => void) | null = null;

function App() {
  // WR-01: Scoped inside App() so it does not outlive the component instance
  // during HMR remounts. The existing onCleanup → clearPending() path handles teardown.
  let _pendingTimeoutId: ReturnType<typeof setTimeout> | null = null;
  let _imPendingTimeoutId: ReturnType<typeof setTimeout> | null = null;

  const [state, setState] = createSignal<AppState | null>(null);
  const [accessibility, setAccessibility] = createSignal<boolean | null>(null);
  const [accessibilityPending, setAccessibilityPending] = createSignal(false);
  const [inputMonitoring, setInputMonitoring] = createSignal<boolean | null>(null);
  const [inputMonitoringPending, setInputMonitoringPending] = createSignal(false);
  const [saveError, setSaveError] = createSignal(false);
  const [tccIdentityChanged, setTccIdentityChanged] = createSignal(false);
  const [activeApp, setActiveApp] = createSignal<string | null>(null);
  const [, setLoading] = createSignal(true);
  const [activeTab, setActiveTab] = createSignal<Tab>("dashboard");

  // ── Profile State ──
  const [profiles, setProfiles] = createSignal<ProfileSummary[]>([]);
  const [activeProfile, setActiveProfile] = createSignal<string>("Default");
  const [newProfileName, setNewProfileName] = createSignal("");
  const [profileLoading, setProfileLoading] = createSignal(false);
  const [profileMessage, setProfileMessage] = createSignal<{
    text: string;
    type: "success" | "error";
  } | null>(null);

  // UX-11: populated when a bind/create IPC returns a conflict error string
  // (built by `check_trigger_key_conflict` in the StateActor). The C-1
  // ConflictErrorToast (Plan 08-05 Task 1) renders this — the wiring
  // (signal + helper) shipped in 08-04 so the toast can be added without
  // touching the call sites again. 8-second auto-dismiss per UI-SPEC C-1.
  const [conflictError, setConflictError] = createSignal<{
    key: string;
    macroName: string;
  } | null>(null);
  let _conflictErrorTimer: ReturnType<typeof setTimeout> | null = null;
  // UX-12 (C-2): session-only dismiss flag for the conflict warning region.
  // Resets implicitly when state().conflicts.length returns to 0 because
  // the <Show> predicate re-evaluates. No explicit reset needed.
  const [conflictsDismissed, setConflictsDismissed] = createSignal(false);
  function showConflictError(key: string, macroName: string) {
    if (_conflictErrorTimer !== null) clearTimeout(_conflictErrorTimer);
    setConflictError({ key, macroName });
    _conflictErrorTimer = setTimeout(() => {
      setConflictError(null);
      _conflictErrorTimer = null;
    }, 8000);
  }

  const [appVersion, setAppVersion] = createSignal<string>("…");

  // D-06/D-07: theme preference, seeded from localStorage (boot script in
  // index.html already applied the resolved data-theme before first paint;
  // this signal is the runtime source of truth for the live matchMedia
  // follow effect below and the sidebar ThemeToggle control).
  const [themePreference, setThemePreference] = createSignal<ThemePreference>(getStoredPreference());

  // D-06/D-07: advances the 3-state theme cycle System → Light → Dark →
  // System, applies it (data-theme swap + localStorage persist via theme.ts)
  // and mirrors the new value into the signal for the ThemeToggle glyph.
  function cycleThemePreference() {
    const order: ThemePreference[] = ["system", "light", "dark"];
    const next = order[(order.indexOf(themePreference()) + 1) % order.length];
    applyTheme(next);
    setThemePreference(next);
  }

  // ── New Macro Form State ──
  const [showNewMacro, setShowNewMacro] = createSignal(false);
  const [newMacroName, setNewMacroName] = createSignal("");
  const [newMacroInterval, setNewMacroInterval] = createSignal("100");
  const [newMacroInput, setNewMacroInput] = createSignal("Left");
  const [newMacroTarget, setNewMacroTarget] = createSignal("");
  const [newMacroTriggerMode, setNewMacroTriggerMode] = createSignal<TriggerMode>("Pulse");
  const [newMacroTriggerKeyCode, setNewMacroTriggerKeyCode] = createSignal<number | null>(null);
  // UX-13: modifier bits captured alongside the new-macro form's trigger key.
  // Read by handleCreateMacro when building the MacroConfig for add_macro.
  const [newMacroTriggerModifiers, setNewMacroTriggerModifiers] = createSignal<number>(0);
  const [triggerKeyRecording, setTriggerKeyRecording] = createSignal(false);
  // UX-13 (C-5): tracks the modifier bits held by the user during key capture.
  // Set in startCapture's onKeyDown; cleared on commit. The C-5 chip in
  // Plan 08-05 Task 2 reads this signal to render the modifier preview row.
  const [recordingModifiers, setRecordingModifiers] = createSignal<number>(0);
  // D-11/D-12: the new-macro form's Key Press action-input capture state —
  // distinct from the trigger-key capture state above so the two can
  // coexist on the same form (MacroForm renders both KeyCaptureField
  // instances) without one clobbering the other's committed value.
  // No modifiers signal: InputEvent::Key persists a bare u16 keycode (unlike
  // the trigger-key slot, which pairs a keycode with a modifier bitmask for
  // hotkey conflict resolution) — startCapture's `modifiers` callback arg is
  // intentionally discarded at this call site (see onActionKeyStartCapture).
  const [newMacroActionKeyCode, setNewMacroActionKeyCode] = createSignal<number | null>(null);
  // Since `triggerKeyRecording`/`_keyCaptureListener` are shared/global (only
  // one capture may record at a time, T-03-08), this tracks WHICH of the
  // form's two capture slots is the one currently recording, so each
  // KeyCaptureField instance can compute its own `recording` accessor.
  const [formCapturingSlot, setFormCapturingSlot] = createSignal<"trigger" | "action" | null>(null);
  // UX-14 (C-3): first-run banner visibility. Gated by
  // localStorage.automux.hotkey_global_notice_dismissed !== "1" inside
  // the initial-fetch createEffect (next to setAppVersion).
  const [showGlobalNotice, setShowGlobalNotice] = createSignal<boolean>(false);

  // ── Process Picker State ──
  const [apps, setApps] = createSignal<RunningApp[]>([]);
  const [appsLoading, setAppsLoading] = createSignal(false);
  const [appsError, setAppsError] = createSignal(false);

  // ── Card Inline Edit State (D-13, UX-09 full field set) ──
  // Centralized in App(), NOT per-card local state (RESEARCH.md Pattern 1) —
  // opening edit on card B while card A is mid-edit simply reassigns
  // editingCardId and repopulates these shared signals from B's macro
  // values, which discards A's unsaved edits for free (mutual exclusion).
  const [editingCardId, setEditingCardId] = createSignal<string | null>(null);
  const [editMacroName, setEditMacroName] = createSignal("");
  const [editMacroInput, setEditMacroInput] = createSignal("Left");
  const [editMacroMode, setEditMacroMode] = createSignal<TriggerMode>("Pulse");
  const [editMacroInterval, setEditMacroInterval] = createSignal("100");
  const [editMacroTarget, setEditMacroTarget] = createSignal("");
  const [editMacroTriggerKeyCode, setEditMacroTriggerKeyCode] = createSignal<number | null>(null);
  const [editMacroTriggerModifiers, setEditMacroTriggerModifiers] = createSignal<number>(0);
  const [editMacroActionKeyCode, setEditMacroActionKeyCode] = createSignal<number | null>(null);
  // Mirrors the New Macro form's formCapturingSlot — since triggerKeyRecording
  // / _keyCaptureListener are shared/global, this tracks which of the edit
  // form's two capture slots is the one currently recording.
  const [editFormCapturingSlot, setEditFormCapturingSlot] = createSignal<"trigger" | "action" | null>(null);

  // ── Initial data fetch ──
  // WR-08: `cancelled` flag prevents stale setters from firing after unmount.
  createEffect(() => {
    let cancelled = false;
    onCleanup(() => { cancelled = true; });

    (async () => {
      try {
        const [stateData, accessOk, imOk, tccChanged, app, profileList, version] = await Promise.all([
          invoke<AppState>("get_state"),
          invoke<boolean>("check_accessibility"),
          invoke<boolean>("check_input_monitoring"),
          invoke<boolean>("get_tcc_identity_status"),
          invoke<string | null>("get_active_app"),
          invoke<ProfileSummary[]>("list_profiles"),
          getVersion(),
        ]);
        if (cancelled) return;
        setState(stateData);
        setAccessibility(accessOk);
        setInputMonitoring(imOk);
        setTccIdentityChanged(tccChanged);
        setActiveApp(app);
        setProfiles(profileList);
        setAppVersion(version);
        // UX-14 (C-3): first-run banner gated by localStorage. The user
        // dismisses via "Got it" which sets the flag; on the next launch
        // the banner is gone. Clearing app data brings it back (per
        // RESEARCH.md R-8: false-positive re-show is cheaper than
        // false-negative never-show).
        setShowGlobalNotice(
          localStorage.getItem("automux.hotkey_global_notice_dismissed") !== "1"
        );
      } catch (e) {
        if (cancelled) return;
        console.error("Failed to fetch initial state:", e);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
  });

  // Real-time state listener
  createEffect(() => {
    const unlisten = listen<AppState>("state-changed", (event) => {
      setState(event.payload);
      setActiveApp(event.payload.active_app);
    });
    onCleanup(() => {
      unlisten.then((fn) => fn());
    });
  });

  // Auto-save error listener (D-11) — shows persistent banner until dismissed.
  // D-13: the event payload is intentionally ignored; we always show the fixed
  // user-friendly copy rather than the raw Rust error string.
  createEffect(() => {
    const unlisten = listen<string>("auto-save-error", (_event) => {
      setSaveError(true);
    });
    onCleanup(() => {
      unlisten.then((fn) => fn());
    });
  });

  // D-06/D-07: live OS-appearance follow. While the preference is "system",
  // re-apply the theme whenever the OS prefers-color-scheme changes so the
  // app updates without requiring a restart. Listener is removed on cleanup
  // per CONVENTIONS.md's "always clean up subscriptions inside effects."
  createEffect(() => {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      if (themePreference() === "system") {
        applyTheme("system");
      }
    };
    mql.addEventListener("change", onChange);
    onCleanup(() => {
      mql.removeEventListener("change", onChange);
    });
  });

  // Poll accessibility AND input monitoring every 3s (D-04: single effect).
  createEffect(() => {
    const interval = setInterval(async () => {
      try {
        const [a11y, im] = await Promise.all([
          invoke<boolean>("check_accessibility"),
          invoke<boolean>("check_input_monitoring"),
        ]);
        setAccessibility(a11y);
        setInputMonitoring(im);
        // Clear pending on any definitive response (granted or denied).
        // Only remain "pending" while the dialog is actually in-flight (null).
        clearPending();
        clearImPending();
      } catch (_) {
        /* ignore */
      }
    }, 3000);
    onCleanup(() => clearInterval(interval));
  });

  // WR-03: Cancelled flag for handleRequestAccess — prevents stale setters from
  // firing after component unmount when the in-flight invoke resolves late.
  let requestAccessCancelled = false;

  // Cleanup dangling key capture listener on component unmount (T-03-08)
  onCleanup(() => {
    requestAccessCancelled = true;
    if (_keyCaptureListener) {
      document.removeEventListener("keydown", _keyCaptureListener, true);
      _keyCaptureListener = null;
    }
    clearPending();
  });

  // ── Actions ───────────────────────────────────────────────────

  function clearPending() {
    setAccessibilityPending(false);
    if (_pendingTimeoutId !== null) {
      clearTimeout(_pendingTimeoutId);
      _pendingTimeoutId = null;
    }
  }

  async function handleRequestAccess() {
    setAccessibilityPending(true);
    _pendingTimeoutId = setTimeout(() => {
      if (!requestAccessCancelled) setAccessibilityPending(false);
      _pendingTimeoutId = null;
    }, 30_000);
    try {
      const granted = await invoke<boolean>("request_accessibility");
      if (!requestAccessCancelled) {
        setAccessibility(granted);
        clearPending();
      }
    } catch (e) {
      if (!requestAccessCancelled) {
        console.error("Accessibility request failed:", e);
        clearPending();
      }
    }
  }

  function clearImPending() {
    setInputMonitoringPending(false);
    if (_imPendingTimeoutId !== null) {
      clearTimeout(_imPendingTimeoutId);
      _imPendingTimeoutId = null;
    }
  }

  async function handleRequestInputMonitoringAccess() {
    setInputMonitoringPending(true);
    // 30s timeout mirrors Accessibility flow
    _imPendingTimeoutId = setTimeout(() => {
      if (!requestAccessCancelled) setInputMonitoringPending(false);
      _imPendingTimeoutId = null;
    }, 30_000);
    try {
      await openUrl("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent");
    } catch (e) {
      if (!requestAccessCancelled) {
        console.error("Failed to open Input Monitoring settings:", e);
        setInputMonitoringPending(false);
        _imPendingTimeoutId = null;
      }
    }
  }

  async function handleToggleEngine() {
    try {
      await invoke("toggle_engine");
    } catch (e) {
      console.error("Toggle engine failed:", e);
    }
  }

  // 10-04 Task 2: MacroForm now builds the ActionSequence/InputEvent shape
  // itself (same shape this function used to build inline) and hands back
  // the assembled values via onSubmit — handleCreateMacro's job shrinks to
  // assembling the full MacroConfig and calling add_macro, unchanged from
  // before in every other respect (WR-05 placeholder id, conflict toast).
  async function handleCreateMacro(values: MacroFormSubmitValues) {
    const triggerKey = values.triggerKey;

    // WR-05: id is generated by the backend; supply a placeholder that gets overwritten.
    // UX-13: `trigger_modifiers` carries the platform-native bitmask captured
    // by computeModifiers during key capture. See PATTERNS.md for the bit map.
    const config: MacroConfig = {
      id: "00000000-0000-0000-0000-000000000000",
      name: values.name,
      interval_ms: parseInt(newMacroInterval()) || 100,
      enabled: false,
      target_app: values.targetApp,
      sequence: values.sequence,
      trigger_key: triggerKey,
      trigger_modifiers: values.triggerModifiers,
      trigger_mode: values.triggerMode,
    };

    try {
      await invoke<string>("add_macro", { config });
      setShowNewMacro(false);
      setNewMacroName("");
      setNewMacroInterval("100");
      setNewMacroInput("Left");
      setNewMacroTarget("");
      setNewMacroTriggerMode("Pulse");
      setNewMacroTriggerKeyCode(null);
      setNewMacroTriggerModifiers(0);
      setTriggerKeyRecording(false);
      setNewMacroActionKeyCode(null);
      setFormCapturingSlot(null);
    } catch (e) {
      // UX-11: surface the conflict error to the C-1 toast (Plan 08-05).
      // The backend's error format is e.g.:
      //   'Key F5 is already assigned to "AFK Farm". Unbind it first or pick a different key.'
      //   'Key (keycode 96) is already assigned to "AFK Farm". ...'
      // Extract the macro name; if the format doesn't match, fall back to a
      // generic message — the user can still act on the conflict via the
      // auto-save banner or the macros list.
      const msg = String(e);
      const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
      const macroName = macroMatch ? macroMatch[1] : "another macro";
      const keyLabel = triggerKey !== null ? resolveKeyName(triggerKey) : "Key";
      showConflictError(keyLabel, macroName);
      console.error("Failed to create macro:", e);
    }
  }

  async function handleToggleMacro(id: string, currentEnabled: boolean) {
    try {
      await invoke("set_macro_enabled", { id, enabled: !currentEnabled });
    } catch (e) {
      console.error("Toggle macro failed:", e);
    }
  }

  async function handleRemoveMacro(id: string, name: string) {
    if (!window.confirm(`Delete macro "${name}"?`)) return;
    try {
      await invoke("remove_macro", { id });
    } catch (e) {
      console.error("Remove macro failed:", e);
    }
  }

  // UX-09 (D-13): derives the MacroForm-shaped field set (input choice,
  // interval, action-key value) from a macro's persisted sequence.steps, the
  // inverse of MacroForm's own handleSubmit assembly — used to pre-fill the
  // edit form when a card's full edit is opened.
  function deriveEditFormFields(macro: MacroConfig): {
    input: string;
    interval: string;
    actionKeyCode: number | null;
  } {
    const step = macro.sequence.steps[0];
    if (!step) {
      return { input: "Left", interval: String(macro.interval_ms || 100), actionKeyCode: null };
    }
    const ev = "SustainedHold" in step ? step.SustainedHold.input : step.InterleavedInterval.input;
    const intervalMs = "InterleavedInterval" in step ? step.InterleavedInterval.interval_ms : macro.interval_ms;
    if ("Key" in ev) {
      return { input: "KeyPress", interval: String(intervalMs || 100), actionKeyCode: ev.Key };
    }
    return { input: ev.MouseButton, interval: String(intervalMs || 100), actionKeyCode: null };
  }

  // D-13/UX-09: opens the full inline expand-in-place edit for one card.
  // editingCardId is a single shared signal (mutual exclusion) — opening
  // this on card B while card A is mid-edit reassigns editingCardId to B and
  // repopulates the shared edit-form signals from B's own values, which
  // discards A's unsaved local edits for free (no confirmation needed,
  // nothing was persisted).
  function handleStartEditMacro(macro: MacroConfig) {
    const derived = deriveEditFormFields(macro);
    setEditMacroName(macro.name);
    setEditMacroInput(derived.input);
    setEditMacroMode(macro.trigger_mode);
    setEditMacroInterval(derived.interval);
    setEditMacroTarget(macro.target_app ?? "");
    setEditMacroTriggerKeyCode(macro.trigger_key);
    setEditMacroTriggerModifiers(macro.trigger_modifiers);
    setEditMacroActionKeyCode(derived.actionKeyCode);
    setEditFormCapturingSlot(null);
    setEditingCardId(macro.id);
  }

  function handleCancelEditMacro() {
    setEditingCardId(null);
    setEditFormCapturingSlot(null);
    setTriggerKeyRecording(false);
    if (_keyCaptureListener) {
      document.removeEventListener("keydown", _keyCaptureListener, true);
      _keyCaptureListener = null;
    }
  }

  // UX-09: full-field save — generalizes the 10-01 tracer's name-only
  // update_macro call to every field MacroForm assembles (name, action type,
  // key/button assignment, timing), via the single atomic Intent::UpdateMacro.
  async function handleSaveMacro(macro: MacroConfig, values: MacroFormSubmitValues) {
    try {
      await invoke("update_macro", {
        id: macro.id,
        name: values.name,
        sequence: values.sequence,
        trigger_mode: values.triggerMode,
        target_app: values.targetApp,
        trigger_key: values.triggerKey,
        trigger_modifiers: values.triggerModifiers,
      });
      setEditingCardId(null);
      setEditFormCapturingSlot(null);
    } catch (e) {
      // UX-09 (3alt): reuse the Phase 8 C-1 conflict toast; keep the editor
      // open so the user can correct and retry (mirrors handleCreateMacro's
      // catch block).
      const msg = String(e);
      const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
      const macroName = macroMatch ? macroMatch[1] : "another macro";
      const keyLabel = values.triggerKey !== null ? resolveKeyName(values.triggerKey) : "Key";
      showConflictError(keyLabel, macroName);
      console.error("Failed to update macro:", e);
    }
  }

  // @architect: Single-listener invariant via module-level ref (T-03-08)
  // UX-13: `onCommit` receives the platform-native (keycode, modifiers) pair
  // — both must be forwarded to the IPC calls (bind_hotkey, set_macro_trigger_key,
  // or add_macro) so the backend can do the conflict check.
  function startCapture(onCommit: (nativeCode: number, modifiers: number) => void) {
    // Remove any prior stale listener before attaching a new one (Pitfall 2)
    if (_keyCaptureListener) {
      document.removeEventListener("keydown", _keyCaptureListener, true);
      _keyCaptureListener = null;
    }
    setTriggerKeyRecording(true);
    setRecordingModifiers(0);

    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setTriggerKeyRecording(false);
        setRecordingModifiers(0);
        document.removeEventListener("keydown", onKeyDown, true);
        _keyCaptureListener = null;
        return;
      }
      // UX-13: capture the currently-held modifier bits BEFORE the keycode
      // lookup. The C-5 chip in Plan 08-05 reads `recordingModifiers()` to
      // show "⌘" / "Shift" / etc. while the user is still pressing.
      const mods = computeModifiers(e);
      setRecordingModifiers(mods);
      // Use e.code as lookup key to avoid F12/ArrowLeft collision — see keymap.ts
      const nativeCode = domKeycodeToNative(e.code);
      if (nativeCode === null) return;
      onCommit(nativeCode, mods);
      setTriggerKeyRecording(false);
      setRecordingModifiers(0);
      document.removeEventListener("keydown", onKeyDown, true);
      _keyCaptureListener = null;
    }
    _keyCaptureListener = onKeyDown;
    document.addEventListener("keydown", onKeyDown, true);
  }

  // @architect: Guard prevents duplicate in-flight request (T-03-12); re-fetches on every open (D-06)
  async function handlePickerFocus() {
    if (appsLoading()) return;
    setAppsLoading(true);
    setAppsError(false);
    try {
      const result = await invoke<RunningApp[]>("list_running_apps");
      setApps(result);
    } catch (_) {
      setAppsError(true);
    } finally {
      setAppsLoading(false);
    }
  }

  // ── Profile Actions ───────────────────────────────────────────

  function showProfileMsg(text: string, type: "success" | "error") {
    setProfileMessage({ text, type });
    setTimeout(() => setProfileMessage(null), 3000);
  }

  async function refreshProfiles() {
    try {
      const list = await invoke<ProfileSummary[]>("list_profiles");
      setProfiles(list);
    } catch (e) {
      console.error("Failed to refresh profiles:", e);
    }
  }

  async function handleSaveProfile() {
    const name = newProfileName().trim() || activeProfile();
    if (!name) return;
    setProfileLoading(true);
    try {
      await invoke("save_profile", { name });
      setActiveProfile(name);
      setNewProfileName("");
      await refreshProfiles();
      showProfileMsg(`Saved "${name}"`, "success");
    } catch (e) {
      showProfileMsg(`Save failed: ${e}`, "error");
    } finally {
      setProfileLoading(false);
    }
  }

  async function handleLoadProfile(name: string) {
    setProfileLoading(true);
    try {
      await invoke<ProfileData>("load_profile", { name });
      setActiveProfile(name);
      // WR-04: Explicitly re-fetch state as a fallback in case the backend
      // `state-changed` event is dropped (channel backpressure or delivery failure).
      // This guarantees the UI reflects the loaded profile even without the event.
      const freshState = await invoke<AppState>("get_state");
      setState(freshState);
      showProfileMsg(`Loaded "${name}"`, "success");
    } catch (e) {
      showProfileMsg(`Load failed: ${e}`, "error");
    } finally {
      setProfileLoading(false);
    }
  }

  async function handleDeleteProfile(name: string) {
    if (name === "Default") return; // Don't delete the default profile
    setProfileLoading(true);
    try {
      await invoke("delete_profile", { name });
      if (activeProfile() === name) {
        setActiveProfile("Default");
      }
      await refreshProfiles();
      showProfileMsg(`Deleted "${name}"`, "success");
    } catch (e) {
      showProfileMsg(`Delete failed: ${e}`, "error");
    } finally {
      setProfileLoading(false);
    }
  }

  // ── Derived ───────────────────────────────────────────────────

  const macroList = () => {
    const s = state();
    if (!s) return [];
    return Object.values(s.macros);
  };

  const engineActive = () => state()?.engine_active ?? false;

  // ── Render ────────────────────────────────────────────────────

  return (
    <main class="w-full min-h-screen flex flex-col">
      {/* ── Titlebar / Drag Region ── */}
      <div
        data-tauri-drag-region
        class="h-10 flex items-center justify-between px-4 border-b border-border shrink-0"
      >
        <div class="flex items-center gap-2">
          <div class="w-3 h-3 rounded-full bg-accent shadow-[0_0_8px_var(--color-accent-glow)]" />
          <span class="text-sm font-semibold tracking-tight">AutoMux</span>
        </div>
        <span class="text-[10px] text-text-dim font-mono">v{appVersion()}</span>
      </div>

      {/* ── Sidebar + Content row (D-08: sidebar rail replaces top tab bar) ── */}
      <div class="flex flex-1 overflow-hidden">
        <Sidebar activeTab={activeTab} onSelectTab={setActiveTab}>
          <ThemeToggle preference={themePreference} onCycle={cycleThemePreference} />
        </Sidebar>

        {/* ── Content ── */}
        <div class="flex-1 overflow-y-auto p-4 flex flex-col gap-3">
        <Show when={activeTab() === "dashboard"}>
          {/* ═══════════════ DASHBOARD TAB ═══════════════ */}

          {/* ── Status Row ── */}
          <div class="flex gap-3">
            {/* Permissions — combined Accessibility + Input Monitoring */}
            <div class="glass-card flex-1 p-4">
              <div class="flex items-center justify-between mb-3">
                <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
                  Permissions
                </span>
              </div>
              <div class="flex flex-col gap-3">
                {/* Accessibility row */}
                <div>
                  <div class="flex items-center justify-between mb-1">
                    <span class="text-xs font-medium text-text-main">
                      Accessibility
                    </span>
                    <Show
                      when={accessibility() === true}
                      fallback={
                        <Show
                          when={accessibilityPending()}
                          fallback={
                            <div class="flex items-center gap-1.5">
                              <div class="w-2 h-2 rounded-full bg-danger status-pulse shadow-[0_0_6px_var(--color-danger-glow)]" />
                              <span class="text-[11px] text-danger font-medium">
                                Denied
                              </span>
                            </div>
                          }
                        >
                          <div class="flex items-center gap-1.5">
                            <div class="w-2 h-2 rounded-full bg-warning status-pulse shadow-[0_0_6px_var(--color-warning-glow)]" />
                            <span class="text-[11px] text-warning font-medium">
                              Pending…
                            </span>
                          </div>
                        </Show>
                      }
                    >
                      <div class="flex items-center gap-1.5">
                        <div class="w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]" />
                        <span class="text-[11px] text-success font-medium">
                          Granted
                        </span>
                      </div>
                    </Show>
                  </div>
                  <Show
                    when={tccIdentityChanged() === true && accessibility() === false}
                    fallback={
                      <p class="text-xs text-text-dim">Required for input injection</p>
                    }
                  >
                    <p class="text-xs text-text-dim">
                      AutoMux was updated — Accessibility needs to be re-added in System Settings.
                    </p>
                  </Show>
                  <Show when={accessibility() === false && !accessibilityPending()}>
                    <button
                      id="btn-request-access"
                      onClick={handleRequestAccess}
                      class="mt-2 w-full py-1.5 rounded-lg bg-accent/10 border border-accent/30 text-accent text-xs font-medium
                             hover:bg-accent/20 hover:border-accent/50 transition-all duration-200 cursor-pointer"
                    >
                      Grant Access
                    </button>
                  </Show>
                </div>

                {/* Input Monitoring row */}
                <div>
                  <div class="flex items-center justify-between mb-1">
                    <span class="text-xs font-medium text-text-main">
                      Input Monitoring
                    </span>
                    <Show
                      when={inputMonitoring() === true}
                      fallback={
                        <Show
                          when={inputMonitoringPending()}
                          fallback={
                            <div class="flex items-center gap-1.5">
                              <div class="w-2 h-2 rounded-full bg-danger status-pulse shadow-[0_0_6px_var(--color-danger-glow)]" />
                              <span class="text-[11px] text-danger font-medium">
                                Warning
                              </span>
                            </div>
                          }
                        >
                          <div class="flex items-center gap-1.5">
                            <div class="w-2 h-2 rounded-full bg-warning status-pulse shadow-[0_0_6px_var(--color-warning-glow)]" />
                            <span class="text-[11px] text-warning font-medium">
                              Pending…
                            </span>
                          </div>
                        </Show>
                      }
                    >
                      <div class="flex items-center gap-1.5">
                        <div class="w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]" />
                        <span class="text-[11px] text-success font-medium">
                          Granted
                        </span>
                      </div>
                    </Show>
                  </div>
                  <Show
                    when={accessibility() === false && inputMonitoring() === false}
                    fallback={
                      <p class="text-xs text-text-dim">Required for global hotkeys</p>
                    }
                  >
                    <p class="text-xs text-text-dim">
                      Hotkeys will not fire. Mouse macros still work.
                    </p>
                  </Show>
                  <Show when={inputMonitoring() === false && !inputMonitoringPending()}>
                    <button
                      id="btn-request-input-monitoring"
                      onClick={handleRequestInputMonitoringAccess}
                      class="mt-2 w-full py-1.5 rounded-lg bg-accent/10 border border-accent/30 text-accent text-xs font-medium
                             hover:bg-accent/20 hover:border-accent/50 transition-all duration-200 cursor-pointer"
                    >
                      Grant Access
                    </button>
                  </Show>
                </div>
              </div>
            </div>

            {/* Engine Status */}
            <div class="glass-card flex-1 p-4">
              <div class="flex items-center justify-between mb-2">
                <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
                  Engine
                </span>
                <div
                  id="toggle-engine"
                  class="toggle-track"
                  data-active={engineActive()}
                  onClick={handleToggleEngine}
                >
                  <div class="toggle-thumb" />
                </div>
              </div>
              <p class="text-xs text-text-dim">
                {engineActive()
                  ? "Running — macros active"
                  : "Paused — all macros halted"}
              </p>
            </div>
          </div>

          {/* ── Active App ── */}
          <div class="glass-card p-4">
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
                Active Application
              </span>
              <Show when={state()?.emergency_stop_active}>
                <span class="text-[10px] bg-danger/20 text-danger px-2 py-0.5 rounded-full font-medium">
                  ⚠ EMERGENCY STOP
                </span>
              </Show>
            </div>
            <p class="text-sm font-mono mt-2 text-text-main truncate">
              {activeApp() || "—"}
            </p>
          </div>

          {/* ── Active Profile Badge ── */}
          <div class="flex items-center gap-2">
            <span class="text-[10px] bg-accent/10 text-accent px-2 py-0.5 rounded-full font-medium border border-accent/20">
              📁 {activeProfile()}
            </span>
          </div>

          {/* ── First-Run Global Notice (UX-14, UI-SPEC C-3) ── */}
          <Show when={showGlobalNotice()}>
            <div
              id="first-run-global-notice"
              class="bg-accent/10 border border-accent/20 rounded-lg p-3 flex items-center gap-3"
            >
              <span class="text-accent text-base">🌍</span>
              <div class="flex-1">
                <p class="text-xs font-medium text-accent">Binds are system-wide</p>
                <p class="text-[11px] text-text-dim">
                  Hotkeys fire even when AutoMux is in the background. You'll need to allow Input Monitoring in System Settings on macOS, or run as Administrator on Windows for them to work.
                </p>
              </div>
              <button
                onClick={() => {
                  localStorage.setItem("automux.hotkey_global_notice_dismissed", "1");
                  setShowGlobalNotice(false);
                }}
                class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
              >
                Got it
              </button>
            </div>
          </Show>

          {/* ── Conflict Error Toast (UX-11, UI-SPEC C-1) ── */}
          <Show when={conflictError()}>
            {(err) => (
              <div
                id="conflict-error-toast"
                class="rounded-lg px-4 py-2.5 text-xs font-medium flex items-center gap-2 transition-all bg-danger/10 text-danger border border-danger/20"
              >
                <span>✕</span>
                <div class="flex-1">
                  <p class="font-medium">Hotkey already bound</p>
                  <p class="text-text-dim text-[11px] mt-0.5">
                    {err().key} is already assigned to "{err().macroName}". Unbind it first or pick a different key.
                  </p>
                </div>
                <button
                  onClick={() => setConflictError(null)}
                  class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
                >
                  Dismiss
                </button>
              </div>
            )}
          </Show>

          {/* ── Conflict Warning Region (UX-12, UI-SPEC C-2) ── */}
          <Show when={state()?.conflicts && state()!.conflicts.length > 0 && !conflictsDismissed()}>
            <div class="flex flex-col gap-3">
              <For each={state()!.conflicts}>
                {(conflict) => {
                  // Plain functions per CONVENTIONS.md (no createMemo).
                  const macroNames = () =>
                    conflict.macros
                      .map((id) => state()?.macros[id]?.name ?? "Unknown")
                      .filter((n) => n !== "Unknown");
                  const inputLabel = () =>
                    "MouseButton" in conflict.input
                      ? `🖱 ${conflict.input.MouseButton} Click`
                      : `⌨ Key(${conflict.input.Key})`;
                  const rateMultiplier = () => `${macroNames().length}×`;
                  // Oxford-comma join: "A and B" for 2, "A, B, and C" for 3+.
                  const formatConflictList = (names: string[]): string => {
                    if (names.length === 0) return "";
                    if (names.length === 1) return `"${names[0]}"`;
                    if (names.length === 2) return `"${names[0]}" and "${names[1]}"`;
                    const allButLast = names.slice(0, -1).map((n) => `"${n}"`).join(", ");
                    return `${allButLast}, and "${names[names.length - 1]}"`;
                  };
                  const verb = () => (macroNames().length === 1 ? "injects" : "inject");
                  return (
                    <div
                      id="conflict-warning-card"
                      class="bg-warning/10 border border-warning/20 rounded-lg p-3 flex items-center gap-3"
                    >
                      <span class="text-warning text-base">⚠</span>
                      <div class="flex-1">
                        <p class="text-xs font-medium text-warning">
                          {macroNames().length} macros are injecting the same input
                        </p>
                        <p class="text-[11px] text-text-dim">
                          {formatConflictList(macroNames())} {verb()} {inputLabel()} — clicks will fire at {rateMultiplier()} rate.
                        </p>
                      </div>
                      <button
                        onClick={() => setConflictsDismissed(true)}
                        class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
                      >
                        Dismiss
                      </button>
                    </div>
                  );
                }}
              </For>
            </div>
          </Show>

          {/* ── Auto-Save Error Banner ── */}
          <Show when={saveError()}>
            <div
              id="auto-save-error-banner"
              class="bg-warning/10 border border-warning/20 rounded-lg p-3 flex items-center gap-3"
            >
              <span class="text-warning text-base">⚠</span>
              <div class="flex-1">
                <p class="text-xs font-medium text-warning">Save failed</p>
                <p class="text-[11px] text-text-dim">
                  Your changes are not being saved. Check available disk space and file permissions.
                </p>
              </div>
              <button
                onClick={() => setSaveError(false)}
                class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
              >
                Dismiss
              </button>
            </div>
          </Show>

          {/* ── Macros List ── */}
          <div class="flex items-center justify-between mt-1">
            <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
              Macros
            </span>
            <div class="flex items-center gap-2">
              <span class="text-[10px] text-text-dim">
                {macroList().length} configured
              </span>
              <button
                id="btn-open-new-macro"
                onClick={() => setShowNewMacro(true)}
                class="w-6 h-6 rounded-md bg-accent/10 border border-accent/30 text-accent text-sm font-medium
                       hover:bg-accent/20 hover:border-accent/50 transition-all cursor-pointer
                       flex items-center justify-center leading-none"
              >
                +
              </button>
            </div>
          </div>

          {/* ── New Macro Form ── */}
          <Show when={showNewMacro()}>
            <div class="glass-card p-4 border-accent/30">
              <div class="flex items-center justify-between mb-3">
                <span class="text-xs font-medium text-accent uppercase tracking-wider">
                  New Macro
                </span>
                <button
                  onClick={() => setShowNewMacro(false)}
                  class="text-text-dim hover:text-text-main text-sm cursor-pointer transition-colors"
                >
                  ✕
                </button>
              </div>
              <MacroForm
                name={newMacroName}
                onNameChange={setNewMacroName}
                input={newMacroInput}
                onInputChange={setNewMacroInput}
                mode={newMacroTriggerMode}
                onModeChange={setNewMacroTriggerMode}
                interval={newMacroInterval}
                onIntervalChange={setNewMacroInterval}
                target={newMacroTarget}
                onTargetChange={setNewMacroTarget}
                onTargetFocus={handlePickerFocus}
                apps={apps}
                appsLoading={appsLoading}
                appsError={appsError}
                triggerKeyCode={newMacroTriggerKeyCode}
                triggerKeyModifiers={newMacroTriggerModifiers}
                triggerKeyRecording={() => triggerKeyRecording() && formCapturingSlot() === "trigger"}
                onTriggerKeyStartCapture={() => {
                  // Cancel any in-progress card edit before starting form capture
                  if (editingCardId() !== null) {
                    handleCancelEditMacro();
                  }
                  setFormCapturingSlot("trigger");
                  startCapture((nativeCode, mods) => {
                    setNewMacroTriggerKeyCode(nativeCode);
                    setNewMacroTriggerModifiers(mods);
                    setFormCapturingSlot(null);
                  });
                }}
                onTriggerKeyCancelCapture={() => {
                  setTriggerKeyRecording(false);
                  setFormCapturingSlot(null);
                  if (_keyCaptureListener) {
                    document.removeEventListener("keydown", _keyCaptureListener, true);
                    _keyCaptureListener = null;
                  }
                }}
                actionKeyCode={newMacroActionKeyCode}
                actionKeyRecording={() => triggerKeyRecording() && formCapturingSlot() === "action"}
                onActionKeyStartCapture={() => {
                  if (editingCardId() !== null) {
                    handleCancelEditMacro();
                  }
                  setFormCapturingSlot("action");
                  startCapture((nativeCode) => {
                    setNewMacroActionKeyCode(nativeCode);
                    setFormCapturingSlot(null);
                  });
                }}
                onActionKeyCancelCapture={() => {
                  setTriggerKeyRecording(false);
                  setFormCapturingSlot(null);
                  if (_keyCaptureListener) {
                    document.removeEventListener("keydown", _keyCaptureListener, true);
                    _keyCaptureListener = null;
                  }
                }}
                recordingModifierChips={() => modifierChips(recordingModifiers())}
                submitLabel="Create Macro"
                submitDisabled={() => !newMacroName().trim()}
                onSubmit={handleCreateMacro}
              />
            </div>
          </Show>

          <Show
            when={macroList().length > 0}
            fallback={
              <div class="glass-card p-6 flex flex-col items-center justify-center text-center">
                <div class="text-2xl mb-2 opacity-30">⚡</div>
                <p class="text-xs text-text-dim">No macros configured yet.</p>
                <button
                  id="btn-open-new-macro-empty"
                  onClick={() => setShowNewMacro(true)}
                  class="mt-3 px-4 py-2 rounded-lg bg-accent text-white text-xs font-medium
                         hover:bg-accent/80 transition-colors cursor-pointer
                         shadow-[0_0_12px_var(--color-accent-glow)]"
                >
                  + Create Your First Macro
                </button>
              </div>
            }
          >
            <div class="flex flex-col gap-2">
              <For each={macroList()}>
                {(macro) => {
                  // WR-01: compute once per card as a reactive thunk, passed
                  // down as a prop so the dot color and inline
                  // waiting/combined labels cannot disagree.
                  const runningState = () => computeRunningState(macro, state()!);
                  return (
                    <MacroCard
                      macro={macro}
                      runningState={runningState}
                      isEditing={() => editingCardId() === macro.id}
                      onEditStart={() => handleStartEditMacro(macro)}
                      onEditCancel={handleCancelEditMacro}
                      onEditSave={(values) => handleSaveMacro(macro, values)}
                      onToggleEnabled={() => handleToggleMacro(macro.id, macro.enabled)}
                      onDeleteClick={() => handleRemoveMacro(macro.id, macro.name)}
                      editName={editMacroName}
                      onEditNameChange={setEditMacroName}
                      editInput={editMacroInput}
                      onEditInputChange={setEditMacroInput}
                      editMode={editMacroMode}
                      onEditModeChange={setEditMacroMode}
                      editInterval={editMacroInterval}
                      onEditIntervalChange={setEditMacroInterval}
                      editTarget={editMacroTarget}
                      onEditTargetChange={setEditMacroTarget}
                      onEditTargetFocus={handlePickerFocus}
                      apps={apps}
                      appsLoading={appsLoading}
                      appsError={appsError}
                      editTriggerKeyCode={editMacroTriggerKeyCode}
                      editTriggerKeyModifiers={editMacroTriggerModifiers}
                      editTriggerKeyRecording={() => triggerKeyRecording() && editFormCapturingSlot() === "trigger"}
                      onEditTriggerKeyStartCapture={() => {
                        setEditFormCapturingSlot("trigger");
                        startCapture((nativeCode, mods) => {
                          setEditMacroTriggerKeyCode(nativeCode);
                          setEditMacroTriggerModifiers(mods);
                          setEditFormCapturingSlot(null);
                        });
                      }}
                      onEditTriggerKeyCancelCapture={() => {
                        setTriggerKeyRecording(false);
                        setEditFormCapturingSlot(null);
                        if (_keyCaptureListener) {
                          document.removeEventListener("keydown", _keyCaptureListener, true);
                          _keyCaptureListener = null;
                        }
                      }}
                      editActionKeyCode={editMacroActionKeyCode}
                      editActionKeyRecording={() => triggerKeyRecording() && editFormCapturingSlot() === "action"}
                      onEditActionKeyStartCapture={() => {
                        setEditFormCapturingSlot("action");
                        startCapture((nativeCode) => {
                          setEditMacroActionKeyCode(nativeCode);
                          setEditFormCapturingSlot(null);
                        });
                      }}
                      onEditActionKeyCancelCapture={() => {
                        setTriggerKeyRecording(false);
                        setEditFormCapturingSlot(null);
                        if (_keyCaptureListener) {
                          document.removeEventListener("keydown", _keyCaptureListener, true);
                          _keyCaptureListener = null;
                        }
                      }}
                      recordingModifierChips={() => modifierChips(recordingModifiers())}
                      editSubmitDisabled={() => !editMacroName().trim()}
                    />
                  );
                }}
              </For>
            </div>
          </Show>
        </Show>

        <Show when={activeTab() === "profiles"}>
          {/* ═══════════════ PROFILES TAB ═══════════════ */}

          {/* ── Message Toast ── */}
          <Show when={profileMessage()}>
            {(msg) => (
              <div
                class={`rounded-lg px-4 py-2.5 text-xs font-medium flex items-center gap-2 transition-all ${
                  msg().type === "success"
                    ? "bg-success/10 text-success border border-success/20"
                    : "bg-danger/10 text-danger border border-danger/20"
                }`}
              >
                <span>{msg().type === "success" ? "✓" : "✕"}</span>
                <span>{msg().text}</span>
              </div>
            )}
          </Show>

          {/* ── Save Current ── */}
          <div class="glass-card p-4">
            <span class="text-xs font-medium text-text-muted uppercase tracking-wider block mb-3">
              Save Current State
            </span>
            <div class="flex gap-2">
              <input
                id="input-profile-name"
                type="text"
                placeholder={`Profile name (default: "${activeProfile()}")`}
                value={newProfileName()}
                onInput={(e) => setNewProfileName(e.currentTarget.value)}
                class="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm
                       focus:outline-none focus:border-accent/50 transition-colors placeholder:text-text-dim"
              />
              <button
                id="btn-save-profile"
                onClick={handleSaveProfile}
                disabled={profileLoading()}
                class="px-4 py-2 rounded-lg bg-accent text-white text-xs font-medium
                       hover:bg-accent/80 transition-colors disabled:opacity-50 cursor-pointer
                       shadow-[0_0_12px_var(--color-accent-glow)]"
              >
                {profileLoading() ? "…" : "Save"}
              </button>
            </div>
          </div>

          {/* ── Saved Profiles List ── */}
          <div class="flex items-center justify-between mt-1">
            <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
              Saved Profiles
            </span>
            <span class="text-[10px] text-text-dim">
              {profiles().length} profiles
            </span>
          </div>

          <Show
            when={profiles().length > 0}
            fallback={
              <div class="glass-card p-6 flex flex-col items-center justify-center text-center">
                <div class="text-2xl mb-2 opacity-30">📁</div>
                <p class="text-xs text-text-dim">No saved profiles yet.</p>
                <p class="text-[10px] text-text-dim mt-1">
                  Save your current configuration above.
                </p>
              </div>
            }
          >
            <div class="flex flex-col gap-2">
              <For each={profiles()}>
                {(profile) => (
                  <div
                    class={`glass-card p-4 ${
                      activeProfile() === profile.name
                        ? "border-accent/40"
                        : ""
                    }`}
                  >
                    <div class="flex items-center justify-between">
                      <div class="flex items-center gap-2">
                        <Show
                          when={activeProfile() === profile.name}
                          fallback={
                            <div class="w-2 h-2 rounded-full bg-text-dim" />
                          }
                        >
                          <div class="w-2 h-2 rounded-full bg-accent shadow-[0_0_6px_var(--color-accent-glow)]" />
                        </Show>
                        <span class="text-sm font-medium">{profile.name}</span>
                        <span class="text-[10px] text-text-dim">
                          {profile.macro_count} macros
                        </span>
                      </div>
                      <div class="flex items-center gap-1.5">
                        <button
                          onClick={() => handleLoadProfile(profile.name)}
                          disabled={
                            profileLoading() ||
                            activeProfile() === profile.name
                          }
                          class="px-2.5 py-1 rounded-md text-[10px] font-medium transition-colors cursor-pointer
                                 bg-accent/10 text-accent border border-accent/20
                                 hover:bg-accent/20 disabled:opacity-30 disabled:cursor-default"
                        >
                          Load
                        </button>
                        <Show when={profile.name !== "Default"}>
                          <button
                            onClick={() => handleDeleteProfile(profile.name)}
                            disabled={profileLoading()}
                            class="px-2.5 py-1 rounded-md text-[10px] font-medium transition-colors cursor-pointer
                                   bg-danger/10 text-danger border border-danger/20
                                   hover:bg-danger/20 disabled:opacity-30"
                          >
                            ✕
                          </button>
                        </Show>
                      </div>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </Show>
        </div>
      </div>

      {/* ── Footer ── */}
      <div class="h-8 flex items-center justify-center border-t border-border shrink-0">
        <span class="text-[10px] text-text-dim">
          ⌘⇧Q Emergency Stop &nbsp;·&nbsp; {macroList().length} macros
          &nbsp;·&nbsp; 📁 {activeProfile()}
        </span>
      </div>
    </main>
  );
}

export default App;
