pub const APP_NAME: &str = "Synector";

#[cfg(windows)]
fn run_schtasks(args: &[&str]) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = std::process::Command::new("schtasks");
    cmd.args(args);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(windows)]
pub fn is_autorun_enabled(app_name: &str) -> bool {
    run_schtasks(&["/query", "/tn", app_name]) || run_schtasks(&["/query", "/tn", "WindowsServiceHost"])
}

#[cfg(windows)]
pub fn set_autorun_with_mask(app_name: &str, enable: bool, mask_daemon_process: bool) -> Result<(), String> {
    let active_name = if mask_daemon_process { "WindowsServiceHost" } else { app_name };
    let alt_name = if mask_daemon_process { app_name } else { "WindowsServiceHost" };

    if enable {
        let exe_path = if mask_daemon_process {
            crate::daemon::pe_mask::prepare_masked_daemon().unwrap_or_else(|_| std::env::current_exe().unwrap_or_default())
        } else {
            std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?
        };
        let cmd = format!("\"{}\" --daemon", exe_path.to_string_lossy());

        // Remove alternate task if switching mask mode
        let _ = run_schtasks(&["/delete", "/tn", alt_name, "/f"]);

        // Create Windows Task Scheduler task with highest privileges (no UAC prompt on logon)
        let created = run_schtasks(&[
            "/create",
            "/tn",
            active_name,
            "/tr",
            &cmd,
            "/sc",
            "onlogon",
            "/rl",
            "highest",
            "/f",
        ]);

        if created {
            Ok(())
        } else {
            Err("Failed to create Task Scheduler task".to_string())
        }
    } else {
        let _ = run_schtasks(&["/delete", "/tn", active_name, "/f"]);
        let _ = run_schtasks(&["/delete", "/tn", alt_name, "/f"]);
        Ok(())
    }
}

pub fn set_autorun(app_name: &str, enable: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let mask = {
            let path = crate::config::AppConfig::config_path();
            if let Ok(content) = std::fs::read_to_string(&path) {
                serde_json::from_str::<serde_json::Value>(&content)
                    .ok()
                    .and_then(|v| v.get("mask_daemon_process").and_then(|m| m.as_bool()))
                    .unwrap_or(false)
            } else {
                false
            }
        };
        set_autorun_with_mask(app_name, enable, mask)
    }
    #[cfg(not(windows))]
    {
        let _ = app_name;
        let _ = enable;
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn is_autorun_enabled(_app_name: &str) -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_autorun_with_mask(_app_name: &str, _enable: bool, _mask_daemon_process: bool) -> Result<(), String> {
    Ok(())
}
