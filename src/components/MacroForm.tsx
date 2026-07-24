import type { Component } from "solid-js";
import { Show, For } from "solid-js";
import KeyCaptureField from "./KeyCaptureField";
import { resolveKeyName } from "../keymap";
import type { TriggerMode, InputEvent, ActionSequence, RunningApp } from "../App";

// D-13 (10-04 Task 2): shared field set used by BOTH the "New Macro" create
// panel (this plan) and, in plan 10-05, the inline card-edit form. Builds
// the same ActionSequence shape as the pre-existing handleCreateMacro
// (App.tsx) so create and edit can never drift into producing a different
// persisted shape for the same user input.
export interface MacroFormSubmitValues {
  name: string;
  sequence: ActionSequence;
  triggerMode: TriggerMode;
  targetApp: string | null;
  triggerKey: number | null;
  triggerModifiers: number;
}

// Signals-down / accessor-props pattern (RESEARCH.md Pattern 1) — every
// signal this form reads/writes is owned by the caller (App.tsx) and passed
// down as an accessor + setter/callback pair; MacroForm never owns
// createSignal state itself. Read via props.x()/props.onX(...) inline,
// never destructured.
interface MacroFormProps {
  name: () => string;
  onNameChange: (v: string) => void;
  // Input choice: "Left" | "Right" | "Middle" | "KeyPress" (plain string,
  // matching the existing select-bound signal convention in App.tsx).
  input: () => string;
  onInputChange: (v: string) => void;
  mode: () => TriggerMode;
  onModeChange: (v: TriggerMode) => void;
  interval: () => string;
  onIntervalChange: (v: string) => void;
  target: () => string;
  onTargetChange: (v: string) => void;
  onTargetFocus: () => void;
  apps: () => RunningApp[];
  appsLoading: () => boolean;
  appsError: () => boolean;

  // Hotkey trigger-key capture slot (unchanged behavior from the
  // pre-existing New Macro form, now routed through KeyCaptureField).
  triggerKeyCode: () => number | null;
  triggerKeyModifiers: () => number;
  triggerKeyRecording: () => boolean;
  onTriggerKeyStartCapture: () => void;
  onTriggerKeyCancelCapture: () => void;

  // D-11/D-12: Key Press action-input capture slot — net new this phase,
  // a capture instance distinct from the trigger-key slot above so the two
  // can coexist on the same form without one clobbering the other's state.
  actionKeyCode: () => number | null;
  actionKeyRecording: () => boolean;
  onActionKeyStartCapture: () => void;
  onActionKeyCancelCapture: () => void;

  // Shared modifier-chip labels (both capture slots reuse the same
  // App-owned `modifierChips(recordingModifiers())` computation).
  recordingModifierChips: () => string[];

  submitLabel: string;
  submitDisabled: () => boolean;
  onSubmit: (values: MacroFormSubmitValues) => void;
}

const MacroForm: Component<MacroFormProps> = (props) => {
  // Mirrors handleCreateMacro's pre-existing ActionSequence-building logic
  // (App.tsx) verbatim in shape — Input=KeyPress now maps to a real
  // captured keycode instead of the old unreachable `parseInt(inputVal)`
  // fallback, since Key Press was never actually selectable before D-11.
  function handleSubmit() {
    const name = props.name().trim();
    if (!name) return;
    const inputVal = props.input();
    const intervalMs = parseInt(props.interval()) || 100;
    const input: InputEvent =
      inputVal === "KeyPress"
        ? { Key: props.actionKeyCode() ?? 0 }
        : { MouseButton: inputVal as "Left" | "Right" | "Middle" };
    const sequence: ActionSequence =
      props.mode() === "Hold"
        ? { steps: [{ SustainedHold: { input } }] }
        : { steps: [{ InterleavedInterval: { input, interval_ms: intervalMs } }] };
    props.onSubmit({
      name,
      sequence,
      triggerMode: props.mode(),
      targetApp: props.target().trim() || null,
      triggerKey: props.triggerKeyCode(),
      triggerModifiers: props.triggerKeyModifiers(),
    });
  }

  return (
    <div class="flex flex-col gap-2">
      <input
        id="input-macro-name"
        type="text"
        placeholder="Macro name (e.g. AFK Farm)"
        value={props.name()}
        onInput={(e) => props.onNameChange(e.currentTarget.value)}
        class="bg-background border border-border rounded-lg px-3 py-2 text-sm
               focus:outline-none focus:border-accent/50 transition-colors placeholder:text-text-dim"
      />
      <div class="flex gap-2">
        {/* UX-10/D-11: exactly 4 labeled options, fixed order, incl. the
            net-new Key Press choice — InputEvent::Key already existed in
            the type system but was never selectable until this phase. */}
        <select
          id="select-macro-input"
          value={props.input()}
          onChange={(e) => props.onInputChange(e.currentTarget.value)}
          class="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm
                 focus:outline-none focus:border-accent/50 transition-colors text-text-main"
        >
          <option value="Left">🖱 Left Click</option>
          <option value="Right">🖱 Right Click</option>
          <option value="Middle">🖱 Middle Click</option>
          <option value="KeyPress">⌨ Key Press</option>
        </select>
        {/* A held input never repeats — interval-ms is hidden (not just
            disabled) when Mode = Hold (Sustained), matching the rule
            already implicit for mouse buttons today. */}
        <Show when={props.mode() !== "Hold"}>
          <input
            id="input-macro-interval"
            type="number"
            placeholder="ms"
            value={props.interval()}
            onInput={(e) => props.onIntervalChange(e.currentTarget.value)}
            class="w-24 bg-background border border-border rounded-lg px-3 py-2 text-sm
                   focus:outline-none focus:border-accent/50 transition-colors placeholder:text-text-dim"
          />
        </Show>
      </div>

      {/* D-11/D-12: selecting Key Press reveals the reused Phase 8
          key-capture widget for the action's own key, distinct from the
          trigger-key capture slot further below. */}
      <Show when={props.input() === "KeyPress"}>
        <KeyCaptureField
          label={() =>
            props.actionKeyCode() !== null ? resolveKeyName(props.actionKeyCode()!) : "Click to set key…"
          }
          hasValue={() => props.actionKeyCode() !== null}
          recording={props.actionKeyRecording}
          recordingModifierChips={props.recordingModifierChips}
          onStartCapture={() => props.onActionKeyStartCapture()}
          onCancelRecording={() => props.onActionKeyCancelCapture()}
        />
      </Show>

      <select
        id="select-macro-target"
        value={props.target()}
        onChange={(e) => props.onTargetChange(e.currentTarget.value)}
        onFocus={() => props.onTargetFocus()}
        class="w-full bg-background border border-border rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
      >
        <option value="">🌐 Global (no target)</option>
        <Show when={props.appsLoading()}>
          <option disabled>Loading…</option>
        </Show>
        <Show when={props.appsError()}>
          <option disabled>Failed to load apps</option>
        </Show>
        <For each={props.apps()}>
          {(app) => (
            <option value={app.identifier}>
              {app.display_name} ({app.identifier})
            </option>
          )}
        </For>
      </select>

      {/* UX-10: exactly 2 labeled options, fixed order — relabeled from
          the old jargon-y "Interval"/"latched" phrasing. */}
      <div class="flex gap-2">
        <select
          id="select-macro-trigger-mode"
          value={props.mode()}
          onChange={(e) => props.onModeChange(e.currentTarget.value as TriggerMode)}
          class="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm
                 focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
        >
          <option value="Pulse">⏱ Pulse (Repeat)</option>
          <option value="Hold">🔒 Hold (Sustained)</option>
        </select>
        <KeyCaptureField
          label={() =>
            props.triggerKeyCode() !== null ? resolveKeyName(props.triggerKeyCode()!) : "Click to set key…"
          }
          hasValue={() => props.triggerKeyCode() !== null}
          recording={props.triggerKeyRecording}
          recordingModifierChips={props.recordingModifierChips}
          onStartCapture={() => props.onTriggerKeyStartCapture()}
          onCancelRecording={() => props.onTriggerKeyCancelCapture()}
        />
      </div>

      <button
        id="btn-create-macro"
        onClick={handleSubmit}
        disabled={props.submitDisabled()}
        class="w-full py-2 rounded-lg bg-accent text-white text-xs font-medium
               hover:bg-accent/80 transition-colors disabled:opacity-30 cursor-pointer
               shadow-[0_0_12px_var(--color-accent-glow)]"
      >
        {props.submitLabel}
      </button>
    </div>
  );
};

export default MacroForm;
