---
status: diagnosed
trigger: "G-09-1b-no-delete-macro-ui — The user cannot delete a macro from within the AutoMux app UI, even though the Rust backend already exposes a remove_macro IPC command."
created: 2026-07-22T00:00:00Z
updated: 2026-07-22T00:20:00Z
---

## Current Focus

hypothesis: (CONFIRMED) The `remove_macro` IPC command is fully implemented and wired on the Rust backend, but no UI element in `src/App.tsx` ever invokes it — the macro card markup (lines 762-901) has no delete/trash button, no context menu, and no confirm-delete dialog. This is a pure frontend gap: the capability exists but is unreachable from the UI.
test: Grepped `src/App.tsx` for `remove_macro`, `RemoveMacro`, delete/trash icon strings, `confirm(`, `contextmenu`, `dblclick` — zero matches anywhere in the macro list/card section. Read full macro card JSX (lines 762-901) line by line.
expecting: N/A — root cause confirmed, goal is find_root_cause_only.
next_action: Return ROOT CAUSE FOUND to caller (plan-phase --gaps) with fix direction: add a delete button to the macro card that calls `invoke("remove_macro", { id: macro.id })` and refreshes state, following the existing `handleDeleteProfile` pattern (App.tsx:411-423) for confirm + error-handling conventions.

## Symptoms
<!-- IMMUTABLE -->

expected: The user should be able to delete/remove an individual macro from the app's UI (e.g. a delete/trash button on each macro card), which calls the existing `remove_macro` IPC command and removes it from `AppState`'s macro registry (and persisted profile).
actual: User reports: "there is still not a way to delete macros" — no such control exists anywhere in the UI during Phase 9 UAT testing.
errors: None — this is an absence-of-feature report, not a crash.
reproduction: Open the app, look at a macro card / list, and confirm there is no delete/remove affordance wired to an existing macro.
started: Discovered during UAT for Phase 9 (Parallel Macro Execution). Backend `remove_macro` command exists; this is a frontend gap.

## Eliminated

(none — first hypothesis confirmed on first pass)

## Evidence

- timestamp: 2026-07-22T00:05:00Z
  checked: `src-tauri/src/ipc/mod.rs:22-28`
  found: |
    ```rust
    #[command]
    pub async fn remove_macro(state: State<'_, StateManager>, id: Uuid) -> Result<(), String> {
        state
            .send_intent(Intent::RemoveMacro(id))
            .await
            .map_err(|e| e.to_string())
    }
    ```
  implication: Backend `#[command]` fn exists and correctly sends `Intent::RemoveMacro(id)` to the StateActor.

- timestamp: 2026-07-22T00:06:00Z
  checked: `src-tauri/src/lib.rs:83-85` (invoke_handler registration)
  found: "`ipc::remove_macro` is listed in `tauri::generate_handler![...]` immediately after `ipc::add_macro` — confirmed registered and callable from the frontend via `invoke(\"remove_macro\", ...)`."
  implication: Command is exposed to the IPC bridge; nothing missing on the registration side.

- timestamp: 2026-07-22T00:08:00Z
  checked: `src-tauri/src/state/mod.rs:301-308` (`Intent::RemoveMacro` handling in `StateActor::handle_intent`)
  found: |
    ```rust
    Intent::RemoveMacro(id) => {
        self.state.macros.remove(&id);
        let _ = self
            .scheduler_tx
            .send(crate::scheduler::SchedulerIntent::StopMacro(id))
            .await;
        self.auto_save_default().await;
    }
    ```
  implication: Removal correctly deletes the macro from `AppState.macros`, stops any active scheduler timers for it, and persists the change via `auto_save_default()`. Backend end-to-end wiring (ipc -> Intent -> StateActor) is complete and correct. Backend is NOT the problem.

- timestamp: 2026-07-22T00:10:00Z
  checked: "`grep -n \"remove_macro\\|RemoveMacro\\|delete\\|trash\\|Delete\\|Trash\" src/App.tsx`"
  found: "Zero matches for `remove_macro` or `RemoveMacro`. Only matches are `handleDeleteProfile` (line 411), its `invoke(\"delete_profile\", ...)` call (line 415), and its wiring to a button at line 1015 — all of which delete a saved *profile* (a named JSON file of macros), not an individual macro from the live list."
  implication: No frontend code path invokes `remove_macro` anywhere in the single-component app. The only existing \"delete\" feature is unrelated (profile deletion).

- timestamp: 2026-07-22T00:14:00Z
  checked: "`src/App.tsx` lines 762-901 — full JSX for the macro list `<For each={macroList()}>` card render"
  found: |
    Each macro card renders, top to bottom: (1) status dot + macro name (774), (2) enable/disable toggle calling `handleToggleMacro` (779-781), (3) target-app picker (789-834), (4) trigger-key display/capture UI (835-884), (5) step chips (888-898). There is no delete/trash icon, no overflow/kebab menu, no right-click context menu, and no confirm-delete modal anywhere in this block or elsewhere in the file.
  implication: Confirms the UI gap is total — not a broken/dead-code button, not a mis-wired handler, but a complete absence of any delete affordance for individual macros.

- timestamp: 2026-07-22T00:16:00Z
  checked: "`grep -n \"confirm(\\|contextmenu\\|oncontextmenu\\|longpress\\|dblclick\\|remove_macro\" src/App.tsx`"
  found: "No output — zero matches."
  implication: Rules out alternate/hidden delete triggers (context menu, double-click, confirm dialog) that might not appear in the main card markup. The gap is unambiguous."

## Resolution

root_cause: "`remove_macro` is fully implemented and correctly wired end-to-end on the Rust backend (`src-tauri/src/ipc/mod.rs:22-28` -> `Intent::RemoveMacro` -> `src-tauri/src/state/mod.rs:301-308`, registered in `src-tauri/src/lib.rs:85`), but `src/App.tsx` never calls it. The macro card component (App.tsx:762-901) has no delete/trash button, context menu, or confirm-delete dialog — this is a missing frontend feature, not a broken or dead-coded one. The existing `handleDeleteProfile`/`delete_profile` flow (App.tsx:411-423, 1015) deletes an entire saved profile, not an individual macro, and is unrelated."
fix: ""
verification: ""
files_changed: []
