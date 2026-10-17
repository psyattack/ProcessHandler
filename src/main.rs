#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod autorun;
pub mod config;
mod daemon;
mod gui;
pub mod i18n;
mod ipc;
mod models;
pub mod single_instance;

use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;
use ipc::IpcClient;

fn main() {
    let args: Vec<String> = env::args().collect();

    let is_daemon_flag = args.iter().any(|a| {
        a == "--daemon" || a == "-d" || a == "--service" || a == "-s"
    });

    let is_gui_flag = args.iter().any(|a| a == "--gui" || a == "-g");

    if args.iter().any(|a| a == "--status") {
        match IpcClient::call(&ipc::IpcRequest::GetState) {
            Ok(ipc::IpcResponse::State(state)) => {
                println!("DAEMON STATUS: Online");
                println!("  Rules: {} (active: {})", state.total_rules_count, state.active_rules_count);
                println!("  Daemon Memory: {:.2} MB", state.daemon_memory_bytes as f64 / 1_048_576.0);
            }
            Ok(other) => println!("Daemon response: {:?}", other),
            Err(e) => println!("DAEMON STATUS: Offline ({e})"),
        }
        return;
    }

    if is_daemon_flag {
        daemon::run_daemon();
    } else if is_gui_flag {
        if let Err(e) = gui::run_gui() {
            eprintln!("Failed to launch GUI: {e}");
        }
    } else {
        // Default execution (e.g. user clicked executable)
        ensure_daemon_running();
        if let Err(e) = gui::run_gui() {
            eprintln!("Failed to launch GUI: {e}");
        }
    }
}

fn ensure_daemon_running() {
    if IpcClient::is_daemon_running() {
        return;
    }

    if let Ok(exe_path) = env::current_exe() {
        let mut cmd = Command::new(exe_path);
        cmd.arg("--daemon");
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let _ = cmd.spawn();

        // Wait up to 2 seconds for pipe to be available
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(100));
            if IpcClient::is_daemon_running() {
                break;
            }
        }
    }
}
