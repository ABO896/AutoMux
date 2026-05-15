# Plan

Refactor the input triggering mechanism to support universal keyboard keys, per-macro toggles, and two distinct trigger modes (Pulse and Hold), while ensuring O(1) input lookup and robust emergency stop behavior.

## Scope

- In: Adding `trigger_key` and `trigger_mode` to `MacroConfig`. Building a `HashMap<u16, Uuid>` for O(1) lookup in the input hook. Ensuring `InputTrackingRegistry` or scheduler correctly tracks and releases "Hold" states during Emergency Stop.
- Out: Pixel scanning, complex visual triggers, or sequenced hotkey chords.

## Action Items

[ ] Update `MacroConfig` model in `src-tauri/src/state/mod.rs` to include `trigger_key: Option<u16>` and `trigger_mode: TriggerMode`.
[ ] Define the `TriggerMode` enum (`Pulse`, `Hold`) in `src-tauri/src/state/mod.rs`.
[ ] Maintain a `HashMap<u16, Uuid>` mapping trigger keys to macro IDs for $O(1)$ lookup in the platform's input hook observer.
[ ] Update the global input hooks (macOS and Windows) to intercept universal keyboard keys and dispatch `ToggleMacroHotkey(Uuid)` or `SetMacroEnabled` based on the trigger key.
[ ] Implement `TriggerMode::Hold` behavior in `Scheduler` or `StateActor` to issue a continuous "Down" state on activation and an "Up" state on deactivation.
[ ] Refactor the Emergency Stop mechanism (and `InputTrackingRegistry`/`active_holds`) to guarantee that all keys in a "Hold" state are fully released.
[ ] Validate memory usage remains under the <60MB limit and that Emergency Stop successfully releases held keys in all test scenarios.

## Open Questions

- For `TriggerMode::Hold`, should the macro remain active *only while* the `trigger_key` is physically held down, or does the user press the `trigger_key` once to toggle the held state on, and press it again to toggle it off?
