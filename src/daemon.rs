pub mod actions;
pub mod discord_rpc;
pub mod engine;
pub mod file_watcher;
pub mod hotkeys;
pub mod ipc_server;
pub mod lua_runner;
pub mod overlay_manager;
pub mod pe_mask;
pub mod system_events;
pub mod tray;
pub mod watcher;
pub mod webhook_server;

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

    // Pre-initialize overlay manager and install bundled fonts
    overlay_manager::init();

    let config = AppConfig::load_from_file(AppConfig::config_path());
    let engine = Arc::new(Mutex::new(DaemonEngine::new(config.clone())));

    // Create multi-threaded Tokio runtime for background tasks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime for daemon");

    // Initialize Global Hotkey Listener
    let (hotkey_tx, mut hotkey_rx) = tokio::sync::mpsc::unbounded_channel::<hotkeys::HotkeyEvent>();
    let hotkey_handle = hotkeys::start_hotkey_listener(hotkey_tx);
    hotkey_handle.reload(&config.rules);

    {
        let engine_clone = Arc::clone(&engine);
        let h_handle = hotkey_handle.clone();
        rt.block_on(async {
            let mut eng = engine_clone.lock().await;
            eng.hotkey_handle = Some(h_handle);
        });
    }

    // Spawn Global Hotkey Event Loop (supports Normal toggle, Hold mode, and Infinite macro cancellation)
    let engine_hotkey = Arc::clone(&engine);
    let hold_tasks: Arc<tokio::sync::Mutex<std::collections::HashMap<String, tokio::task::AbortHandle>>> =
        Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

    let hold_tasks_loop = Arc::clone(&hold_tasks);
    rt.spawn(async move {
        while let Some(ev) = hotkey_rx.recv().await {
            let mut eng = engine_hotkey.lock().await;
            match ev {
                hotkeys::HotkeyEvent::Trigger(rule_id) => {
                    let was_looping = if let Some(h) = hold_tasks_loop.lock().await.remove(&rule_id) {
                        h.abort();
                        true
                    } else {
                        false
                    };

                    if eng.is_rule_executing(&rule_id) || was_looping {
                        eng.stop_rule_execution(&rule_id);
                        crate::daemon::actions::release_all_simulated_inputs();
                        eng.rule_last_triggered.insert(rule_id.clone(), std::time::Instant::now());
                    } else if let Some(rule) = eng.config.rules.iter().find(|r| r.id == rule_id).cloned() {
                        if rule.is_enabled {
                            let is_repeat = matches!(rule.trigger, Some(crate::models::Trigger::GlobalHotkey { repeat_while_held: true, .. }));
                            if is_repeat {
                                let engine_for_loop = Arc::clone(&engine_hotkey);
                                let rule_id_clone = rule_id.clone();
                                let hold_tasks_inner = Arc::clone(&hold_tasks_loop);

                                let task = tokio::spawn(async move {
                                    loop {
                                        let handle_opt = {
                                            let mut eng = engine_for_loop.lock().await;
                                            if let Some(r) = eng.config.rules.iter().find(|r| r.id == rule_id_clone).cloned() {
                                                if !r.is_enabled {
                                                    break;
                                                }
                                                eng.try_execute_rule(&r, "Global Hotkey (Loop)", 0).await
                                            } else {
                                                break;
                                            }
                                        };

                                        if let Some(handle) = handle_opt {
                                            match handle.await {
                                                Ok(()) => {},
                                                Err(_) => {
                                                    break;
                                                }
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                    hold_tasks_inner.lock().await.remove(&rule_id_clone);
                                });
                                hold_tasks_loop.lock().await.insert(rule_id, task.abort_handle());
                            } else {
                                let _ = eng.try_execute_rule(&rule, "Global Hotkey", 0).await;
                            }
                        }
                    }
                }
                hotkeys::HotkeyEvent::HoldStart(rule_id) => {
                    if let Some(h) = hold_tasks_loop.lock().await.remove(&rule_id) {
                        h.abort();
                    }

                    if !eng.is_rule_executing(&rule_id) {
                        if let Some(rule) = eng.config.rules.iter().find(|r| r.id == rule_id).cloned() {
                            if rule.is_enabled {
                                let engine_for_hold = Arc::clone(&engine_hotkey);
                                let rule_id_clone = rule_id.clone();
                                let hold_tasks_inner = Arc::clone(&hold_tasks_loop);

                                let task = tokio::spawn(async move {
                                    loop {
                                        if !crate::daemon::hotkeys::is_hotkey_held(&rule_id_clone) {
                                            break;
                                        }

                                        let handle_opt = {
                                            let mut eng = engine_for_hold.lock().await;
                                            if let Some(r) = eng.config.rules.iter().find(|r| r.id == rule_id_clone).cloned() {
                                                if !r.is_enabled {
                                                    break;
                                                }
                                                eng.try_execute_rule(&r, "Global Hotkey (Hold)", 0).await
                                            } else {
                                                break;
                                            }
                                        };

                                        if let Some(handle) = handle_opt {
                                            match handle.await {
                                                Ok(()) => {},
                                                Err(_) => {
                                                    // Rule execution task was cancelled / aborted; stop loop immediately!
                                                    break;
                                                }
                                            }
                                        } else {
                                            break;
                                        }

                                        if !crate::daemon::hotkeys::is_hotkey_repeat_held(&rule_id_clone) {
                                            break;
                                        }

                                        if !crate::daemon::hotkeys::is_hotkey_held(&rule_id_clone) {
                                            break;
                                        }

                                        // Yield brief pause between loop cycles so input queue & events can breathe
                                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                                    }

                                    let mut tasks = hold_tasks_inner.lock().await;
                                    tasks.remove(&rule_id_clone);
                                });

                                hold_tasks_loop.lock().await.insert(rule_id.clone(), task.abort_handle());
                            }
                        }
                    }
                }
                hotkeys::HotkeyEvent::HoldStop(rule_id) => {
                    if let Some(h) = hold_tasks_loop.lock().await.remove(&rule_id) {
                        h.abort();
                    }
                    eng.stop_rule_execution(&rule_id);
                    crate::daemon::actions::release_all_simulated_inputs();
                }
            }
        }
    });

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
                        let _ = eng.try_execute_rule(rule, "App Startup", 0).await;
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

    // Spawn Webhook Server Loop
    let engine_webhook = Arc::clone(&engine);
    rt.spawn(async move {
        let server = crate::daemon::webhook_server::WebhookServer::new(
            engine_webhook,
            crate::daemon::webhook_server::DEFAULT_WEBHOOK_PORT,
        );
        server.run().await;
    });

    // Spawn File Watcher Loop
    let engine_file = Arc::clone(&engine);
    rt.spawn(async move {
        let mut file_watcher = crate::daemon::file_watcher::FileWatcherService::new(engine_file);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1500));
        loop {
            interval.tick().await;
            file_watcher.poll_once().await;
        }
    });

    println!("[DAEMON] Service started. 0.0% CPU, listening on Named Pipe.");

    // Run native Win32 Tray on main thread
    match TrayManager::new(config.tray_icon_path.clone(), config.hide_tray_icon, config.mask_daemon_process) {
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
