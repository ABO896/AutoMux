import { createSignal, createEffect, onCleanup, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import { domKeycodeToNative, resolveKeyName } from "./keymap";
import "./App.css";

// ── Types (mirrors Rust state) ──────────────────────────────────

type TriggerMode = "Pulse" | "Hold";

interface MacroConfig {
  id: string;
  name: string;
  interval_ms: number;
  enabled: boolean;
  target_app: string | null;
  sequence: { steps: ActionStep[] };
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

type InputEvent =
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

interface RunningApp {
  display_name: string;
  identifier: string;
}

// ── Helpers ─────────────────────────────────────────────────────

function formatInputEvent(ev: InputEvent): string {
  if ("MouseButton" in ev) return `🖱 ${ev.MouseButton}`;
  if ("Key" in ev) return `⌨ Key(${ev.Key})`;
  return "?";
}

function formatStep(step: ActionStep): string {
  if ("SustainedHold" in step)
    return `Hold ${formatInputEvent(step.SustainedHold.input)}`;
  if ("InterleavedInterval" in step)
    return `${formatInputEvent(step.InterleavedInterval.input)} every ${step.InterleavedInterval.interval_ms}ms`;
  return "?";
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

// ── App ─────────────────────────────────────────────────────────

type Tab = "dashboard" | "profiles";

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
  // ConflictErrorToast (Plan 08-05) renders this — the wiring (signal +
  // helper) ships in 08-04 so the toast can be added without touching
  // the call sites again. 8-second auto-dismiss per UI-SPEC C-1.
  const [conflictError, setConflictError] = createSignal<{
    key: string;
    macroName: string;
  } | null>(null);
  // Plan 08-05 C-1 (ConflictErrorToast) will render this signal. The `void`
  // reference is a no-op runtime read that satisfies the strict
  // noUnusedLocals setting until the toast is added.
  void conflictError;
  let _conflictErrorTimer: ReturnType<typeof setTimeout> | null = null;
  function showConflictError(key: string, macroName: string) {
    if (_conflictErrorTimer !== null) clearTimeout(_conflictErrorTimer);
    setConflictError({ key, macroName });
    _conflictErrorTimer = setTimeout(() => {
      setConflictError(null);
      _conflictErrorTimer = null;
    }, 8000);
  }

  const [appVersion, setAppVersion] = createSignal<string>("…");

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
  // Plan 08-05 reads this signal to render the modifier preview row.
  const [recordingModifiers, setRecordingModifiers] = createSignal<number>(0);
  // Plan 08-05 C-5 (ModifierPreviewChip) will render this signal. The `void`
  // reference is a no-op runtime read that satisfies the strict
  // noUnusedLocals setting until the chip is added.
  void recordingModifiers;

  // ── Process Picker State ──
  const [apps, setApps] = createSignal<RunningApp[]>([]);
  const [appsLoading, setAppsLoading] = createSignal(false);
  const [appsError, setAppsError] = createSignal(false);

  // ── Card Inline Edit State ──
  const [editingCardId, setEditingCardId] = createSignal<string | null>(null);
  const [editingField, setEditingField] = createSignal<"key" | "target" | null>(null);

  // ── Platform Detection ──
  // WR-02: navigator.platform is deprecated and returns empty string in some Chromium
  // WebView configurations (including Tauri on macOS). Use userAgent as the primary
  // signal — it is always populated in Tauri's Chromium-based WebView and reliably
  // contains "Mac" on macOS builds.
  const IS_MACOS = navigator.userAgent.toLowerCase().includes("mac");

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

  async function handleCreateMacro() {
    const name = newMacroName().trim();
    if (!name) return;
    const interval = parseInt(newMacroInterval()) || 100;
    const inputVal = newMacroInput();
    const target = newMacroTarget().trim() || null;

    const input: InputEvent = inputVal === "Left" || inputVal === "Right" || inputVal === "Middle"
      ? { MouseButton: inputVal as "Left" | "Right" | "Middle" }
      : { Key: parseInt(inputVal) || 0 };

    const triggerKey = newMacroTriggerKeyCode();

    // WR-05: id is generated by the backend; supply a placeholder that gets overwritten.
    // UX-13: `trigger_modifiers` carries the platform-native bitmask captured
    // by computeModifiers during key capture. See PATTERNS.md for the bit map.
    const config: MacroConfig = {
      id: "00000000-0000-0000-0000-000000000000",
      name,
      interval_ms: interval,
      enabled: false,
      target_app: target,
      sequence: {
        steps: [{ InterleavedInterval: { input, interval_ms: interval } }],
      },
      trigger_key: triggerKey,
      trigger_modifiers: newMacroTriggerModifiers(),
      trigger_mode: newMacroTriggerMode(),
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

  // @architect: macOS uses unbind+rebind path per Pitfall 3 (T-03-07); Windows uses set_macro_trigger_key
  // UX-13: `modifiers` is the platform-native bitmask from computeModifiers,
  // forwarded to both bind_hotkey (macOS) and set_macro_trigger_key (both).
  async function handleCardSetTriggerKey(id: string, nativeCode: number, modifiers: number) {
    try {
      if (IS_MACOS) {
        await invoke("unbind_hotkey", { macro_id: id });
        await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
        await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
      } else {
        await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
      }
      setEditingCardId(null);
      setEditingField(null);
    } catch (e) {
      // UX-11: surface the conflict error to the C-1 toast (Plan 08-05).
      // Same parsing as handleCreateMacro — extract the conflicting macro
      // name from the backend's well-known error format. Falls back to a
      // generic message if the format doesn't match.
      const msg = String(e);
      const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
      const macroName = macroMatch ? macroMatch[1] : "another macro";
      const keyLabel = resolveKeyName(nativeCode);
      showConflictError(keyLabel, macroName);
      console.error("Card trigger key update failed:", e);
    }
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

  // @architect: Empty string converts to null for Global targeting (T-03-13)
  async function handleCardSetTargetApp(id: string, targetApp: string | null) {
    try {
      await invoke("set_macro_target_app", { id, target_app: targetApp || null });
      setEditingCardId(null);
      setEditingField(null);
    } catch (e) {
      console.error("Card target update failed:", e);
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

      {/* ── Tab Bar ── */}
      <div class="flex border-b border-border shrink-0">
        <button
          id="tab-dashboard"
          class={`flex-1 py-2 text-xs font-medium text-center transition-colors cursor-pointer ${
            activeTab() === "dashboard"
              ? "text-accent border-b-2 border-accent"
              : "text-text-muted hover:text-text-main"
          }`}
          onClick={() => setActiveTab("dashboard")}
        >
          Dashboard
        </button>
        <button
          id="tab-profiles"
          class={`flex-1 py-2 text-xs font-medium text-center transition-colors cursor-pointer ${
            activeTab() === "profiles"
              ? "text-accent border-b-2 border-accent"
              : "text-text-muted hover:text-text-main"
          }`}
          onClick={() => setActiveTab("profiles")}
        >
          Profiles
        </button>
      </div>

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
              <div class="flex flex-col gap-2">
                <input
                  id="input-macro-name"
                  type="text"
                  placeholder="Macro name (e.g. AFK Farm)"
                  value={newMacroName()}
                  onInput={(e) => setNewMacroName(e.currentTarget.value)}
                  class="bg-background border border-border rounded-lg px-3 py-2 text-sm
                         focus:outline-none focus:border-accent/50 transition-colors placeholder:text-text-dim"
                />
                <div class="flex gap-2">
                  <select
                    id="select-macro-input"
                    value={newMacroInput()}
                    onChange={(e) => setNewMacroInput(e.currentTarget.value)}
                    class="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm
                           focus:outline-none focus:border-accent/50 transition-colors text-text-main"
                  >
                    <option value="Left">🖱 Left Click</option>
                    <option value="Right">🖱 Right Click</option>
                    <option value="Middle">🖱 Middle Click</option>
                  </select>
                  <input
                    id="input-macro-interval"
                    type="number"
                    placeholder="ms"
                    value={newMacroInterval()}
                    onInput={(e) => setNewMacroInterval(e.currentTarget.value)}
                    class="w-24 bg-background border border-border rounded-lg px-3 py-2 text-sm
                           focus:outline-none focus:border-accent/50 transition-colors placeholder:text-text-dim"
                  />
                </div>
                <select
                  id="select-macro-target"
                  value={newMacroTarget()}
                  onChange={(e) => setNewMacroTarget(e.currentTarget.value)}
                  onFocus={handlePickerFocus}
                  class="w-full bg-background border border-border rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
                >
                  <option value="">🌐 Global (no target)</option>
                  <Show when={appsLoading()}>
                    <option disabled>Loading…</option>
                  </Show>
                  <Show when={appsError()}>
                    <option disabled>Failed to load apps</option>
                  </Show>
                  <For each={apps()}>
                    {(app) => (
                      <option value={app.identifier}>
                        {app.display_name} ({app.identifier})
                      </option>
                    )}
                  </For>
                </select>
                
                {/* ── Trigger Mode Selector ── */}
                <div class="flex gap-2">
                  <select
                    id="select-macro-trigger-mode"
                    value={newMacroTriggerMode()}
                    onChange={(e) => setNewMacroTriggerMode(e.currentTarget.value as TriggerMode)}
                    class="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm
                           focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
                  >
                    <option value="Pulse">⏱ Pulse (Interval)</option>
                    <option value="Hold">🔒 Hold (Latched)</option>
                  </select>
                  <div
                    class={`rounded-lg px-3 py-2 text-sm w-40 cursor-pointer flex items-center justify-between
                      ${triggerKeyRecording()
                        ? "bg-background border border-accent text-accent shadow-[0_0_8px_var(--color-accent-glow)]"
                        : newMacroTriggerKeyCode() !== null
                          ? "bg-background border border-border text-text-main"
                          : "bg-background border border-border text-text-dim"
                      }`}
                    role="button"
                    tabIndex={0}
                    onClick={() => {
                      // Cancel any in-progress card edit before starting form capture
                      if (editingCardId() !== null) {
                        setEditingCardId(null);
                        setEditingField(null);
                      }
                      startCapture((nativeCode, mods) => {
                        setNewMacroTriggerKeyCode(nativeCode);
                        setNewMacroTriggerModifiers(mods);
                      });
                    }}
                  >
                    <span>
                      {triggerKeyRecording()
                        ? "Press a key…"
                        : newMacroTriggerKeyCode() !== null
                          ? resolveKeyName(newMacroTriggerKeyCode()!)
                          : "Click to set key…"
                      }
                    </span>
                    <Show when={triggerKeyRecording()}>
                      <span
                        class="text-text-dim hover:text-text-main ml-2 leading-none"
                        onClick={(e) => {
                          e.stopPropagation();
                          setTriggerKeyRecording(false);
                          if (_keyCaptureListener) {
                            document.removeEventListener("keydown", _keyCaptureListener, true);
                            _keyCaptureListener = null;
                          }
                        }}
                      >✕</span>
                    </Show>
                  </div>
                </div>

                <button
                  id="btn-create-macro"
                  onClick={handleCreateMacro}
                  disabled={!newMacroName().trim()}
                  class="w-full py-2 rounded-lg bg-accent text-white text-xs font-medium
                         hover:bg-accent/80 transition-colors disabled:opacity-30 cursor-pointer
                         shadow-[0_0_12px_var(--color-accent-glow)]"
                >
                  Create Macro
                </button>
              </div>
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
                {(macro) => (
                  <div class="glass-card p-4">
                    <div class="flex items-center justify-between mb-2">
                      <div class="flex items-center gap-2">
                        <div
                          class={`w-2 h-2 rounded-full ${
                            macro.enabled
                              ? "bg-success shadow-[0_0_6px_var(--color-success-glow)]"
                              : "bg-text-dim"
                          }`}
                        />
                        <span class="text-sm font-medium">{macro.name}</span>
                      </div>
                      <div
                        class="toggle-track"
                        data-active={macro.enabled}
                        onClick={() =>
                          handleToggleMacro(macro.id, macro.enabled)
                        }
                        style={{ transform: "scale(0.8)" }}
                      >
                        <div class="toggle-thumb" />
                      </div>
                    </div>

                    {/* Target & Trigger */}
                    <div class="flex items-center justify-between text-[11px] text-text-dim mb-1">
                      <div class="flex items-center gap-2">
                        <span>🎯</span>
                        <Show
                          when={editingCardId() === macro.id && editingField() === "target"}
                          fallback={
                            <span
                              class="font-mono cursor-pointer border border-transparent hover:border-accent/40 rounded px-1"
                              onClick={() => {
                                setEditingCardId(macro.id);
                                setEditingField("target");
                                handlePickerFocus();
                              }}
                            >
                              {macro.target_app || "Global"}
                            </span>
                          }
                        >
                          <select
                            class="flex-1 min-w-0 bg-background border border-border rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
                            onFocus={handlePickerFocus}
                            onChange={(e) => handleCardSetTargetApp(macro.id, e.currentTarget.value || null)}
                            onKeyDown={(e) => {
                              if (e.key === "Escape") {
                                setEditingCardId(null);
                                setEditingField(null);
                              }
                            }}
                          >
                            <option value="">🌐 Global (no target)</option>
                            <Show when={appsLoading()}>
                              <option disabled>Loading…</option>
                            </Show>
                            <Show when={appsError()}>
                              <option disabled>Failed to load apps</option>
                            </Show>
                            <For each={apps()}>
                              {(app) => (
                                <option value={app.identifier}>
                                  {app.display_name} ({app.identifier})
                                </option>
                              )}
                            </For>
                          </select>
                        </Show>
                      </div>
                      <Show when={macro.trigger_key !== null} fallback={
                        <div class="flex items-center gap-1">
                          <span
                            class="px-1.5 py-0.5 rounded border border-dashed border-border text-[10px] font-mono text-text-dim cursor-pointer hover:border-accent/40 hover:text-text-main"
                            onClick={() => {
                              setEditingCardId(macro.id);
                              setEditingField("key");
                              startCapture((nativeCode, mods) => handleCardSetTriggerKey(macro.id, nativeCode, mods));
                            }}
                          >Set key…</span>
                          <span class="text-[10px] text-text-muted">({macro.trigger_mode})</span>
                        </div>
                      }>
                        <div class="flex items-center gap-1">
                          <Show
                            when={editingCardId() === macro.id && editingField() === "key"}
                            fallback={
                              <span
                                class="px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono cursor-pointer hover:border-accent/40"
                                onClick={() => {
                                  setEditingCardId(macro.id);
                                  setEditingField("key");
                                  startCapture((nativeCode, mods) => handleCardSetTriggerKey(macro.id, nativeCode, mods));
                                }}
                              >
                                {resolveKeyName(macro.trigger_key!)}
                              </span>
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
                          <span class="text-[10px] text-text-muted">
                            ({macro.trigger_mode})
                          </span>
                        </div>
                      </Show>
                    </div>

                    {/* Steps */}
                    <Show when={macro.sequence.steps.length > 0}>
                      <div class="flex flex-wrap gap-1.5 mt-2">
                        <For each={macro.sequence.steps}>
                          {(step) => (
                            <span class="text-[10px] bg-surface-alt border border-border rounded px-2 py-0.5 text-text-muted">
                              {formatStep(step)}
                            </span>
                          )}
                        </For>
                      </div>
                    </Show>
                  </div>
                )}
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
