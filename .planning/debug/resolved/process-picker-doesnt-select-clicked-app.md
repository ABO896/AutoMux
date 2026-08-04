---
status: resolved
trigger: "Target-app process picker doesn't select the process the user clicked"
created: 2026-08-04
updated: 2026-08-04
---

## Bug Classification (Phase 1.75)

bug_class: Bohrbug in practice — mechanism is a timing race (async fetch vs.
native dropdown open), but the timing skew is so lopsided (µs-to-low-ms IPC
call vs. hundreds-of-ms human click latency) that it resolves the same way
every time, matching the reported "100%, not intermittent" symptom.
Routed as: common-bug-patterns match on Async/Timing "Race condition" +
State Management "Stale render... wrong reference".

## Symptoms

- **Expected behavior:** Clicking a process/app in the per-card target-app `<select>` (src/App.tsx:1443-1465) assigns that exact app as the macro's `target_app`.
- **Actual behavior:** Wrong app gets selected — clicking one option in the list results in a different app being assigned than the one clicked.
- **Error messages:** None reported.
- **Timeline:** First reported during the Phase 10 Plan 03 (10-03) human checkpoint manual walkthrough on 2026-07-24. Not caused by Phase 10 changes — no 10-01/10-02/10-03 plan touched this code block. `apps()` / `list_running_apps` wiring predates this phase (Phase 9 Plan 8 already fixed a related "uncontrolled dropdown" bug by adding `value={macro.target_app ?? ""}`), so this may be a second, distinct defect in the same picker.
- **Reproduction:** Every time — 100% reproducible, not intermittent. Open a macro card's target-app picker and click any process in the list; the app assigned does not match the one clicked.

## Prior investigation notes (from backlog todo, not yet diagnosed)

Candidate causes to check:
- Whether `apps()` re-renders/reorders between the dropdown opening and the click landing, causing a stale index to resolve to the wrong `app.identifier`.
- Whether `handleCardSetTargetApp`'s `set_macro_target_app` IPC call round-trips correctly.
- Whether the native `<select>`'s `onChange` value doesn't match the visually clicked option under SolidJS's reactivity model (e.g. `<For>` keying/reordering desync).

## Evidence

- timestamp: 2026-08-04T00:00:00Z
  checked: src/App.tsx and src/components/{MacroCard,MacroForm}.tsx — the picker moved out of App.tsx during Phase 10's UI redesign (debug file's line refs 1443-1465 are stale). Real picker `<select id="select-macro-target">` now lives in src/components/MacroForm.tsx:156-177, used by both the "New Macro" panel and each card's inline edit form (via MacroCard.tsx props).
  found: The `<select>`'s `onFocus` prop calls `props.onTargetFocus()`, wired in App.tsx (both call sites, line 1223 and 1326) to a single shared handler `handlePickerFocus` (App.tsx:737-749).
  implication: Every open of either target-app picker (create panel or any card's edit form) routes through the same focus handler.

- timestamp: 2026-08-04T00:00:01Z
  checked: App.tsx:736-749 `handlePickerFocus` body
  found: |
    async function handlePickerFocus() {
      if (appsLoading()) return;
      setAppsLoading(true);
      setAppsError(false);
      try {
        const result = await invoke<RunningApp[]>("list_running_apps");
        setApps(result);
      } ...
    }
    `apps()` (App.tsx:314) is a signal ONLY ever written here — no mount-time/eager fetch exists elsewhere (verified via grep for `setApps(` — single call site). So apps() starts as `[]` and is replaced with a brand-new array on every single focus of either select.
  implication: Opening the picker ALWAYS triggers an async IPC round-trip that mutates the `apps()` signal driving the `<For>` that renders the `<option>` list — every time, not conditionally.

- timestamp: 2026-08-04T00:00:02Z
  checked: MacroForm.tsx:170-176 `<For each={props.apps()}>{(app) => <option value={app.identifier}>...</option>}</For>`
  found: SolidJS's `<For>` diffs list items by reference (`===`), not by a stable key like `app.identifier`. `invoke<RunningApp[]>(...)` deserializes a fresh JSON payload into brand-new JS objects on every call, so no object in a later `apps()` array is ever `===` to an object in the previous array — even when the underlying set of running apps is byte-for-byte identical between two fetches.
  implication: `<For>` treats every `setApps(result)` call as "all previous items removed, all new items added," destroying and recreating every `<option>` DOM node on EVERY focus event, regardless of whether the actual app list changed.

- timestamp: 2026-08-04T00:00:03Z
  checked: src-tauri/src/platform/macos/observer.rs:178-198 `list_running_apps_impl` — measured directly with a temporary `#[test]` timing probe (`cargo test`, 5 runs), removed after measurement (confirmed via `git diff --stat` showing no residual change)
  found: "[TEMP PROBE] run 0: 9.885375ms ... run 1-4: ~25-35µs" (first run pays NSWorkspace warmup cost, steady-state is tens of microseconds). Full round trip including Tauri IPC serialization/JS promise resolution is still expected to land well under 50ms in practice.
  implication: The async re-fetch triggered by `onFocus` resolves and calls `setApps()` far faster (single-digit-to-low-double-digit ms) than a human's typical dropdown-open-to-click reaction time (hundreds of ms to ~1-2s). The DOM-destroying mutation from evidence #3 is therefore virtually guaranteed to land WHILE the native `<select>` dropdown is still open and visible to the user on every single open — explaining the reported "100%, not intermittent" reproducibility rather than an occasional flake.

- timestamp: 2026-08-04T00:00:04Z
  checked: App.tsx:532-578 `handleCreateMacro` and App.tsx:669-680 `handleSaveMacro` (the two `onSubmit`/`onEditSave` consumers of MacroFormSubmitValues.targetApp) plus MacroForm.tsx:74-95 `handleSubmit` which builds `targetApp: props.target().trim() || null` directly from the `target()` accessor (itself set verbatim by the select's `onChange={(e) => props.onTargetChange(e.currentTarget.value)}`)
  found: No transformation, lookup, or indexing happens anywhere in this chain — the value delivered to the backend IPC call (`add_macro` / `update_macro`) is exactly whatever `e.currentTarget.value` reported for the `onChange` event.
  implication: Rules out an application-level wiring/logic bug (candidate causes #2 from the prior backlog notes) as a contributing or alternative cause. If the wrong app ends up assigned, the browser/webview itself is reporting the wrong `.value` on the `change` event — consistent with the native-select-mutated-while-open mechanism, not a code-level indexing/lookup error.

- timestamp: 2026-08-04T00:00:05Z
  checked: Web search — "HTML select dropdown wrong option selected when options mutated while open native select webkit"
  found: Corroborating context (not an exact match) that WebKit has a documented history of option-selection/insertion-order bugs specifically around dynamic mutation of `<option>` elements' selectedness while the element is live (WebKit changeset 288174, iOS Safari `<select>` dropdown-state bugs). No spec guarantees behavior when a `<select>`'s options are mutated while its native popup is displayed.
  implication: Corroborates that "mutate `<option>` list while the native dropdown may be showing" is a known-fragile category of browser/webview behavior, not a far-fetched theory.

## Eliminated

- hypothesis: "handleCardSetTargetApp's set_macro_target_app IPC call doesn't round-trip correctly" (prior backlog candidate #2)
  evidence: Traced the full data path from `<select onChange>` through `MacroFormSubmitValues.targetApp` to both `add_macro` and `update_macro` invoke() calls — every step is a direct passthrough of `e.currentTarget.value` with no transformation, indexing, or lookup. No opportunity for a code-level bug to substitute one app for another exists in this path. (2026-08-04)

- hypothesis: "`setApps(reconcile(result, { key: 'identifier' }))` correctly preserves reactivity while suppressing unnecessary `<For>` DOM churn" (the fix applied in the first fix_and_verify pass)
  evidence: REGRESSION reported by human-verify checkpoint — picker gets stuck on "Loading…" then shows only "Global (no target)" forever, even on the very first open. Root-caused via `node_modules/solid-js` source read + an empirical Node script using a real PUSH-based `createEffect` subscriber (not the prior script's pull-based direct `mapped()` call): `reconcile()`'s returned function (`solid-js/store/dist/store.js:383-396`) calls `applyState`, which mutates the PREVIOUS array in place via `setProperty` (`state[property] = value`) and returns that SAME object reference (`store.js:394`, `res === undefined ? state : res` where `state` is literally `s.value`, the signal's current stored value). `createSignal`'s setter (`solid-js/dist/solid.js:206-211`) calls `value(s.value)` when given a function, then passes the result to `writeSignal`. `writeSignal` (`solid.js:643-653`) uses `node.comparator` — default `equalFn = (a,b) => a === b` (`solid.js:150,155-157`) — to decide whether to notify observers. Since reconcile always returns the exact same reference as the current value (mutated in place, never reallocated), `current === value` is always true, so `writeSignal` NEVER notifies observers — not just on steady-state opens, but on the very FIRST `setApps()` call too (mutating `[]` in place and returning that same `[]` reference). Verified directly: `node /private/tmp/.../scratchpad/verify-reconcile-signal-bug.mjs` — Test 1 shows a `createEffect` subscribed to `apps()` runs exactly once (at mount, seeing length 0) and NEVER re-runs across two `setApps(reconcile(...))` calls, even though the raw underlying signal value's `.length` silently becomes 3 (Test 2 confirms `ref0 === ref1 === ref2`, i.e. the identical object reference throughout). Test 3 reproduces why the PRIOR verification script (`verify-picker-fix.mjs`) falsely passed: it called `mapArray`'s returned function directly and synchronously (a PULL, re-reading `list()` fresh on every manual call) rather than through a push-based reactive computation — masking that observers are never notified. `<For>` in the real app is driven by exactly this kind of push-based computation (JSX `<For>` wraps its list accessor in a tracked computation), so this is not a test-harness-only artifact — it reproduces the real regression. CONCLUSION: `reconcile()` is designed for use with `createStore` setters, where property-level writes go through the store's Proxy machinery (`setProperty` → `getNode`/`node.$()` → per-property fine-grained signals) that DO notify correctly on mutation, independent of top-level reference identity. Applied to a plain `createSignal` (as `apps` is, per this project's explicit "createSignal for all local reactive state, never createStore" convention), reconcile's in-place-mutate-and-return-same-reference behavior defeats the signal's own top-level reference-equality dedup, permanently freezing the signal's reactive consumers at their initial state. This is a different, and worse, root cause than the original bug: the fix as applied was based on a correct diagnosis of the ORIGINAL defect (see Resolution.root_cause below, still valid) but an incorrect/incompatible implementation choice (`reconcile` + plain signal) for the fix itself. (2026-08-04)

## Resolution

root_cause: |
  The target-app `<select>`'s `onFocus` handler (`handlePickerFocus`, App.tsx:737)
  unconditionally re-fetches the running-app list and calls `setApps()` with a
  brand-new array of freshly-deserialized objects on every single open of the
  picker (create panel or card edit form). Because SolidJS's `<For>`
  (MacroForm.tsx:170) diffs list items by reference equality rather than by a
  stable key (`app.identifier`), this guarantees every `<option>` DOM node in
  the picker is destroyed and rebuilt on every focus — even when the actual
  set of running apps hasn't changed. Since the native IPC call resolves in
  single-digit-to-low-double-digit milliseconds (measured), far faster than a
  human's click-after-dropdown-opens reaction time, this DOM-destroying
  mutation reliably happens WHILE the OS-native `<select>` popup is still
  open and visible. Mutating a focused/open `<select>`'s options is not a
  browser-spec-guaranteed-safe operation; the click that lands afterward
  resolves against the just-rebuilt (not the visually-displayed) option
  list, producing a value that doesn't match what the user visually clicked.
  Single root cause (code category) — no AND-gate; see reasoning_checkpoint
  in Current Focus for the branching check performed.
fix: |
  SUPERSEDES the reconcile()-based fix (reverted — see Eliminated section:
  it broke reactivity entirely and caused the reported regression). In
  App.tsx's `handlePickerFocus`, replaced `setApps(reconcile(result, {
  key: "identifier" }))` with a hand-rolled reference-preserving merge that
  is compatible with a PLAIN `createSignal` (this project's convention;
  `reconcile` is a `createStore`-oriented primitive): build a `Map` of the
  current `apps()` array keyed by `identifier`, map the freshly-fetched
  `result` array to reuse each existing app's OLD object reference when its
  `identifier` and `display_name` are unchanged (otherwise use the new
  object), and always call `setApps()` with this freshly-allocated
  top-level array. Because the top-level array is always a new reference,
  the signal's default reference-equality check always sees a change and
  always notifies observers — restoring reactivity (fixes the regression:
  stuck-on-Loading / only-Global-shown). Because unchanged apps deliberately
  keep their old object reference, `<For>`'s own per-item reference-equality
  diffing (`mapArray`) still recognizes them as unchanged and skips
  rebuilding their `<option>` DOM nodes — preserving the original bug's fix
  intent (no destroy-and-rebuild of the option list while the native
  dropdown may still be open). No `solid-js/store` import remains; `apps`
  stays a plain signal, consistent with project conventions.
verification:
  target_test: { result: skipped, reason: "no frontend test framework exists in this repo (confirmed via TECH-STACK docs: 'No frontend test framework detected')" }
  mutation_check: { result: skipped, reason: "no Stryker configured for the TypeScript/SolidJS frontend" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: "diff replaces the incompatible reconcile()+plain-signal call with an equivalent-intent hand-rolled merge; no branch, assertion, or logic was removed or short-circuited without replacement. Verified via `git diff -- src/App.tsx` (net diff against HEAD is clean — the reconcile import/call was never committed, so this shows original buggy code -> corrected fix directly)." }
  adjacent_tests: { result: skipped, reason: "no test suite touches src/App.tsx's import graph" }
  typecheck: { result: pass, note: "`npx tsc --noEmit` — zero errors after removing the reconcile import and replacing its call site." }
  build: { result: pass, note: "`npm run build` (vite build) succeeds; only a pre-existing, unrelated Tailwind arbitrary-value CSS warning (shadow-[...] token) present before this change too." }
  revert_and_reconfirm:
    result: pass
    bug_returned_on_revert: true
    fixed_on_reapply: true
    note: |
      Same native-<select>-popup-click automation limitation as before
      applies (cannot physically click a live macOS native dropdown in this
      environment). This round's mechanism-level verification specifically
      targets and closes the gap that let the FIRST fix's verification
      falsely pass: that script (verify-picker-fix.mjs) called `mapArray`'s
      returned function directly (a PULL — re-reads the list fresh on every
      manual call regardless of whether the signal notified observers),
      which is NOT how `<For>` is actually driven in the DOM (a PUSH-based
      tracked computation). This round used a real `createEffect` subscriber
      wrapping `mapArray`/reading `apps()` — matching the actual push-based
      mechanism — via two scripts run against the project's real solid-js
      build:
        - verify-reconcile-signal-bug.mjs: reproduced the REGRESSION exactly
          — a push-based effect subscribed to `apps()` runs once (at mount,
          length 0) and NEVER re-runs across two
          `setApps(reconcile(result,{key:"identifier"}))` calls, even though
          the raw signal's `.length` silently becomes 3 underneath (proves
          `ref0 === ref1 === ref2`, i.e. reconcile always returns the
          identical object reference, permanently defeating the plain
          signal's reference-equality-based notify).
        - verify-corrected-fix.mjs (Tests A/B/C, all PASS) using the SAME
          push-based createEffect+mapArray harness against the new
          hand-rolled-merge logic:
          - Test A: effect re-runs on every `setApps(merge)` call (3 runs
            for 3 writes: mount + 2 opens), ending at the correct length —
            confirms reactivity is restored (regression fixed).
          - Test B: 3 opens with byte-identical app-list content produce
            only 3 total `<option>`-equivalent CREATE calls (i.e. only the
            first open builds anything; opens 2-3 touch zero nodes) —
            confirms the original bug's fix intent (no DOM churn while
            native dropdown may be open) is preserved.
          - Test C: a real content change (one app quits, a different app
            launches) between two opens is still correctly reflected
            (removed app gone, new app present, unrelated apps' identities
            preserved) — confirms the fix doesn't "freeze" the list.
      Scripts (not committed — throwaway verification, no frontend test
      framework exists in this repo to house a permanent regression test):
      /private/tmp/claude-501/-Users-alvaro-AutoClicker/7a748221-a159-4786-b82c-255907d9cbfd/scratchpad/verify-reconcile-signal-bug.mjs
      /private/tmp/claude-501/-Users-alvaro-AutoClicker/7a748221-a159-4786-b82c-255907d9cbfd/scratchpad/verify-corrected-fix.mjs
      This confirms the CAUSAL MECHANISM (both the regression's cause and
      the fix's correctness) with direct empirical evidence against the
      actual push-based reactivity path that drives `<For>`, closing the
      specific gap that made the first fix's self-verification misleading.
      It does not, by itself, confirm the full end-to-end native-click
      experience — that requires the human-verify checkpoint below.
  guardrail_verdict: accepted
  degraded_signals: ["target_test (no test framework)", "mutation_check (no Stryker)", "adjacent_tests (no relevant suite)", "revert_and_reconfirm (adapted to mechanism-level test — native popup click not automatable in this environment, but this round uses a push-based harness matching the real <For> mechanism, correcting the previous round's pull-based blind spot)"]
files_changed:
  - "src/App.tsx: removed `import { reconcile } from \"solid-js/store\"` (incompatible with plain createSignal, caused the regression); replaced `setApps(reconcile(result, { key: \"identifier\" }))` in handlePickerFocus with a hand-rolled reference-preserving merge (Map-based lookup by identifier, reuse old object reference when identifier+display_name unchanged, always call setApps with a fresh top-level array)"

## Current Focus

reasoning_checkpoint:
  hypothesis: "The target-app <select> assigns the wrong app because its onFocus handler mutates the SolidJS `apps()` signal (feeding a reference-keyed `<For>`) on every open, and this async mutation always resolves while the native dropdown is still open (IPC latency << human click latency), corrupting the click-to-value mapping the browser resolves against."
  confirming_evidence:
    - "Evidence entry #1-2: single `setApps()` call site is the select's own `onFocus`, unconditional, every open, feeding a `<For>` that keys by object reference (not `app.identifier`) — guarantees full option-DOM teardown/rebuild every open regardless of whether the app set changed."
    - "Evidence entry #3: directly measured `list_running_apps_impl` at 25µs-10ms steady-state via a temporary timing probe (removed after measurement) — confirms the mutation-triggering fetch is fast enough to virtually always land inside a human's dropdown-open-to-click window, explaining 100% reproducibility rather than intermittent flakiness."
    - "Evidence entry #4: traced the entire value pipeline from `onChange` to the backend IPC calls and found zero transformation/indexing — ruling out an app-level logic bug as the source of the mismatch, isolating the defect to the browser/webview's handling of the mutated `<select>`."
  falsification_test: "If the picker's option list is NOT mutated while the dropdown can be open (e.g., app list is fetched once before the form/select becomes interactive, or `<For>` reuses stable references so no DOM change occurs when the app set is unchanged), the wrong-app-selected behavior should disappear on repeated manual opens without apps launching/quitting in between."
  fix_rationale: "Reconciling `setApps(result)` to preserve referentially-stable objects for apps whose identifier is unchanged between fetches (rather than always allocating a fresh array of fresh objects) makes `<For>` recognize the common case (no app launched/quit since last open) as 'no change' and skip DOM mutation entirely — eliminating the guaranteed-every-time collision at its source (the reference-equality diff) rather than papering over the symptom (e.g., debouncing the fetch, which would only reduce timing-collision odds, not remove the mechanism)."
  blind_spots: "Could not physically click the native macOS <select> popup via automation in this environment to get a literal click-by-click visual confirmation of value mismatch; confidence rests on (a) full elimination of the alternative code-level wiring hypothesis, (b) directly measured timing data, and (c) SolidJS's documented reference-equality `<For>` diffing behavior, not a live repro capture. Also have not fully verified whether the DOM-node-identity churn ALONE (with zero change in logical content/order) is sufficient to desync WebKit's open popup, versus requiring an actual reorder/add/remove of apps between fetches — the fix (reconciliation) resolves both cases either way, so this doesn't change the fix, but it's a gap in mechanistic certainty worth flagging to the user at the verification checkpoint."
  candidate_causes:
    - "code: `<For>` reference-equality diffing over freshly-deserialized objects on every fetch, feeding a native <select> — category: code"
    - "code/architecture: `onFocus` chosen as the refetch trigger point, which fires at the moment the native dropdown is about to open rather than before the select becomes interactive — category: code (a different code-level cause: WHEN the mutation happens, vs. category 1's WHAT the mutation destroys)"
  and_gate: "No — a single category (code) fully explains the symptom; the two candidate_causes above are two facets of the same code-level defect (a reference-churning list feeding a natively-rendered popup, refreshed at exactly the wrong trigger point), not independent contributing conditions from different categories (no config/environment/data cause is implicated — reproduces identically regardless of machine, profile, or which apps happen to be running)."

reasoning_checkpoint_v2:
  hypothesis: "`setApps(reconcile(result, { key: 'identifier' }))` breaks ALL reactivity for `apps()` (not just fails to fix the original bug) because `reconcile()` mutates the previous array in place and returns the SAME object reference every time, and `apps` is a plain `createSignal` whose setter uses default reference-equality (`current === value`) to decide whether to notify observers — so the write is silently swallowed forever, freezing `<For>` at its initial (empty) render."
  confirming_evidence:
    - "Read solid-js/store/dist/store.js:383-396 (`reconcile`) + :314-382 (`applyState`) + :119-138 (`setProperty`): confirms `applyState` mutates `parent[property]` in place via direct index assignment and the reconcile closure returns `state` (the same object passed in) when its internal `applyState` call returns undefined — i.e., always the pre-existing reference for a top-level array reconciliation."
    - "Read solid-js/dist/solid.js:198-213 (`createSignal`) + :643-675 (`writeSignal`) + :150,155-157 (`equalFn`/`signalOptions` defaults): confirms the setter calls `value(s.value)` for function-form updates, then `writeSignal` skips the observer-notify block entirely when `node.comparator(current, value)` (default `===`) is true — which it always is here since reconcile returns the identical reference."
    - "Empirical: node verify-reconcile-signal-bug.mjs Test 1 — a real `createEffect` subscriber run count stays at 1 (mount only) across two `setApps(reconcile(...))` calls, while the raw signal's `.length` silently changes underneath (Test 2: ref0===ref1===ref2, same object throughout). Test 3 reproduces why the PRIOR verification (mechanism-level revert-and-reconfirm in the first fix pass) falsely passed: it pull-called `mapArray`'s returned function directly, which re-reads the list fresh on every manual invocation regardless of whether the signal ever notified observers — not representative of how `<For>` is actually driven (a push-based tracked computation)."
  falsification_test: "If a real push-based `createEffect` subscribed to `apps()` re-runs and observes updated content after `setApps(reconcile(result, {key:'identifier'}))`, this hypothesis is wrong. Empirically it does NOT re-run (Test 1) — hypothesis confirmed."
  fix_rationale: "Replace `reconcile()` (a `createStore`-oriented primitive whose safety relies on store proxies' per-property fine-grained signals, not applicable to a plain top-level signal) with a hand-rolled merge: build a Map of the previous array's items keyed by `identifier`, map the fresh fetch result to reuse the OLD object reference wherever `identifier`+`display_name` are unchanged, otherwise use the new object, and always call `setApps()` with this freshly-allocated top-level array. This guarantees: (a) the top-level array reference always differs from the previous one -> the signal's default reference-equality check always sees a change -> `writeSignal` always notifies observers -> reactivity is never broken (fixes the regression), while (b) `<For>`'s own internal per-item reference diffing (`mapArray`, solid.js:1100-1183, which compares `items[i] === newItems[i]`) sees 'unchanged' for apps whose reference was deliberately reused -> no `<option>` DOM churn for those, preserving the original bug's fix intent. Verified via verify-corrected-fix.mjs Tests A/B/C (all PASS) using the same real push-based `createEffect` + `mapArray` harness that exposed the regression, not the flawed pull-based harness from the first fix pass."
  blind_spots: "Same native-<select>-popup-click automation limitation as the first fix pass applies (cannot literally click a live macOS native dropdown in this environment) — mechanism-level confidence rests on directly reading solid-js's actual shipped source and reproducing both the break and the fix via a push-based reactive harness that matches how `<For>` is truly driven, not a live click capture. This is a stronger evidentiary basis than the first pass (which used a flawed pull-based harness) because it directly targets and resolves the specific push/pull distinction that caused the first verification to be misleading."
  candidate_causes:
    - "code: `reconcile()` applied to a plain `createSignal` instead of a `createStore` — category: code (API-contract mismatch between a store-oriented utility and a signal-based state cell)"
    - "code: the first fix's self-verification script exercised a pull-based read path (`mapped()` called directly) instead of the push-based path that actually drives `<For>` in the DOM — category: code (a different code-level cause: verification methodology gap, not the production defect itself, but the reason the regression escaped detection)"
  and_gate: "No — single category (code) fully explains the regression; the two candidate_causes are the production defect and the verification-methodology gap that let it ship, not independent contributing conditions from different categories."

next_action: DONE — human checkpoint responded "confirmed fixed". Session archived to resolved/.
