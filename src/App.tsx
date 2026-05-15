import { createSignal, createEffect, onCleanup, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

// ── Types (mirrors Rust state) ──────────────────────────────────

interface MacroConfig {
  id: string;
  name: string;
  interval_ms: number;
  enabled: boolean;
  target_app: string | null;
  sequence: { steps: ActionStep[] };
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

// ── App ─────────────────────────────────────────────────────────

type Tab = "dashboard" | "profiles";

function App() {
  const [state, setState] = createSignal<AppState | null>(null);
  const [accessibility, setAccessibility] = createSignal<boolean | null>(null);
  const [activeApp, setActiveApp] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);
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

  // ── Initial data fetch ──
  createEffect(async () => {
    try {
      const [stateData, accessOk, app, profileList] = await Promise.all([
        invoke<AppState>("get_state"),
        invoke<boolean>("check_accessibility"),
        invoke<string | null>("get_active_app"),
        invoke<ProfileSummary[]>("list_profiles"),
      ]);
      setState(stateData);
      setAccessibility(accessOk);
      setActiveApp(app);
      setProfiles(profileList);
    } catch (e) {
      console.error("Failed to fetch initial state:", e);
    } finally {
      setLoading(false);
    }
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

  // Poll accessibility every 3s
  createEffect(() => {
    const interval = setInterval(async () => {
      try {
        const ok = await invoke<boolean>("check_accessibility");
        setAccessibility(ok);
      } catch (_) {
        /* ignore */
      }
    }, 3000);
    onCleanup(() => clearInterval(interval));
  });

  // ── Actions ───────────────────────────────────────────────────

  async function handleRequestAccess() {
    try {
      const granted = await invoke<boolean>("request_accessibility");
      setAccessibility(granted);
    } catch (e) {
      console.error("Accessibility request failed:", e);
    }
  }

  async function handleToggleEngine() {
    try {
      await invoke("toggle_engine");
    } catch (e) {
      console.error("Toggle engine failed:", e);
    }
  }

  async function handleToggleMacro(id: string, currentEnabled: boolean) {
    try {
      await invoke("set_macro_enabled", { id, enabled: !currentEnabled });
    } catch (e) {
      console.error("Toggle macro failed:", e);
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
        <span class="text-[10px] text-text-dim font-mono">v0.1.0</span>
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
            {/* Accessibility Status */}
            <div class="glass-card flex-1 p-4">
              <div class="flex items-center justify-between mb-2">
                <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
                  Security
                </span>
                <Show
                  when={accessibility() === true}
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
                    <div class="w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]" />
                    <span class="text-[11px] text-success font-medium">
                      Granted
                    </span>
                  </div>
                </Show>
              </div>
              <p class="text-xs text-text-dim">Accessibility Permissions</p>
              <Show when={accessibility() === false}>
                <button
                  id="btn-request-access"
                  onClick={handleRequestAccess}
                  class="mt-3 w-full py-1.5 rounded-lg bg-accent/10 border border-accent/30 text-accent text-xs font-medium
                         hover:bg-accent/20 hover:border-accent/50 transition-all duration-200 cursor-pointer"
                >
                  Grant Access
                </button>
              </Show>
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

          {/* ── Macros List ── */}
          <div class="flex items-center justify-between mt-1">
            <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
              Macros
            </span>
            <span class="text-[10px] text-text-dim">
              {macroList().length} configured
            </span>
          </div>

          <Show
            when={macroList().length > 0}
            fallback={
              <div class="glass-card p-6 flex flex-col items-center justify-center text-center">
                <div class="text-2xl mb-2 opacity-30">⚡</div>
                <p class="text-xs text-text-dim">No macros configured yet.</p>
                <p class="text-[10px] text-text-dim mt-1">
                  Use the API to create your first macro.
                </p>
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

                    {/* Target */}
                    <div class="flex items-center gap-2 text-[11px] text-text-dim mb-1">
                      <span>🎯</span>
                      <span class="font-mono">
                        {macro.target_app || "Global"}
                      </span>
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
