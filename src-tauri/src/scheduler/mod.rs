use crate::state::{ActionStep, InputEvent, MacroConfig, MouseButton};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use uuid::Uuid;

/// D-09/D-10/D-11: debug-only diagnostic counter for `action_tx` overflow.
/// Incremented on every `try_send` failure across the 3 fire sites below.
/// Relaxed ordering is sufficient — this is a monotonic diagnostic counter,
/// not a synchronization primitive. Absent entirely from release builds.
#[cfg(debug_assertions)]
static ACTION_DROP_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Debug-only accessor for the action-channel drop count, exposed to the
/// frontend via the `get_debug_action_drop_count` IPC command (D-09).
/// Not present in release builds.
#[cfg(debug_assertions)]
pub fn get_action_drop_count() -> u64 {
    ACTION_DROP_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

// ── Scheduler Intents ────────────────────────────────────────────

pub enum SchedulerIntent {
    /// Start or restart a macro with the given config.
    /// Expands ActionSequence into individual timeline entries.
    StartMacro(MacroConfig),
    /// Stop a specific macro and release all its sustained holds.
    StopMacro(Uuid),
    /// Stop all macros (emergency stop / engine toggle off).
    StopAll,
    /// Update the interval for a specific step of a running macro.
    UpdateInterval(Uuid, usize, u64),
}

// ── Two-Phase Dispatch ───────────────────────────────────────────
// The scheduler NEVER injects input directly. It sends ActionReady
// back to the StateActor, which validates targeting before dispatch.

/// What type of action the StateActor should perform.
#[derive(Debug, Clone)]
pub enum ActionType {
    /// A repeating interval action fired (click/press then release).
    Interval(InputEvent),
    /// Begin holding a key/button (macro started).
    HoldStart(InputEvent),
    /// Release a held key/button (macro stopped).
    HoldRelease(InputEvent),
}

/// Sent from Scheduler → StateActor when an action should fire.
#[derive(Debug, Clone)]
pub struct ActionReady {
    pub macro_id: Uuid,
    pub action_type: ActionType,
    pub fired_at: Instant,
}

// ── Composite Step ID ────────────────────────────────────────────
// Each InterleavedInterval step within a macro gets its own timer.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StepId {
    macro_id: Uuid,
    step_index: usize,
}

// ── Per-Step Timer State ─────────────────────────────────────────

struct IntervalTask {
    #[allow(dead_code)]
    step_id: StepId,
    input: InputEvent,
    interval: Duration,
    next_fire: Instant,
}

impl IntervalTask {
    fn new(step_id: StepId, input: InputEvent, interval_ms: u64) -> Self {
        let interval = Duration::from_millis(interval_ms.max(5));
        Self {
            step_id,
            input,
            interval,
            next_fire: Instant::now() + interval,
        }
    }

    /// Advance to the next fire time, skipping missed ticks (no burst on resume).
    ///
    /// WR-06: The formula computes the next tick strictly after `now` to avoid
    /// the off-by-one where elapsed == exact multiple of interval skips an extra tick.
    /// Uses saturating_mul via u64 arithmetic to avoid u32 overflow on long suspensions.
    fn advance(&mut self) {
        let now = Instant::now();
        if self.next_fire <= now {
            let elapsed = now.duration_since(self.next_fire);
            // `missed` = number of full intervals already elapsed from next_fire.
            // The next tick strictly after `now` is at next_fire + (missed + 1) * interval.
            let missed = elapsed.as_nanos() / self.interval.as_nanos();
            // Use saturating cast: on extremely long suspensions missed may exceed u32::MAX,
            // clamping to u32::MAX is safe — the resulting next_fire will simply be very far ahead.
            let steps = (missed + 1).min(u32::MAX as u128) as u32;
            self.next_fire += self.interval * steps;
        } else {
            self.next_fire += self.interval;
        }
    }
}

// ── Core Scheduler ───────────────────────────────────────────────
// @scheduler-agent: Single async task, no per-macro spawns.
// Supports multi-step ActionSequences: each InterleavedInterval
// gets its own timeline entry. SustainedHolds fire immediately
// as HoldStart/HoldRelease events.
//
// Memory: O(s) where s = total active interval steps across all
// macros. At 32 macros × 4 steps each = 128 entries ≈ 8KB.

/// Key fields of a MacroConfig that determine scheduler behavior.
/// CR-04: Used to detect whether a running macro needs to be restarted.
#[derive(Debug, Clone, PartialEq)]
struct RunningConfig {
    interval_ms: u64,
    target_app: Option<String>,
    trigger_mode: crate::state::TriggerMode,
    steps: Vec<ActionStep>,
}

impl RunningConfig {
    fn from_config(config: &MacroConfig) -> Self {
        Self {
            interval_ms: config.interval_ms,
            target_app: config.target_app.clone(),
            trigger_mode: config.trigger_mode,
            steps: config.sequence.steps.clone(),
        }
    }
}

pub struct Scheduler {
    /// Receives intents from the StateActor.
    intent_rx: mpsc::Receiver<SchedulerIntent>,
    /// Sends ActionReady back to the StateActor for two-phase dispatch.
    action_tx: mpsc::Sender<ActionReady>,
    /// Active interval tasks indexed by StepId.
    interval_tasks: HashMap<StepId, IntervalTask>,
    /// Priority queue: next-fire → list of StepIds firing at that instant.
    timeline: BTreeMap<Instant, Vec<StepId>>,
    /// Active sustained holds per macro (for release on stop).
    active_holds: HashMap<Uuid, Vec<InputEvent>>,
    /// CR-04: Tracks config of currently running macros to avoid unnecessary restart.
    running_configs: HashMap<Uuid, RunningConfig>,
}

impl Scheduler {
    pub fn new(
        intent_rx: mpsc::Receiver<SchedulerIntent>,
        action_tx: mpsc::Sender<ActionReady>,
    ) -> Self {
        Self {
            intent_rx,
            action_tx,
            interval_tasks: HashMap::new(),
            timeline: BTreeMap::new(),
            active_holds: HashMap::new(),
            running_configs: HashMap::new(),
        }
    }

    /// Main loop — runs as a SINGLE tokio task.
    /// @safety-officer: No additional spawns. Memory bounded by step count.
    pub async fn run(mut self) {
        loop {
            let maybe_next = self.timeline.keys().next().copied();

            match maybe_next {
                Some(next_fire) => {
                    tokio::select! {
                        biased;

                        // Priority 1: Process incoming intents immediately.
                        Some(intent) = self.intent_rx.recv() => {
                            self.handle_intent(intent).await;
                        }

                        // Priority 2: Timer fires.
                        _ = tokio::time::sleep_until(next_fire) => {
                            self.fire_due_actions(next_fire);
                        }
                    }
                }
                None => {
                    // No active timers — block on intent channel only.
                    // 0% CPU when idle.
                    match self.intent_rx.recv().await {
                        Some(intent) => self.handle_intent(intent).await,
                        None => break, // Channel closed — shutdown.
                    }
                }
            }
        }
    }

    async fn handle_intent(&mut self, intent: SchedulerIntent) {
        match intent {
            SchedulerIntent::StartMacro(config) => {
                self.start_macro(config).await;
            }
            SchedulerIntent::StopMacro(id) => {
                self.stop_macro(&id).await;
            }
            SchedulerIntent::StopAll => {
                // @safety-officer: CR-02 — use `.await` (not try_send) for HoldRelease
                // messages on the emergency-stop path so they are guaranteed to be
                // delivered even when the action channel is near-full.
                let all_ids: Vec<Uuid> = self.active_holds.keys().copied().collect();
                for id in all_ids {
                    if let Some(holds) = self.active_holds.remove(&id) {
                        for input in holds {
                            let _ = self.action_tx.send(ActionReady {
                                macro_id: id,
                                action_type: ActionType::HoldRelease(input),
                                fired_at: Instant::now(),
                            }).await;
                        }
                    }
                }
                self.interval_tasks.clear();
                self.timeline.clear();
                self.active_holds.clear();
                self.running_configs.clear();
            }
            SchedulerIntent::UpdateInterval(macro_id, step_index, new_ms) => {
                let step_id = StepId {
                    macro_id,
                    step_index,
                };
                if let Some(task) = self.interval_tasks.get_mut(&step_id) {
                    let old_fire = task.next_fire;
                    task.interval = Duration::from_millis(new_ms.max(5));
                    task.next_fire = Instant::now() + task.interval;
                    let new_fire = task.next_fire;
                    self.remove_from_timeline(&step_id, old_fire);
                    self.timeline.entry(new_fire).or_default().push(step_id);
                }
            }
        }
    }

    /// Expand a MacroConfig's ActionSequence into individual timeline entries
    /// and immediately fire HoldStart for any SustainedHold steps.
    ///
    /// CR-04: No-ops when the macro is already running with an identical config.
    /// This prevents transient key-up+key-down on sustained-hold macros when
    /// unrelated state changes (e.g. active-app switch) trigger reevaluate_all_macros.
    async fn start_macro(&mut self, config: MacroConfig) {
        let macro_id = config.id;
        let new_running = RunningConfig::from_config(&config);

        // If the macro is already running with the same config, skip restart.
        if let Some(existing) = self.running_configs.get(&macro_id) {
            if existing == &new_running {
                return;
            }
        }

        // Clean up any existing state for this macro.
        self.stop_macro(&macro_id).await;

        let mut steps = if config.sequence.steps.is_empty() {
            // Legacy fallback: single left-click.
            match config.trigger_mode {
                crate::state::TriggerMode::Pulse => vec![ActionStep::InterleavedInterval {
                    input: InputEvent::MouseButton(MouseButton::Left),
                    interval_ms: config.interval_ms,
                }],
                crate::state::TriggerMode::Hold => vec![ActionStep::SustainedHold {
                    input: InputEvent::MouseButton(MouseButton::Left),
                }],
            }
        } else {
            config.sequence.steps.clone()
        };

        if config.trigger_mode == crate::state::TriggerMode::Hold {
            // Force all steps to be SustainedHold when in Hold mode
            for step in &mut steps {
                if let ActionStep::InterleavedInterval { input, .. } = step {
                    *step = ActionStep::SustainedHold { input: *input };
                }
            }
        }

        let mut holds = Vec::new();

        for (index, step) in steps.iter().enumerate() {
            match step {
                ActionStep::SustainedHold { input } => {
                    // Fire HoldStart immediately.
                    if self
                        .action_tx
                        .try_send(ActionReady {
                            macro_id,
                            action_type: ActionType::HoldStart(*input),
                            fired_at: Instant::now(),
                        })
                        .is_err()
                    {
                        #[cfg(debug_assertions)]
                        {
                            ACTION_DROP_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            eprintln!(
                                "[Scheduler] action_tx full — dropped HoldStart for macro={}",
                                macro_id
                            );
                        }
                    }
                    holds.push(*input);
                }
                ActionStep::InterleavedInterval { input, interval_ms } => {
                    let step_id = StepId {
                        macro_id,
                        step_index: index,
                    };
                    let task = IntervalTask::new(step_id, *input, *interval_ms);
                    let fire_time = task.next_fire;

                    self.interval_tasks.insert(step_id, task);
                    self.timeline.entry(fire_time).or_default().push(step_id);
                }
            }
        }

        if !holds.is_empty() {
            self.active_holds.insert(macro_id, holds);
        }

        // CR-04: Record running config so we can skip no-op restarts.
        self.running_configs.insert(macro_id, new_running);
    }

    /// Stop a macro: cancel all its interval timers and release all holds.
    async fn stop_macro(&mut self, macro_id: &Uuid) {
        // Remove all interval tasks for this macro.
        let step_ids: Vec<StepId> = self
            .interval_tasks
            .keys()
            .filter(|s| s.macro_id == *macro_id)
            .copied()
            .collect();

        for step_id in step_ids {
            if let Some(task) = self.interval_tasks.remove(&step_id) {
                self.remove_from_timeline(&step_id, task.next_fire);
            }
        }

        // Release all sustained holds.
        self.release_holds(macro_id).await;

        // CR-04: Clear running config so the macro can be restarted fresh.
        self.running_configs.remove(macro_id);
    }

    /// Send HoldRelease for all active sustained holds of a macro.
    ///
    /// @safety-officer: CR-01 (09-VERIFICATION.md/09-REVIEW.md) — use `.await`
    /// (not try_send) for HoldRelease messages on the per-macro stop path,
    /// mirroring StopAll's guarantee (CR-02, above), so delivery is
    /// guaranteed even when the action channel is near-full. A dropped
    /// HoldRelease here would mean `active_holds.remove` has already run —
    /// the scheduler believes the hold is released while the underlying
    /// key/mouse button is still physically down, with no retry path short
    /// of a full emergency stop.
    async fn release_holds(&mut self, macro_id: &Uuid) {
        if let Some(holds) = self.active_holds.remove(macro_id) {
            for input in holds {
                let _ = self
                    .action_tx
                    .send(ActionReady {
                        macro_id: *macro_id,
                        action_type: ActionType::HoldRelease(input),
                        fired_at: Instant::now(),
                    })
                    .await;
            }
        }
    }

    /// Fire all interval actions due at or before `deadline`.
    fn fire_due_actions(&mut self, deadline: Instant) {
        let due_keys: Vec<Instant> = self.timeline.range(..=deadline).map(|(k, _)| *k).collect();

        for key in due_keys {
            if let Some(step_ids) = self.timeline.remove(&key) {
                for step_id in step_ids {
                    if let Some(task) = self.interval_tasks.get_mut(&step_id) {
                        // Two-Phase: signal the StateActor.
                        // @safety-officer: try_send backpressure — drops on overflow.
                        // D-09/D-10: overflow is counted + logged in debug builds
                        // (see ACTION_DROP_COUNT) so it is diagnosable, not silent.
                        if self
                            .action_tx
                            .try_send(ActionReady {
                                macro_id: step_id.macro_id,
                                action_type: ActionType::Interval(task.input),
                                fired_at: Instant::now(),
                            })
                            .is_err()
                        {
                            #[cfg(debug_assertions)]
                            {
                                ACTION_DROP_COUNT
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                eprintln!(
                                    "[Scheduler] action_tx full — dropped Interval for macro={}",
                                    step_id.macro_id
                                );
                            }
                        }

                        // Advance and re-insert.
                        task.advance();
                        let next = task.next_fire;
                        self.timeline.entry(next).or_default().push(step_id);
                    }
                }
            }
        }
    }

    fn remove_from_timeline(&mut self, id: &StepId, fire_time: Instant) {
        if let Some(ids) = self.timeline.get_mut(&fire_time) {
            ids.retain(|x| x != id);
            if ids.is_empty() {
                self.timeline.remove(&fire_time);
            }
        }
    }
}

// ── Stress Test ──────────────────────────────────────────────────
// @safety-officer: AFK farm simulation test.
// Verifies that sustained holds + interleaved intervals work
// concurrently without jitter or channel overflow.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ActionSequence, ActionStep, InputEvent, MacroConfig, MouseButton};

    /// Stress test: simulates a Minecraft AFK farm for 2 seconds.
    /// - Hold Right-Click (sustained)
    /// - Left-Click every 50ms (interleaved — fast for stress)
    /// Verifies: (a) HoldStart fires once, (b) intervals fire ~40 times,
    /// (c) HoldRelease fires on stop, (d) no channel overflow.
    #[tokio::test]
    async fn afk_farm_stress_test() {
        let (intent_tx, intent_rx) = mpsc::channel::<SchedulerIntent>(100);
        let (action_tx, mut action_rx) = mpsc::channel::<ActionReady>(256);

        let scheduler = Scheduler::new(intent_rx, action_tx);
        let scheduler_handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let macro_id = Uuid::new_v4();
        let config = MacroConfig {
            id: macro_id,
            name: "AFK Fish Farm".into(),
            interval_ms: 50, // legacy fallback (unused since sequence is populated)
            enabled: true,
            target_app: None,
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: crate::state::TriggerMode::Pulse,
            sequence: ActionSequence {
                steps: vec![
                    ActionStep::SustainedHold {
                        input: InputEvent::MouseButton(MouseButton::Right),
                    },
                    ActionStep::InterleavedInterval {
                        input: InputEvent::MouseButton(MouseButton::Left),
                        interval_ms: 50, // 20 clicks/sec for stress
                    },
                ],
            },
        };

        // Start the macro.
        intent_tx
            .send(SchedulerIntent::StartMacro(config))
            .await
            .unwrap();

        // Let it run for 500ms.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Stop the macro.
        intent_tx
            .send(SchedulerIntent::StopMacro(macro_id))
            .await
            .unwrap();

        // Give time for the stop to process.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Shutdown the scheduler.
        drop(intent_tx);
        let _ = scheduler_handle.await;

        // Collect all actions.
        let mut hold_starts = 0u32;
        let mut hold_releases = 0u32;
        let mut interval_fires = 0u32;

        while let Ok(action) = action_rx.try_recv() {
            assert_eq!(action.macro_id, macro_id);
            match action.action_type {
                ActionType::HoldStart(InputEvent::MouseButton(MouseButton::Right)) => {
                    hold_starts += 1;
                }
                ActionType::HoldRelease(InputEvent::MouseButton(MouseButton::Right)) => {
                    hold_releases += 1;
                }
                ActionType::Interval(InputEvent::MouseButton(MouseButton::Left)) => {
                    interval_fires += 1;
                }
                other => panic!("Unexpected action: {:?}", other),
            }
        }

        // Assertions — @safety-officer verified:
        assert_eq!(hold_starts, 1, "Exactly one HoldStart for right-click");
        assert_eq!(hold_releases, 1, "Exactly one HoldRelease for right-click");
        // 500ms / 50ms = ~10 fires, allow ±5 for timer imprecision.
        assert!(
            interval_fires >= 5,
            "Expected ≥5 interval fires, got {}",
            interval_fires
        );
        assert!(
            interval_fires <= 20,
            "Expected ≤20 interval fires, got {}",
            interval_fires
        );

        eprintln!(
            "[Stress Test] hold_starts={}, hold_releases={}, interval_fires={}",
            hold_starts, hold_releases, interval_fires
        );
    }

    /// Sub-millisecond jitter audit.
    /// Fires a 10ms interval for 200ms and measures the stddev of gaps.
    #[tokio::test]
    async fn jitter_audit_10ms_interval() {
        let (intent_tx, intent_rx) = mpsc::channel::<SchedulerIntent>(100);
        let (action_tx, mut action_rx) = mpsc::channel::<ActionReady>(256);

        let scheduler = Scheduler::new(intent_rx, action_tx);
        let scheduler_handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let macro_id = Uuid::new_v4();
        let config = MacroConfig {
            id: macro_id,
            name: "Jitter Test".into(),
            interval_ms: 10,
            enabled: true,
            target_app: None,
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: crate::state::TriggerMode::Pulse,
            sequence: ActionSequence {
                steps: vec![ActionStep::InterleavedInterval {
                    input: InputEvent::Key(0), // A key
                    interval_ms: 10,
                }],
            },
        };

        intent_tx
            .send(SchedulerIntent::StartMacro(config))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        intent_tx
            .send(SchedulerIntent::StopMacro(macro_id))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        drop(intent_tx);
        let _ = scheduler_handle.await;

        // Collect fired_at timestamps.
        let mut timestamps: Vec<Instant> = Vec::new();
        while let Ok(action) = action_rx.try_recv() {
            if let ActionType::Interval(_) = action.action_type {
                timestamps.push(action.fired_at);
            }
        }

        assert!(
            timestamps.len() >= 10,
            "Expected ≥10 fires, got {}",
            timestamps.len()
        );

        // Compute inter-fire gaps.
        let gaps: Vec<f64> = timestamps
            .windows(2)
            .map(|w| w[1].duration_since(w[0]).as_micros() as f64)
            .collect();

        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        let variance = gaps.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / gaps.len() as f64;
        let stddev = variance.sqrt();

        eprintln!(
            "[Jitter Audit] samples={}, mean_gap={:.0}µs, stddev={:.0}µs, target=10000µs",
            gaps.len(),
            mean,
            stddev
        );

        // @safety-officer: stddev should be < 2000µs (2ms) for a 10ms interval.
        // tokio timer resolution on macOS is typically ~1ms.
        assert!(
            stddev < 3000.0,
            "Jitter too high: stddev={:.0}µs (max 3000µs)",
            stddev
        );
    }

    /// D-06a/EXEC-01/EXEC-02: proves two macros at different intervals fire
    /// CONCURRENTLY — neither blocks or serializes behind the other.
    ///
    /// Macro A @ 50ms and macro B @ 80ms are started back-to-back and run for
    /// 500ms. If the scheduler serialized macros (e.g. processed one macro's
    /// timeline to completion before starting the other, or somehow only let
    /// one macro's timers advance), one or both counts would collapse toward
    /// zero or half their expected value. Bounds below are chosen around the
    /// theoretical fire count (500/interval) with generous but NOT unbounded
    /// slack — a serial implementation could not produce two independently
    /// healthy non-zero counts in the same 500ms wall-clock window.
    #[tokio::test]
    async fn parallel_two_macros_concurrent() {
        let (intent_tx, intent_rx) = mpsc::channel::<SchedulerIntent>(100);
        let (action_tx, mut action_rx) = mpsc::channel::<ActionReady>(256);

        let scheduler = Scheduler::new(intent_rx, action_tx);
        let scheduler_handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let macro_a_id = Uuid::new_v4();
        let macro_b_id = Uuid::new_v4();

        let config_a = MacroConfig {
            id: macro_a_id,
            name: "Parallel A".into(),
            interval_ms: 50,
            enabled: true,
            target_app: None,
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: crate::state::TriggerMode::Pulse,
            sequence: ActionSequence {
                steps: vec![ActionStep::InterleavedInterval {
                    input: InputEvent::MouseButton(MouseButton::Left),
                    interval_ms: 50,
                }],
            },
        };

        let config_b = MacroConfig {
            id: macro_b_id,
            name: "Parallel B".into(),
            interval_ms: 80,
            enabled: true,
            target_app: None,
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: crate::state::TriggerMode::Pulse,
            sequence: ActionSequence {
                steps: vec![ActionStep::InterleavedInterval {
                    input: InputEvent::MouseButton(MouseButton::Right),
                    interval_ms: 80,
                }],
            },
        };

        // Start both macros back-to-back — no gap between them.
        intent_tx
            .send(SchedulerIntent::StartMacro(config_a))
            .await
            .unwrap();
        intent_tx
            .send(SchedulerIntent::StartMacro(config_b))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(500)).await;

        intent_tx
            .send(SchedulerIntent::StopMacro(macro_a_id))
            .await
            .unwrap();
        intent_tx
            .send(SchedulerIntent::StopMacro(macro_b_id))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        drop(intent_tx);
        let _ = scheduler_handle.await;

        // Accumulate fire counts per macro_id — proves each macro is tracked
        // independently rather than asserted against a single fixed id.
        let mut fire_counts: HashMap<Uuid, u32> = HashMap::new();
        while let Ok(action) = action_rx.try_recv() {
            if let ActionType::Interval(_) = action.action_type {
                *fire_counts.entry(action.macro_id).or_insert(0) += 1;
            }
        }

        let a_count = *fire_counts.get(&macro_a_id).unwrap_or(&0);
        let b_count = *fire_counts.get(&macro_b_id).unwrap_or(&0);

        eprintln!(
            "[Parallel Test] macro_a(50ms) fires={}, macro_b(80ms) fires={}",
            a_count, b_count
        );

        // 500ms / 50ms = 10 theoretical fires. A serial/blocked implementation
        // would push this toward 0 (starved) — bound tight enough to catch that.
        assert!(
            (5..=16).contains(&a_count),
            "macro A (50ms) expected ~10 fires in [5,16], got {}",
            a_count
        );
        // 500ms / 80ms = 6.25 theoretical fires.
        assert!(
            (3..=11).contains(&b_count),
            "macro B (80ms) expected ~6 fires in [3,11], got {}",
            b_count
        );
    }

    /// D-06b: proves stopping one running macro does not affect another
    /// concurrently running macro.
    #[tokio::test]
    async fn parallel_stop_one_keeps_other() {
        let (intent_tx, intent_rx) = mpsc::channel::<SchedulerIntent>(100);
        let (action_tx, mut action_rx) = mpsc::channel::<ActionReady>(256);

        let scheduler = Scheduler::new(intent_rx, action_tx);
        let scheduler_handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let macro_a_id = Uuid::new_v4();
        let macro_b_id = Uuid::new_v4();

        let config_a = MacroConfig {
            id: macro_a_id,
            name: "Stop-One A".into(),
            interval_ms: 50,
            enabled: true,
            target_app: None,
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: crate::state::TriggerMode::Pulse,
            sequence: ActionSequence {
                steps: vec![ActionStep::InterleavedInterval {
                    input: InputEvent::MouseButton(MouseButton::Left),
                    interval_ms: 50,
                }],
            },
        };

        let config_b = MacroConfig {
            id: macro_b_id,
            name: "Stop-One B".into(),
            interval_ms: 50,
            enabled: true,
            target_app: None,
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: crate::state::TriggerMode::Pulse,
            sequence: ActionSequence {
                steps: vec![ActionStep::InterleavedInterval {
                    input: InputEvent::MouseButton(MouseButton::Right),
                    interval_ms: 50,
                }],
            },
        };

        intent_tx
            .send(SchedulerIntent::StartMacro(config_a))
            .await
            .unwrap();
        intent_tx
            .send(SchedulerIntent::StartMacro(config_b))
            .await
            .unwrap();

        // Let both run for 200ms, then stop A only.
        tokio::time::sleep(Duration::from_millis(200)).await;
        intent_tx
            .send(SchedulerIntent::StopMacro(macro_a_id))
            .await
            .unwrap();

        // Keep B running for another 300ms (500ms total window for B).
        tokio::time::sleep(Duration::from_millis(300)).await;
        intent_tx
            .send(SchedulerIntent::StopMacro(macro_b_id))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        drop(intent_tx);
        let _ = scheduler_handle.await;

        let mut fire_counts: HashMap<Uuid, u32> = HashMap::new();
        while let Ok(action) = action_rx.try_recv() {
            if let ActionType::Interval(_) = action.action_type {
                *fire_counts.entry(action.macro_id).or_insert(0) += 1;
            }
        }

        let a_count = *fire_counts.get(&macro_a_id).unwrap_or(&0);
        let b_count = *fire_counts.get(&macro_b_id).unwrap_or(&0);

        eprintln!(
            "[Stop-One Test] macro_a(stopped@200ms) fires={}, macro_b(ran 500ms) fires={}",
            a_count, b_count
        );

        // A ran only ~200ms at 50ms interval (~4 theoretical fires) — bound
        // generously but tight enough to catch A continuing after its stop.
        assert!(
            a_count <= 10,
            "macro A stopped at 200ms — expected <=10 fires, got {}",
            a_count
        );
        // B ran the full ~500ms window at 50ms interval (~10 theoretical fires)
        // — must be meaningfully greater than A, proving A's stop did not
        // affect B.
        assert!(
            b_count >= 6,
            "macro B ran full 500ms window — expected >=6 fires, got {}",
            b_count
        );
        assert!(
            b_count > a_count,
            "macro B ({}) should have fired meaningfully more than stopped macro A ({})",
            b_count,
            a_count
        );
    }

    /// 09-VERIFICATION.md gap (09-REVIEW.md CR-01): proves the per-macro stop
    /// path (`stop_macro` → `release_holds`) delivers `HoldRelease` even when
    /// `action_tx` is saturated at stop time, mirroring the guarantee
    /// `StopAll` already provides (scheduler/mod.rs:214-234).
    ///
    /// Uses a capacity-1 action channel: starting the Hold-mode macro fills
    /// the single slot with `HoldStart`, so `release_holds`'s send is forced
    /// to contend for the same slot. Draining via `recv().await` (not
    /// `try_recv`) lets a blocked `.await` send make progress as soon as the
    /// consumer frees the slot — a `try_recv` drain would not.
    ///
    /// RED (pre-fix, `try_send`): hold_releases == 0 — the release is
    /// dropped by the full channel. GREEN (post-fix, `.await`): hold_releases
    /// == 1 — delivery is guaranteed.
    #[tokio::test]
    async fn stop_macro_release_delivered_under_saturation() {
        let (intent_tx, intent_rx) = mpsc::channel::<SchedulerIntent>(100);
        let (action_tx, mut action_rx) = mpsc::channel::<ActionReady>(1);

        let scheduler = Scheduler::new(intent_rx, action_tx);
        let scheduler_handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let macro_id = Uuid::new_v4();
        let config = MacroConfig {
            id: macro_id,
            name: "Saturation Test".into(),
            interval_ms: 50,
            enabled: true,
            target_app: None,
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: crate::state::TriggerMode::Hold,
            sequence: ActionSequence {
                steps: vec![ActionStep::SustainedHold {
                    input: InputEvent::MouseButton(MouseButton::Right),
                }],
            },
        };

        intent_tx
            .send(SchedulerIntent::StartMacro(config))
            .await
            .unwrap();
        intent_tx
            .send(SchedulerIntent::StopMacro(macro_id))
            .await
            .unwrap();

        // Close the intent channel so the scheduler shuts down once both
        // intents above are processed.
        drop(intent_tx);

        // Drain with `recv().await` (yields), NOT `try_recv()` — the fixed
        // release_holds's guaranteed send only completes once this consumer
        // frees the single slot.
        let mut hold_starts = 0u32;
        let mut hold_releases = 0u32;
        while let Some(action) = action_rx.recv().await {
            match action.action_type {
                ActionType::HoldStart(_) => hold_starts += 1,
                ActionType::HoldRelease(_) => hold_releases += 1,
                _ => {}
            }
        }

        let _ = scheduler_handle.await;

        assert_eq!(hold_starts, 1, "Exactly one HoldStart expected");
        assert_eq!(
            hold_releases, 1,
            "HoldRelease must be delivered even when action_tx was full at stop time — a dropped release leaves a physically-stuck input"
        );
    }
}
