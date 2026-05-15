pub mod platform;
pub mod ipc;
pub mod persistence;
pub mod scheduler;
pub mod state;

use persistence::ProfileManager;
use scheduler::{Scheduler, SchedulerIntent};
use state::{Intent, StateActor, StateManager};
use tokio::sync::mpsc;
use tauri::Manager;

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
            let profile_mgr = ProfileManager::from_app_handle(app.handle())?;
            app.manage(profile_mgr.clone());

            // ── Task 5.3: Startup Restoration ──
            // Load the default profile and replay macros into the StateActor.
            let startup_tx = state_tx.clone();
            let startup_mgr = profile_mgr.clone();
            tauri::async_runtime::spawn(async move {
                let profile = startup_mgr.load_or_create_default().await;
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Startup] Restored profile '{}' with {} macros",
                    profile.name,
                    profile.macros.len()
                );
                for (_, config) in profile.macros {
                    let _ = startup_tx.send(Intent::AddMacro(config)).await;
                }
            });

            // Spawn Scheduler (single async task — no per-macro spawns)
            let scheduler = Scheduler::new(sched_rx, action_tx);
            tauri::async_runtime::spawn(async move {
                scheduler.run().await;
            });

            // Spawn the State Actor
            let app_handle = app.handle().clone();
            let actor = StateActor::new(state_rx, sched_tx, action_rx, app_handle);
            tauri::async_runtime::spawn(async move {
                actor.run().await;
            });

            // Make the sender available to Tauri commands
            let state_manager = StateManager::new(state_tx.clone());
            app.manage(state_manager);

            // Wire the state sender into the platform observer so
            // the CGEventTap hotkey callback can dispatch Intents.
            #[cfg(target_os = "macos")]
            {
                platform::macos::observer::set_state_tx(state_tx);
                let mut observer = MacPlatformObserver::new();
                observer.start_observing();
                // Observer is long-lived; leak it intentionally so it
                // outlives the setup closure (its threads + ObjC refs
                // are all 'static).
                std::mem::forget(observer);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

