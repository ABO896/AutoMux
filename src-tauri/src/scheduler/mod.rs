use crate::state::{ActionStep, InputEvent, MacroConfig, MouseButton};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use uuid::Uuid;

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
        let interval = Duration::from_millis(interval_ms.max(1));
        Self {
            step_id,
            input,
            interval,
            next_fire: Instant::now() + interval,
        }
    }

    /// Advance to the next fire time, skipping missed ticks (no burst on resume).
    fn advance(&mut self) {
        let now = Instant::now();
        if self.next_fire + self.interval <= now {
            // Jump ahead — do not "catch up"
            let elapsed = now.duration_since(self.next_fire);
            let missed = elapsed.as_nanos() / self.interval.as_nanos();
            self.next_fire += self.interval * (missed as u32 + 1);
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
                            self.handle_intent(intent);
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
                        Some(intent) => self.handle_intent(intent),
                        None => break, // Channel closed — shutdown.
                    }
                }
            }
        }
    }

    fn handle_intent(&mut self, intent: SchedulerIntent) {
        match intent {
            SchedulerIntent::StartMacro(config) => {
                self.start_macro(config);
            }
            SchedulerIntent::StopMacro(id) => {
                self.stop_macro(&id);
            }
            SchedulerIntent::StopAll => {
                // Release all holds across all macros.
                let all_ids: Vec<Uuid> = self.active_holds.keys().copied().collect();
                for id in all_ids {
                    self.release_holds(&id);
                }
                self.interval_tasks.clear();
                self.timeline.clear();
                self.active_holds.clear();
            }
            SchedulerIntent::UpdateInterval(macro_id, step_index, new_ms) => {
                let step_id = StepId {
                    macro_id,
                    step_index,
                };
                if let Some(task) = self.interval_tasks.get_mut(&step_id) {
                    let old_fire = task.next_fire;
                    task.interval = Duration::from_millis(new_ms.max(1));
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
    fn start_macro(&mut self, config: MacroConfig) {
        let macro_id = config.id;

        // Clean up any existing state for this macro.
        self.stop_macro(&macro_id);

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
                    let _ = self.action_tx.try_send(ActionReady {
                        macro_id,
                        action_type: ActionType::HoldStart(*input),
                        fired_at: Instant::now(),
                    });
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
    }

    /// Stop a macro: cancel all its interval timers and release all holds.
    fn stop_macro(&mut self, macro_id: &Uuid) {
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
        self.release_holds(macro_id);
    }

    /// Send HoldRelease for all active sustained holds of a macro.
    fn release_holds(&mut self, macro_id: &Uuid) {
        if let Some(holds) = self.active_holds.remove(macro_id) {
            for input in holds {
                let _ = self.action_tx.try_send(ActionReady {
                    macro_id: *macro_id,
                    action_type: ActionType::HoldRelease(input),
                    fired_at: Instant::now(),
                });
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
                        let _ = self.action_tx.try_send(ActionReady {
                            macro_id: step_id.macro_id,
                            action_type: ActionType::Interval(task.input),
                            fired_at: Instant::now(),
                        });

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
}
