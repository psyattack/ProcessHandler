pub mod actions;
pub mod engine;
pub mod ipc_server;
pub mod system_events;
pub mod tray;
pub mod watcher;

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::AppConfig;
use crate::daemon::engine::DaemonEngine;
use crate::daemon::ipc_server::IpcServer;
use crate::daemon::system_events::SystemEventsWatcher;
use crate::daemon::tray::TrayManager;

pub fn run_daemon() {
    let _instance_guard = match crate::single_instance::SingleInstanceGuard::acquire_daemon() {
        Some(guard) => guard,
        None => {
            eprintln!("[DAEMON] Another instance of ProcessHandler daemon is already running.");
            return;
        }
    };

    println!("[DAEMON] Initializing ProcessHandler background daemon...");

    let config = AppConfig::load_from_file(AppConfig::config_path());
    let engine = Arc::new(Mutex::new(DaemonEngine::new(config.clone())));

    // Create multi-threaded Tokio runtime for background tasks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime for daemon");

    // Log daemon initialization status
    {
        let autorun_status = if crate::autorun::is_autorun_enabled(crate::autorun::APP_NAME) {
            "enabled"
        } else {
            "disabled"
        };
        let engine_clone = Arc::clone(&engine);
        rt.block_on(async {
            let mut eng = engine_clone.lock().await;
            eng.log("INFO", "DAEMON", &format!("Daemon initialized (autorun: {})", autorun_status));
        });
    }

    // Spawn IPC Server
    let ipc_server = IpcServer::new(Arc::clone(&engine));
    rt.spawn(async move {
        ipc_server.run().await;
    });

    // Spawn Process Watcher Loop
    let engine_process = Arc::clone(&engine);
    rt.spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1500));
        loop {
            interval.tick().await;
            let mut eng = engine_process.lock().await;
            eng.tick_process_events().await;
        }
    });

    // Spawn System Events Watcher Loop
    let engine_sys = Arc::clone(&engine);
    rt.spawn(async move {
        let mut sys_watcher = SystemEventsWatcher::new();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(2000));
        loop {
            interval.tick().await;
            let events = sys_watcher.check_events();
            if !events.is_empty() {
                let mut eng = engine_sys.lock().await;
                for ev in events {
                    eng.handle_system_event(ev).await;
                }
            }
        }
    });

    // Spawn Schedule Watcher Loop
    let engine_sched = Arc::clone(&engine);
    rt.spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        {
            let mut eng = engine_sched.lock().await;
            let rules = eng.config.rules.clone();
            for rule in &rules {
                if rule.is_enabled {
                    if let Some(crate::models::Trigger::Schedule { schedule_type: crate::models::ScheduleType::AppStartup, .. }) = &rule.trigger {
                        eng.try_execute_rule(rule, "App Startup", 0).await;
                    }
                }
            }
        }

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut eng = engine_sched.lock().await;
            eng.tick_schedule().await;
        }
    });

    println!("[DAEMON] Service started. 0.0% CPU, listening on Named Pipe.");

    // Run native Win32 Tray on main thread
    match TrayManager::new(config.tray_icon_path.clone()) {
        Ok(tray) => {
            tray.run_message_loop();
        }
        Err(err) => {
            eprintln!("[DAEMON] Tray initialization error: {err}. Running headless loop.");
            std::thread::park();
        }
    }

    // Execute On App Close / Shutdown rules when tray message loop ends
    rt.block_on(async {
        let mut eng = engine.lock().await;
        eng.execute_shutdown_rules().await;
    });
}
