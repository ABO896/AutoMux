import type { Component } from "solid-js";
import { Show, For } from "solid-js";

// D-12 (10-04 Task 1): reusable key-capture widget wrapping the existing
// App-owned `startCapture()` flow + modifier-chip/capture-chip rendering.
// Reused for BOTH the hotkey trigger-key slot (existing) and the new Key
// Press action-input slot (MacroForm, Task 2). The single-listener invariant
// (T-03-08) stays App/module-owned — this component never declares its own
// document-level keydown listener ref; it only reflects `recording` state
// and delegates start/cancel to callback props.
//
// Signals-down / accessor-props pattern (RESEARCH.md Pattern 1) — every
// prop is read inline (props.recording(), props.label()), never destructured.
interface KeyCaptureFieldProps {
  // Text to show when NOT recording — either the resolved committed key
  // name, or a placeholder like "Click to set key…" / "Set key…". The
  // component itself supplies the "recording" text ("Press a key…").
  label: () => string;
  // Whether `label()` currently represents a committed value (vs a
  // placeholder) — used for styling only.
  hasValue: () => boolean;
  // Whether THIS instance is the one currently capturing. Callers compute
  // this from their own identity-scoped signals (e.g. editingCardId match,
  // or a form-local "which slot is capturing" signal) since the underlying
  // `triggerKeyRecording` signal is shared/global.
  recording: () => boolean;
  // Modifier chip labels to render while recording (already computed by the
  // caller via the existing `modifierChips(bits)` helper) — kept as a plain
  // prop rather than importing that helper here to avoid a value-level
  // circular import between App.tsx and this component.
  recordingModifierChips: () => string[];
  // Invoked on click anywhere on the field (matches the original behavior:
  // both existing call sites invoke `startCapture` unconditionally on
  // click, regardless of current recording state).
  onStartCapture: () => void;
  // Invoked when the inline ✕ cancel control (visible only while recording)
  // is clicked. Callers compose the correct site-specific cleanup (reset
  // editingCardId/editingField for the card site, or just the recording
  // flag for the form site) since that varies by call site.
  onCancelRecording: () => void;
  // Compact chip styling for tight contexts (e.g. the macro card's
  // single-line meta row) vs the larger select-sized box styling used in
  // MacroForm's field rows. Static per call site, not reactive — read
  // directly (not destructured) like every other prop.
  compact?: boolean;
}

const KeyCaptureField: Component<KeyCaptureFieldProps> = (props) => {
  return (
    <Show
      when={props.compact}
      fallback={
        <div class="flex flex-col">
          <Show when={props.recording() && props.recordingModifierChips().length > 0}>
            <div class="flex items-center gap-1 mb-1">
              <For each={props.recordingModifierChips()}>
                {(label) => (
                  <span class="px-2 py-1 rounded bg-surface-alt border border-border text-[11px] font-semibold text-text-main">
                    {label}
                  </span>
                )}
              </For>
            </div>
          </Show>
          <div
            class={`rounded-lg px-3 py-2 text-sm w-40 cursor-pointer flex items-center justify-between
              ${props.recording()
                ? "bg-background border border-accent text-accent shadow-[0_0_8px_var(--color-accent-glow)]"
                : props.hasValue()
                  ? "bg-background border border-border text-text-main"
                  : "bg-background border border-border text-text-dim"
              }`}
            role="button"
            tabIndex={0}
            onClick={() => props.onStartCapture()}
          >
            <span>{props.recording() ? "Press a key…" : props.label()}</span>
            <Show when={props.recording()}>
              <span
                class="text-text-dim hover:text-text-main ml-2 leading-none"
                onClick={(e) => {
                  e.stopPropagation();
                  props.onCancelRecording();
                }}
              >
                ✕
              </span>
            </Show>
          </div>
        </div>
      }
    >
      <div class="flex flex-col">
        <Show when={props.recording() && props.recordingModifierChips().length > 0}>
          <div class="flex items-center gap-1 mb-1">
            <For each={props.recordingModifierChips()}>
              {(label) => (
                <span class="px-2 py-1 rounded bg-surface-alt border border-border text-[11px] font-semibold text-text-main">
                  {label}
                </span>
              )}
            </For>
          </div>
        </Show>
        <Show
          when={props.recording()}
          fallback={
            <span
              class={
                props.hasValue()
                  ? "px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[11px] font-semibold cursor-pointer hover:border-accent/40"
                  : "px-1.5 py-0.5 rounded border border-dashed border-border text-[11px] font-semibold text-text-dim cursor-pointer hover:border-accent/40 hover:text-text-main"
              }
              onClick={() => props.onStartCapture()}
            >
              {props.label()}
            </span>
          }
        >
          <span class="px-1.5 py-0.5 rounded border border-accent text-[11px] font-semibold text-accent shadow-[0_0_4px_var(--color-accent-glow)] flex items-center gap-1">
            Press…
            <span
              class="text-text-dim hover:text-text-main leading-none cursor-pointer"
              onClick={() => props.onCancelRecording()}
            >
              ✕
            </span>
          </span>
        </Show>
      </div>
    </Show>
  );
};

export default KeyCaptureField;
