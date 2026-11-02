#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod autorun;
pub mod config;
mod daemon;
mod gui;
pub mod i18n;
mod ipc;
pub mod models;
pub mod sharing;
pub mod single_instance;
pub mod updater;

use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;
use ipc::IpcClient;

fn main() {
    // Clean up temporary update files if left from previous self-update
    if let Ok(exe_path) = env::current_exe() {
        let old = exe_path.with_extension("old");
        if old.exists() {
            let _ = std::fs::remove_file(old);
        }
        let pending = exe_path.with_extension("pending");
        if pending.exists() {
            let _ = std::fs::remove_file(pending);
        }
        let download = exe_path.with_extension("download");
        if download.exists() {
            let _ = std::fs::remove_file(download);
        }
        let extracted = exe_path.with_extension("extracted");
        if extracted.exists() {
            let _ = std::fs::remove_dir_all(extracted);
        }
    }

    let args: Vec<String> = env::args().collect();

    let is_daemon_flag = args.iter().any(|a| {
        a == "--daemon" || a == "-d" || a == "--service" || a == "-s"
    });

    let is_gui_flag = args.iter().any(|a| a == "--gui" || a == "-g");

    if let Some(pos) = args.iter().position(|a| a == "--overlay-window" || a == "-o") {
        let overlay_id = args.get(pos + 1).cloned().unwrap_or_default();
        let calibration = args.iter().any(|a| a == "--calibration");
        let duration_ms = args.iter().position(|a| a == "--duration").and_then(|p| args.get(p + 1)).and_then(|s| s.parse::<u32>().ok());
        let override_text = args.iter().position(|a| a == "--override-text").and_then(|p| args.get(p + 1)).cloned();
        let overlay_data = args.iter().position(|a| a == "--overlay-data").and_then(|p| args.get(p + 1)).cloned();
        if let Err(e) = gui::overlay_window::run_overlay_window(overlay_id, calibration, override_text, duration_ms, overlay_data) {
            eprintln!("Failed to launch overlay window: {e}");
        }
        return;
    }

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

    // Auto-elevate to Administrator rights by default for system-wide ETW & UIPI input control
    let no_elevate = args.iter().any(|a| a == "--no-elevate");
    #[cfg(windows)]
    if !no_elevate && try_self_elevate() {
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

pub fn ensure_daemon_running() {
    if IpcClient::is_daemon_running() {
        return;
    }

    let cfg = config::AppConfig::load();
    let exe_path = if cfg.mask_daemon_process {
        daemon::pe_mask::prepare_masked_daemon().unwrap_or_else(|_| env::current_exe().unwrap_or_default())
    } else {
        env::current_exe().unwrap_or_default()
    };

    if exe_path.exists() {
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

#[cfg(windows)]
fn try_self_elevate() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOASYNC};
    use windows::core::{HSTRING, PCWSTR};

    if daemon::watcher::is_elevated() {
        return false;
    }

    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_str = args.iter().map(|a| {
        if a.contains(' ') {
            format!("\"{}\"", a)
        } else {
            a.clone()
        }
    }).collect::<Vec<_>>().join(" ");

    let verb = HSTRING::from("runas");
    let file = HSTRING::from(exe_path.to_string_lossy().to_string());
    let params = HSTRING::from(args_str);

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_NORMAL.0,
        ..Default::default()
    };

    unsafe {
        ShellExecuteExW(&mut info).is_ok()
    }
}
