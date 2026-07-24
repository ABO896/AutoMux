import type { Component } from "solid-js";
import { Show, For } from "solid-js";
import MacroForm, { type MacroFormSubmitValues } from "./MacroForm";
import { resolveKeyName } from "../keymap";
import { formatStep, type MacroConfig, type RunningState, type TriggerMode, type RunningApp } from "../App";

// D-13 (UX-09, primary surface): one macro card, normal + inline expand-in-
// place edit display states (delete-confirm state added in plan 10-05 Task
// 2). Signals-down / accessor-props pattern (RESEARCH.md Pattern 1) — every
// signal driving this card (editingCardId, the edit-form field values) is
// owned by App() and passed down as accessor/callback props; MacroCard has
// no local reactive-state primitives of its own for edit state, which would
// silently break the existing single-card mutual-exclusion invariant.
interface MacroCardProps {
  macro: MacroConfig;
  runningState: () => RunningState;
  isEditing: () => boolean;
  onEditStart: () => void;
  onEditCancel: () => void;
  onEditSave: (values: MacroFormSubmitValues) => void;
  onToggleEnabled: () => void;
  onDeleteClick: () => void;

  // Edit-form field props — mirror MacroForm's own props verbatim; App()
  // owns the underlying signals (pre-filled from the macro's current values
  // when edit is opened) and this card is just a pass-through mount point,
  // same shape as the "New Macro" panel's own MacroForm instance.
  editName: () => string;
  onEditNameChange: (v: string) => void;
  editInput: () => string;
  onEditInputChange: (v: string) => void;
  editMode: () => TriggerMode;
  onEditModeChange: (v: TriggerMode) => void;
  editInterval: () => string;
  onEditIntervalChange: (v: string) => void;
  editTarget: () => string;
  onEditTargetChange: (v: string) => void;
  onEditTargetFocus: () => void;
  apps: () => RunningApp[];
  appsLoading: () => boolean;
  appsError: () => boolean;
  editTriggerKeyCode: () => number | null;
  editTriggerKeyModifiers: () => number;
  editTriggerKeyRecording: () => boolean;
  onEditTriggerKeyStartCapture: () => void;
  onEditTriggerKeyCancelCapture: () => void;
  editActionKeyCode: () => number | null;
  editActionKeyRecording: () => boolean;
  onEditActionKeyStartCapture: () => void;
  onEditActionKeyCancelCapture: () => void;
  recordingModifierChips: () => string[];
  editSubmitDisabled: () => boolean;
}

// Plain function per CONVENTIONS.md (no createMemo) — mirrors the dot-color
// switch previously inline in App.tsx's <For> body.
function runningStateDotClass(runningState: RunningState): string {
  switch (runningState) {
    case "firing":
      return "w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)] animate-pulse";
    case "held":
      return "w-2 h-2 rounded-full bg-accent shadow-[0_0_6px_var(--color-accent-glow)]";
    case "combined":
      return "w-2 h-2 rounded-full bg-warning shadow-[0_0_6px_var(--color-warning-glow)] animate-pulse";
    case "waiting":
      return "w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]";
    default:
      return "w-2 h-2 rounded-full bg-text-dim";
  }
}

const MacroCard: Component<MacroCardProps> = (props) => {
  return (
    <div class="glass-card p-4">
      <Show
        when={!props.isEditing()}
        fallback={
          <div class="flex flex-col gap-2">
            <MacroForm
              name={props.editName}
              onNameChange={props.onEditNameChange}
              input={props.editInput}
              onInputChange={props.onEditInputChange}
              mode={props.editMode}
              onModeChange={props.onEditModeChange}
              interval={props.editInterval}
              onIntervalChange={props.onEditIntervalChange}
              target={props.editTarget}
              onTargetChange={props.onEditTargetChange}
              onTargetFocus={props.onEditTargetFocus}
              apps={props.apps}
              appsLoading={props.appsLoading}
              appsError={props.appsError}
              triggerKeyCode={props.editTriggerKeyCode}
              triggerKeyModifiers={props.editTriggerKeyModifiers}
              triggerKeyRecording={props.editTriggerKeyRecording}
              onTriggerKeyStartCapture={props.onEditTriggerKeyStartCapture}
              onTriggerKeyCancelCapture={props.onEditTriggerKeyCancelCapture}
              actionKeyCode={props.editActionKeyCode}
              actionKeyRecording={props.editActionKeyRecording}
              onActionKeyStartCapture={props.onEditActionKeyStartCapture}
              onActionKeyCancelCapture={props.onEditActionKeyCancelCapture}
              recordingModifierChips={props.recordingModifierChips}
              submitLabel="Save Changes"
              submitDisabled={props.editSubmitDisabled}
              onSubmit={props.onEditSave}
            />
            <button
              onClick={() => props.onEditCancel()}
              class="w-full py-2 rounded-lg text-[11px] font-semibold transition-colors cursor-pointer
                     bg-surface-alt border border-border text-text-muted hover:text-text-main"
            >
              Cancel
            </button>
          </div>
        }
      >
        <>
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2 flex-1 min-w-0">
              <div class={runningStateDotClass(props.runningState())} />
              <span
                class="text-[15px] font-semibold text-text-main truncate"
                title={props.macro.name}
              >
                {props.macro.name}
              </span>
              <Show when={props.runningState() === "waiting"}>
                <span class="text-[11px] font-semibold text-text-dim shrink-0">
                  Waiting for {props.macro.target_app}
                </span>
              </Show>
              <Show when={props.runningState() === "combined"}>
                <span class="text-[11px] font-semibold text-text-dim shrink-0">
                  Active (Hold + Click)
                </span>
              </Show>
            </div>
            <div class="flex items-center gap-1.5 shrink-0">
              <div
                class="toggle-track"
                data-active={props.macro.enabled}
                onClick={() => props.onToggleEnabled()}
                style={{ transform: "scale(0.8)" }}
              >
                <div class="toggle-thumb" />
              </div>
              <button
                onClick={() => props.onEditStart()}
                class="px-2.5 py-1 rounded-md text-[11px] font-semibold transition-colors cursor-pointer
                       bg-surface-alt border border-border text-text-muted hover:text-text-main hover:border-border-hover"
                title="Edit macro"
              >
                ✎
              </button>
              <button
                onClick={() => props.onDeleteClick()}
                class="px-2.5 py-1 rounded-md text-[11px] font-semibold transition-colors cursor-pointer
                       bg-danger/10 text-danger border border-danger/20 hover:bg-danger/20"
                title="Delete macro"
              >
                ✕
              </button>
            </div>
          </div>

          {/* Target & Trigger */}
          <div class="flex items-center justify-between text-[11px] font-semibold text-text-dim mb-1">
            <div class="flex items-center gap-2 min-w-0">
              <span>🎯</span>
              <span
                class="font-mono truncate"
                title={props.macro.target_app || "Global"}
              >
                {props.macro.target_app || "Global"}
              </span>
            </div>
            <div class="flex items-center gap-1 shrink-0">
              <span class="font-mono">
                {props.macro.trigger_key !== null ? resolveKeyName(props.macro.trigger_key) : "Set key…"}
              </span>
              <span class="text-text-muted">({props.macro.trigger_mode})</span>
              <span>↗ Global</span>
            </div>
          </div>

          {/* Steps */}
          <Show when={props.macro.sequence.steps.length > 0}>
            <div class="flex flex-wrap gap-1.5 mt-2">
              <For each={props.macro.sequence.steps}>
                {(step) => (
                  <span class="text-[11px] font-semibold bg-surface-alt border border-border rounded px-2 py-0.5 text-text-muted">
                    {formatStep(step)}
                  </span>
                )}
              </For>
            </div>
          </Show>
        </>
      </Show>
    </div>
  );
};

export default MacroCard;
