pub mod ipc;
pub mod persistence;
pub mod platform;
pub mod scheduler;
pub mod state;

use persistence::ProfileManager;
use scheduler::{Scheduler, SchedulerIntent};
use state::{Intent, StateActor, StateManager};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::mpsc;

#[cfg(target_os = "macos")]
use platform::macos::MacPlatformObserver;
use platform::PlatformObserver;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let (state_tx, state_rx) = mpsc::channel::<Intent>(100);
            let (sched_tx, sched_rx) = mpsc::channel::<SchedulerIntent>(100);
            // Two-Phase Dispatch channel: Scheduler → StateActor
            let (action_tx, action_rx) = mpsc::channel::<scheduler::ActionReady>(100);

            // ── Task 5.1: Initialize ProfileManager ──
            let profile_mgr = Arc::new(ProfileManager::from_app_handle(app.handle())?);
            app.manage(profile_mgr.clone());

            // ── Startup Restoration (RELY-04) ──
            // Send a single LoadProfile intent — the StateActor handles the bracketed
            // batch load, suppressing per-macro auto-saves (Pitfall 2: no N-write storm).
            let startup_tx = state_tx.clone();
            tauri::async_runtime::spawn(async move {
                #[cfg(debug_assertions)]
                eprintln!("[Startup] Dispatching LoadProfile(\"default\") intent");
                // CR-06: Use oneshot channel — result is silently dropped on startup.
                let (tx, _rx) = tokio::sync::oneshot::channel();
                let _ = startup_tx
                    .send(Intent::LoadProfile("default".to_string(), tx))
                    .await;
            });

            // Spawn Scheduler (single async task — no per-macro spawns)
            let scheduler = Scheduler::new(sched_rx, action_tx);
            tauri::async_runtime::spawn(async move {
                scheduler.run().await;
            });

            // Spawn the State Actor
            let app_handle = app.handle().clone();
            let actor = StateActor::new(state_rx, sched_tx, action_rx, app_handle, profile_mgr.clone());
            tauri::async_runtime::spawn(async move {
                actor.run().await;
            });

            // Make the sender available to Tauri commands
            let state_manager = StateManager::new(state_tx.clone());
            app.manage(state_manager);

            // Wire the state sender into the platform observer so
            // the callback can dispatch Intents.
            #[cfg(target_os = "macos")]
            {
                platform::macos::observer::set_state_tx(state_tx);
                let mut observer = MacPlatformObserver::new();
                observer.start_observing();
                // Observer is owned by Tauri managed state for full app lifetime (SAFE-02).
                app.manage(observer);
            }
            #[cfg(target_os = "windows")]
            {
                platform::windows::set_state_tx(state_tx);
                let mut observer = platform::windows::WindowsPlatformObserver::new();
                observer.start_observing();
                // Observer is long-lived; it does not implement Drop.
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::add_macro,
            ipc::remove_macro,
            ipc::set_macro_enabled,
            ipc::set_macro_target_app,
            ipc::get_state,
            ipc::get_active_app,
            ipc::bind_hotkey,
            ipc::unbind_hotkey,
            ipc::toggle_engine,
            ipc::request_accessibility,
            ipc::check_accessibility,
            ipc::set_macro_sequence,
            ipc::update_step_interval,
            ipc::save_profile,
            ipc::load_profile,
            ipc::delete_profile,
            ipc::list_profiles,
            ipc::list_running_apps,
            ipc::set_macro_trigger_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
