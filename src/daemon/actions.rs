use std::collections::HashMap;
use std::process::Command;
use crate::models::{
    Action, AudioVolumeAction, MouseActionType, MouseButton, MouseCoordMode, PowerPlanType,
    ProcessPriorityLevel, SystemPowerType, WindowActionType,
};

#[cfg(windows)]
use windows::core::{BOOL, PCWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, WPARAM, CloseHandle, POINT};
#[cfg(windows)]
use windows::Win32::System::Shutdown::LockWorkStation;
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, SetPriorityClass, SetProcessAffinityMask,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_SET_QUOTA,
    REALTIME_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, ABOVE_NORMAL_PRIORITY_CLASS,
    NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS,
};
#[cfg(windows)]
use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageW, PostMessageW, EnumWindows, GetWindowThreadProcessId, ShowWindow, SetWindowPos,
    GetWindowLongW, SetWindowLongW, SetLayeredWindowAttributes, SetCursorPos, GetCursorPos,
    GetWindow, GW_OWNER, IsWindowVisible, GetWindowTextLengthW, GetWindowTextW,
    HWND_BROADCAST, HWND_TOPMOST, HWND_NOTOPMOST,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
    GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TOPMOST, WS_EX_TOOLWINDOW, LWA_ALPHA,
    WM_SYSCOMMAND, WM_CLOSE,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, MOUSEINPUT, MOUSE_EVENT_FLAGS,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP,
    MOUSEEVENTF_WHEEL, MOUSEEVENTF_MOVE,
    VIRTUAL_KEY, VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RETURN, VK_ESCAPE, VK_TAB, VK_SPACE,
    VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12,
    VK_BACK, VK_DELETE, VK_INSERT, VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_UP, VK_DOWN, VK_LEFT,
    VK_RIGHT, VK_CAPITAL, VK_SNAPSHOT,
};
#[cfg(windows)]
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME};

pub struct ActionRunner;

impl ActionRunner {
    pub async fn execute(action: &Action, context: &HashMap<String, String>) -> Result<(), String> {
        Self::execute_with_output(action, context).await.map(|_| ())
    }

    pub async fn execute_with_output(
        action: &Action,
        context: &HashMap<String, String>,
    ) -> Result<Option<String>, String> {
        match action {
            Action::ExecuteScript { interpreter, script, run_in_window } => {
                Self::run_script_internal(interpreter, script, *run_in_window, context)
            }
            other => {
                Self::execute_void_action(other, context).await?;
                Ok(None)
            }
        }
    }

    pub fn run_script_internal(
        interpreter: &crate::models::ScriptType,
        script: &str,
        run_in_window: bool,
        context: &HashMap<String, String>,
    ) -> Result<Option<String>, String> {
        match interpreter {
            crate::models::ScriptType::Lua => {
                let script_code = Self::interpolate_tokens(script, context);
                let outputs = std::sync::Arc::new(std::sync::Mutex::new(HashMap::new()));
                let res = crate::daemon::lua_runner::LuaRunner::execute(&script_code, context, outputs)?;
                Ok(if res.is_empty() { None } else { Some(res) })
            }
            crate::models::ScriptType::PowerShell => {
                let script_code = Self::interpolate_tokens(script, context);
                let mut cmd = Command::new("powershell");
                cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script_code]);
                for (k, v) in context {
                    cmd.env(format!("PH_{}", k.to_uppercase()), v);
                }
                if !run_in_window {
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x08000000);
                    }
                    let out = cmd.output().map_err(|e| format!("PowerShell error: {e}"))?;
                    let stdout_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !out.status.success() {
                        let stderr_str = String::from_utf8_lossy(&out.stderr).trim().to_string();
                        return Err(format!("PowerShell failed: {}", if stderr_str.is_empty() { stdout_str } else { stderr_str }));
                    }
                    Ok(if stdout_str.is_empty() { None } else { Some(stdout_str) })
                } else {
                    let _ = cmd.spawn().map_err(|e| format!("PowerShell spawn error: {e}"))?;
                    Ok(None)
                }
            }
            crate::models::ScriptType::Batch => {
                let cmd_text = Self::interpolate_tokens(script, context);
                let mut cmd = Command::new("cmd");
                cmd.args(["/C", &cmd_text]);
                if !run_in_window {
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x08000000);
                    }
                    let out = cmd.output().map_err(|e| format!("Cmd error: {e}"))?;
                    let stdout_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !out.status.success() {
                        let stderr_str = String::from_utf8_lossy(&out.stderr).trim().to_string();
                        return Err(format!("Cmd failed: {}", if stderr_str.is_empty() { stdout_str } else { stderr_str }));
                    }
                    Ok(if stdout_str.is_empty() { None } else { Some(stdout_str) })
                } else {
                    let _ = cmd.spawn().map_err(|e| format!("Cmd spawn error: {e}"))?;
                    Ok(None)
                }
            }
        }
    }

    pub async fn execute_void_action(action: &Action, context: &HashMap<String, String>) -> Result<(), String> {
        match action {
            Action::StartProcess {
                file_path,
                arguments,
                working_directory,
                run_as_admin,
                only_if_not_running,
                run_hidden,
                ..
            } => {
                let path = Self::interpolate_tokens(file_path, context);
                let args = Self::interpolate_tokens(arguments, context);

                if *only_if_not_running {
                    let exe_name = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or(&path)
                        .to_lowercase();

                    let snapshot = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
                    if snapshot.values().any(|name| name == &exe_name) {
                        return Ok(());
                    }
                }

                if *run_as_admin {
                    #[cfg(windows)]
                    {
                        // Run as admin via powershell Start-Process -Verb RunAs
                        let mut cmd = Command::new("powershell");
                        let ps_args = format!(
                            "Start-Process -FilePath '{}' -ArgumentList '{}' -Verb RunAs",
                            path.replace('\'', "''"),
                            args.replace('\'', "''")
                        );
                        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_args]);
                        let _ = cmd.spawn();
                        return Ok(());
                    }
                }

                let mut cmd = Command::new(&path);
                if !args.trim().is_empty() {
                    cmd.args(args.split_whitespace());
                }
                if let Some(wd) = working_directory {
                    cmd.current_dir(wd);
                }

                // Window style on Windows
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;
                    if *run_hidden {
                        cmd.creation_flags(CREATE_NO_WINDOW);
                    }
                }

                match cmd.spawn() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Failed to spawn process '{}': {e}", path)),
                }
            }

            Action::KillProcess {
                process_name,
                kill_tree,
                force,
            } => {
                let name = Self::interpolate_tokens(process_name, context);
                let mut cmd = Command::new("taskkill");
                if *force {
                    cmd.arg("/F");
                }
                if *kill_tree {
                    cmd.arg("/T");
                }
                cmd.args(["/IM", &name]);

                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                }

                let _ = cmd.spawn();
                Ok(())
            }

            Action::Window {
                target_process,
                window_action,
                x,
                y,
                width,
                height,
                transparency,
            } => {
                let default_proc = String::new();
                let proc_name = target_process.as_ref().unwrap_or(
                    context.get("ProcessName").unwrap_or(&default_proc)
                );
                let target_pid = context.get("ProcessId").and_then(|p| p.parse::<u32>().ok());
                #[cfg(windows)]
                {
                    window_control(
                        proc_name,
                        target_pid,
                        *window_action,
                        *x,
                        *y,
                        *width,
                        *height,
                        *transparency,
                    )?;
                }
                Ok(())
            }

            Action::ProcessTuning {
                target_process,
                priority,
                affinity_mask,
                trim_memory,
            } => {
                let default_proc = String::new();
                let proc_name = target_process.as_ref().unwrap_or(
                    context.get("ProcessName").unwrap_or(&default_proc)
                );
                let target_pid = context.get("ProcessId").and_then(|p| p.parse::<u32>().ok());
                #[cfg(windows)]
                {
                    tune_process(
                        proc_name,
                        target_pid,
                        *priority,
                        *affinity_mask,
                        *trim_memory,
                    )?;
                }
                Ok(())
            }

            Action::PowerPlan { plan } => {
                set_power_plan(*plan)?;
                Ok(())
            }

            Action::PlaySound { file_path, play_beep, beep_frequency, beep_duration_ms } => {
                if *play_beep {
                    #[cfg(windows)]
                    unsafe {
                        if let Some(freq) = beep_frequency {
                            let f = (*freq).clamp(37, 32767);
                            let d = beep_duration_ms.unwrap_or(200).clamp(10, 10000);
                            let _ = windows::Win32::System::Diagnostics::Debug::Beep(f, d);
                        } else {
                            let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(
                                windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE(0),
                            );
                        }
                    }
                } else {
                    let path = Self::interpolate_tokens(file_path, context);
                    play_sound_file(&path)?;
                }
                Ok(())
            }

            Action::Firewall { process_name, block } => {
                let proc_name = Self::interpolate_tokens(process_name, context);
                set_firewall_rule(&proc_name, *block)?;
                Ok(())
            }

            Action::Notification { title, message } => {
                let t = Self::interpolate_tokens(title, context);
                let m = Self::interpolate_tokens(message, context);

                #[cfg(windows)]
                {
                    let escaped_title = t.replace('\'', "''");
                    let escaped_msg = m.replace('\'', "''");
                    let ps_script = format!(
                        "Add-Type -AssemblyName System.Windows.Forms; \
                         $notify = New-Object System.Windows.Forms.NotifyIcon; \
                         $notify.Icon = [System.Drawing.SystemIcons]::Information; \
                         $notify.Visible = $true; \
                         $notify.ShowBalloonTip(4000, '{escaped_title}', '{escaped_msg}', [System.Windows.Forms.ToolTipIcon]::Info); \
                         Start-Sleep -Milliseconds 2500; \
                         $notify.Dispose()"
                    );

                    let mut cmd = Command::new("powershell");
                    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &ps_script]);
                    use std::os::windows::process::CommandExt;
                    cmd.creation_flags(0x08000000);
                    let _ = cmd.spawn();
                }
                Ok(())
            }

            Action::HttpWebhook {
                url,
                method,
                payload,
                ..
            } => {
                let target_url = Self::interpolate_tokens(url, context);
                let body = payload.as_ref().map(|p| Self::interpolate_tokens(p, context));
                let meth = method.clone();

                tokio::spawn(async move {
                    let mut cmd = Command::new("powershell");
                    let mut ps_cmd = format!("Invoke-RestMethod -Uri '{}' -Method '{}'", target_url, meth);
                    if let Some(b) = body {
                        ps_cmd.push_str(&format!(" -ContentType 'application/json' -Body '{}'", b.replace('\'', "''")));
                    }
                    cmd.args(["-NoProfile", "-Command", &ps_cmd]);
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x08000000);
                    }
                    let _ = cmd.spawn();
                });
                Ok(())
            }

            Action::SystemPower { power_type } => {
                match power_type {
                    SystemPowerType::Lock => {
                        #[cfg(windows)]
                        unsafe {
                            let _ = LockWorkStation();
                        }
                    }
                    SystemPowerType::TurnScreenOff => {
                        #[cfg(windows)]
                        unsafe {
                            const SC_MONITORPOWER: usize = 0xF170;
                            // 2 = turn monitor off
                            let _ = SendMessageW(
                                HWND_BROADCAST,
                                WM_SYSCOMMAND,
                                Some(windows::Win32::Foundation::WPARAM(SC_MONITORPOWER)),
                                Some(windows::Win32::Foundation::LPARAM(2)),
                            );
                        }
                    }
                    SystemPowerType::Sleep => {
                        let mut cmd = Command::new("rundll32.exe");
                        cmd.args(["powrprof.dll,SetSuspendState", "0,1,0"]);
                        let _ = cmd.spawn();
                    }
                    SystemPowerType::Restart => {
                        let mut cmd = Command::new("shutdown");
                        cmd.args(["/r", "/t", "0"]);
                        let _ = cmd.spawn();
                    }
                    SystemPowerType::Shutdown => {
                        let mut cmd = Command::new("shutdown");
                        cmd.args(["/s", "/t", "0"]);
                        let _ = cmd.spawn();
                    }
                }
                Ok(())
            }

            Action::Delay { seconds } => {
                tokio::time::sleep(std::time::Duration::from_secs_f64(*seconds)).await;
                Ok(())
            }

            Action::Display { width, height, refresh_rate, monitor_index } => {
                #[cfg(windows)]
                {
                    change_display_settings(*width, *height, *refresh_rate, *monitor_index)?;
                }
                Ok(())
            }

            Action::AudioControl {
                target,
                process_name,
                action_type,
                volume_level,
                left_volume,
                right_volume,
            } => {
                match target {
                    crate::models::AudioTarget::Playback => {
                        #[cfg(windows)]
                        {
                            if let Err(e) = set_audio_volume_or_mute(*action_type, *volume_level, *left_volume, *right_volume) {
                                eprintln!("Audio action error: {e}");
                            }
                        }
                    }
                    crate::models::AudioTarget::Microphone => {
                        #[cfg(windows)]
                        {
                            if let Err(e) = set_microphone_volume_or_mute(*action_type, *volume_level) {
                                eprintln!("Microphone action error: {e}");
                            }
                        }
                    }
                    crate::models::AudioTarget::Process => {
                        let p_name = process_name.as_deref().unwrap_or("");
                        let target_proc = Self::interpolate_tokens(p_name, context);
                        let target_pid = context.get("ProcessId").and_then(|p| p.parse::<u32>().ok());
                        #[cfg(windows)]
                        {
                            // Retry up to 15 times with 150ms delay (~2.2s total) in case audio session is not yet registered
                            let mut matched = false;
                            for attempt in 0..15 {
                                match set_process_audio(&target_proc, target_pid, *action_type, *volume_level) {
                                    Ok(found) => {
                                        if found {
                                            matched = true;
                                            break;
                                        }
                                    }
                                    Err(e) => return Err(e),
                                }
                                if attempt < 14 {
                                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                                }
                            }
                            if !matched && !target_proc.is_empty() && target_proc != "*" && target_proc != "current" {
                                eprintln!("No audio session found for process '{}'", target_proc);
                            }
                        }
                    }
                }
                Ok(())
            }

            Action::AudioFormat {
                target,
                device_name,
                sample_rate,
                bit_depth,
                channels,
            } => {
                let dev_name = device_name.as_ref().map(|d| Self::interpolate_tokens(d, context));
                #[cfg(windows)]
                {
                    set_audio_device_format(*target, *sample_rate, *bit_depth, *channels, dev_name.as_deref())?;
                }
                #[cfg(not(windows))]
                {
                    let _ = (target, dev_name, sample_rate, bit_depth, channels);
                }
                Ok(())
            }

            Action::DefaultDevice { target, device_name } => {
                let name = Self::interpolate_tokens(device_name, context);
                #[cfg(windows)]
                {
                    match target {
                        crate::models::AudioTarget::Playback => {
                            set_default_audio_device(&name)?;
                        }
                        crate::models::AudioTarget::Microphone => {
                            set_default_microphone_device(&name)?;
                        }
                        crate::models::AudioTarget::Process => {}
                    }
                }
                Ok(())
            }

            Action::ExecuteScript { interpreter, script, run_in_window } => {
                Self::run_script_internal(interpreter, script, *run_in_window, context)?;
                Ok(())
            }

            Action::Macro { events, repeat, speed, infinite } => {
                let rep = (*repeat).max(1);
                let mut count = 0;
                while *infinite || count < rep {
                    for ev in events {
                        execute_macro_event(ev, *speed, context).await?;
                    }
                    count += 1;
                }
                Ok(())
            }

            Action::IfElse { conditions, then_actions, else_if_branches, else_actions } => {
                let is_true = if !conditions.is_empty() {
                    crate::daemon::engine::DaemonEngine::evaluate_rule_conditions(conditions)
                } else {
                    true
                };
                if is_true {
                    for sub_action in then_actions {
                        Box::pin(Self::execute(sub_action, context)).await?;
                    }
                } else {
                    let mut matched = false;
                    for branch in else_if_branches {
                        if !branch.conditions.is_empty() && crate::daemon::engine::DaemonEngine::evaluate_rule_conditions(&branch.conditions) {
                            for sub_action in &branch.actions {
                                Box::pin(Self::execute(sub_action, context)).await?;
                            }
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        for sub_action in else_actions {
                            Box::pin(Self::execute(sub_action, context)).await?;
                        }
                    }
                }
                Ok(())
            }

            Action::TrimWorkingSet { target_process } => {
                let default_proc = String::new();
                let interp = target_process.as_ref().map(|p| Self::interpolate_tokens(p, context));
                let proc_name = interp.as_deref().or_else(|| context.get("ProcessName").map(|s| s.as_str())).unwrap_or(&default_proc);
                let target_pid = context.get("ProcessId").and_then(|p| p.parse::<u32>().ok());
                trim_working_set(proc_name, target_pid)?;
                Ok(())
            }

            Action::DiscordRpcSet {
                app_id,
                details,
                state,
                large_image,
                large_text,
                small_image,
                small_text,
                show_elapsed,
                buttons,
                ..
            } => {
                let app_id_str = Self::interpolate_tokens(app_id, context);
                let details_str = Self::interpolate_tokens(details, context);
                let state_str = Self::interpolate_tokens(state, context);
                let large_image_str = large_image.as_ref().map(|s| Self::interpolate_tokens(s, context));
                let large_text_str = large_text.as_ref().map(|s| Self::interpolate_tokens(s, context));
                let small_image_str = small_image.as_ref().map(|s| Self::interpolate_tokens(s, context));
                let small_text_str = small_text.as_ref().map(|s| Self::interpolate_tokens(s, context));
                let buttons_interp: Vec<crate::models::DiscordRpcButton> = buttons
                    .iter()
                    .map(|b| crate::models::DiscordRpcButton {
                        label: Self::interpolate_tokens(&b.label, context),
                        url: Self::interpolate_tokens(&b.url, context),
                    })
                    .collect();

                crate::daemon::discord_rpc::update_presence(
                    &app_id_str,
                    &details_str,
                    &state_str,
                    large_image_str.as_deref(),
                    large_text_str.as_deref(),
                    small_image_str.as_deref(),
                    small_text_str.as_deref(),
                    *show_elapsed,
                    &buttons_interp,
                ).await?;
                Ok(())
            }

            Action::DiscordRpcClear => {
                crate::daemon::discord_rpc::clear_presence().await?;
                Ok(())
            }

            Action::Archive {
                operation,
                format,
                source_path,
                destination_path,
                compression_level,
                overwrite,
            } => {
                let src = Self::interpolate_tokens(source_path, context);
                let dst = Self::interpolate_tokens(destination_path, context);
                execute_archive(*operation, *format, &src, &dst, *compression_level, *overwrite)?;
                Ok(())
            }

            Action::FileSystem {
                operation,
                path,
                target_path,
                content,
                create_parents,
                overwrite,
            } => {
                let p = Self::interpolate_tokens(path, context);
                let target = target_path.as_ref().map(|s| Self::interpolate_tokens(s, context));
                let body = content.as_ref().map(|s| Self::interpolate_tokens(s, context));
                execute_file_system(*operation, &p, target.as_deref(), body.as_deref(), *create_parents, *overwrite)?;
                Ok(())
            }
        }
    }

    pub fn interpolate_tokens(text: &str, context: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (k, v) in context {
            result = result.replace(&format!("{{{}}}", k), v);
        }
        if result.contains("{Clipboard}") {
            let cb = crate::sharing::read_text_from_clipboard().unwrap_or_else(|_| String::new());
            result = result.replace("{Clipboard}", &cb);
        }
        if result.contains("{Date}") {
            let dt = chrono::Local::now().format("%Y-%m-%d").to_string();
            result = result.replace("{Date}", &dt);
        }
        if result.contains("{Time}") {
            let tm = chrono::Local::now().format("%H:%M:%S").to_string();
            result = result.replace("{Time}", &tm);
        }
        while let Some(start) = result.find("{Env:") {
            if let Some(end) = result[start..].find('}') {
                let full_tag = &result[start..start + end + 1];
                let var_name = &full_tag[5..full_tag.len() - 1];
                let val = std::env::var(var_name).unwrap_or_default();
                result = result.replace(full_tag, &val);
            } else {
                break;
            }
        }
        if result.contains("{ProcessPath}") || result.contains("{ProcessDir}") {
            let proc_path = context.get("ProcessPath").cloned().or_else(|| {
                context.get("ProcessName").and_then(|name| crate::daemon::watcher::ProcessWatcher::find_exe_path_by_name(name))
            }).unwrap_or_default();
            let proc_dir = std::path::Path::new(&proc_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            result = result.replace("{ProcessPath}", &proc_path);
            result = result.replace("{ProcessDir}", &proc_dir);
        }
        if result.contains("{WindowTitle}") {
            let wt = context.get("WindowTitle").cloned().unwrap_or_default();
            result = result.replace("{WindowTitle}", &wt);
        }
        result
    }
}

#[cfg(windows)]
fn change_display_settings(
    width: Option<u32>,
    height: Option<u32>,
    refresh_rate: Option<u32>,
    monitor_index: Option<u32>,
) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_UPDATEREGISTRY,
        DEVMODEW, DISPLAY_DEVICEW, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT,
        DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
    };

    unsafe {
        let mut dm: DEVMODEW = std::mem::zeroed();
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

        let dev_name_buf: Option<Vec<u16>> = if let Some(m_idx) = monitor_index {
            if m_idx > 0 {
                let mut dd: DISPLAY_DEVICEW = std::mem::zeroed();
                dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                let target_idx = m_idx - 1;
                if EnumDisplayDevicesW(None, target_idx, &mut dd, 0).as_bool() {
                    Some(dd.DeviceName.to_vec())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let dev_ptr = dev_name_buf.as_ref().map(|v| PCWSTR(v.as_ptr())).unwrap_or(PCWSTR::null());

        if !EnumDisplaySettingsW(dev_ptr, ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
            return Err(format!("Failed to query current display settings for monitor {:?}", monitor_index));
        }

        if let (Some(w), Some(h)) = (width, height) {
            dm.dmPelsWidth = w;
            dm.dmPelsHeight = h;
            dm.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT;
        }

        if let Some(hz) = refresh_rate {
            dm.dmDisplayFrequency = hz;
            dm.dmFields |= DM_DISPLAYFREQUENCY;
        }

        let res = ChangeDisplaySettingsExW(dev_ptr, Some(&dm), None, CDS_UPDATEREGISTRY, None);
        if res == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(format!("ChangeDisplaySettings failed with status {:?}", res))
        }
    }
}

#[cfg(windows)]
fn set_default_audio_device(name: &str) -> Result<(), String> {
    let ps_cmd = format!(
        r#"$n = '{}';
        if (Get-Command Set-AudioDevice -ErrorAction SilentlyContinue) {{
            Get-AudioDevice -List | Where-Object {{ $_.Name -like "*$n*" }} | Set-AudioDevice
        }} else {{
            Write-Host "Set default audio: $n"
        }}"#,
        name.replace('\'', "''")
    );
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_cmd]);
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
    let _ = cmd.spawn().map_err(|e| format!("Failed to set audio device: {e}"))?;
    Ok(())
}

#[cfg(windows)]
fn set_default_microphone_device(name: &str) -> Result<(), String> {
    let ps_cmd = format!(
        r#"$n = '{}';
        if (Get-Command Set-AudioDevice -ErrorAction SilentlyContinue) {{
            Get-AudioDevice -List | Where-Object {{ ($_.Type -eq 'Recording' -or $_.Type -like '*Capture*' -or $_.Name -like "*$n*") -and $_.Name -like "*$n*" }} | Set-AudioDevice
        }} else {{
            Write-Host "Set default microphone: $n"
        }}"#,
        name.replace('\'', "''")
    );
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_cmd]);
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
    let _ = cmd.spawn().map_err(|e| format!("Failed to set microphone device: {e}"))?;
    Ok(())
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn window_control(
    target_process: &str,
    target_pid: Option<u32>,
    action: WindowActionType,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
    transparency: Option<u8>,
) -> Result<(), String> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, ShowWindowAsync, SWP_NOACTIVATE, GWL_STYLE, WS_CHILD,
    };

    let target_proc_clean = target_process.trim();
    let mut resolved_pids = Vec::new();
    if let Some(pid) = target_pid {
        if pid > 0 {
            resolved_pids.push(pid);
        }
    }
    if resolved_pids.is_empty() && !target_proc_clean.is_empty() {
        let snap = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
        let query_name = target_proc_clean.trim_end_matches(".exe").to_lowercase();
        for (pid, name) in snap {
            let proc_base = name.trim_end_matches(".exe").to_lowercase();
            if proc_base == query_name {
                resolved_pids.push(pid);
            }
        }
    }

    struct CandidateWindow {
        hwnd: HWND,
        area: i64,
        is_visible: bool,
    }

    unsafe extern "system" fn enum_wnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut (Vec<u32>, String, Vec<CandidateWindow>));
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        // Skip windows belonging to our own process
        if pid == std::process::id() {
            return BOOL(1);
        }

        // Must NOT be a child window (must be top-level)
        let style = GetWindowLongW(hwnd, GWL_STYLE);
        if (style & (WS_CHILD.0 as i32)) != 0 {
            return BOOL(1);
        }

        // Must NOT be an owned popup/dialog
        if let Ok(owner) = GetWindow(hwnd, GW_OWNER) {
            if !owner.0.is_null() {
                return BOOL(1);
            }
        }

        // Must NOT be a tool window
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if (ex & (WS_EX_TOOLWINDOW.0 as i32)) != 0 {
            return BOOL(1);
        }

        // Must have non-trivial dimensions
        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r).is_err() {
            return BOOL(1);
        }
        let w = (r.right - r.left).max(0);
        let h = (r.bottom - r.top).max(0);
        if w < 20 || h < 20 {
            return BOOL(1);
        }

        let is_vis = IsWindowVisible(hwnd).as_bool();
        let title_len = GetWindowTextLengthW(hwnd);

        // Filter out common Windows internal hidden framework/IME helper windows
        if title_len > 0 {
            let mut buf = [0u16; 128];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                if title.starts_with("MSCTFIME")
                    || title.starts_with("Default IME")
                    || title.starts_with("OLEChannelWnd")
                    || title.starts_with("GDI+ Window")
                {
                    return BOOL(1);
                }
            }
        }

        let mut matches = false;
        if !data.0.is_empty() {
            if data.0.contains(&pid) {
                matches = true;
            }
        } else if !data.1.is_empty() {
            if let Some(img) = crate::daemon::watcher::ProcessWatcher::get_process_image_path(pid) {
                let name = std::path::Path::new(&img)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("")
                    .trim_end_matches(".exe");
                if name.eq_ignore_ascii_case(data.1.trim_end_matches(".exe")) {
                    matches = true;
                }
            }
        }

        if matches {
            data.2.push(CandidateWindow {
                hwnd,
                area: (w as i64) * (h as i64),
                is_visible: is_vis,
            });
        }
        BOOL(1)
    }

    let mut query = (resolved_pids, target_proc_clean.to_string(), Vec::<CandidateWindow>::new());
    unsafe {
        let _ = EnumWindows(Some(enum_wnd), LPARAM(&mut query as *mut _ as isize));
    }

    let mut target_hwnds: Vec<HWND> = Vec::new();
    if !query.2.is_empty() {
        // Sort by visible first, then by area descending to target the primary main window
        query.2.sort_by(|a, b| {
            b.is_visible.cmp(&a.is_visible).then_with(|| b.area.cmp(&a.area))
        });
        // Select the primary main application window
        if let Some(main_cand) = query.2.first() {
            target_hwnds.push(main_cand.hwnd);
        }
    } else if target_proc_clean.is_empty() && target_pid.is_none() {
        unsafe {
            let fg = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
            if !fg.0.is_null() {
                let mut fg_pid = 0u32;
                GetWindowThreadProcessId(fg, Some(&mut fg_pid));
                if fg_pid != std::process::id() {
                    target_hwnds.push(fg);
                }
            }
        }
    }

    if target_hwnds.is_empty() {
        return Err(format!("No eligible window found for target '{}'", target_process));
    }

    for hwnd in target_hwnds {
        unsafe {
            match action {
                WindowActionType::Minimize => {
                    let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
                }
                WindowActionType::Maximize => {
                    let _ = ShowWindowAsync(hwnd, SW_MAXIMIZE);
                }
                WindowActionType::Restore => {
                    let _ = ShowWindowAsync(hwnd, SW_RESTORE);
                }
                WindowActionType::Hide => {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                WindowActionType::Show => {
                    let _ = ShowWindowAsync(hwnd, SW_SHOW);
                }
                WindowActionType::Close => {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                WindowActionType::ToggleAlwaysOnTop => {
                    let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
                    let is_topmost = (ex & (WS_EX_TOPMOST.0 as i32)) != 0;
                    let flag = if is_topmost { HWND_NOTOPMOST } else { HWND_TOPMOST };
                    let _ = SetWindowPos(hwnd, Some(flag), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
                WindowActionType::AlwaysOnTop => {
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
                WindowActionType::RemoveAlwaysOnTop => {
                    let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
                WindowActionType::SetTransparency => {
                    let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
                    SetWindowLongW(hwnd, GWL_EXSTYLE, ex | (WS_EX_LAYERED.0 as i32));
                    let alpha = transparency.unwrap_or(255);
                    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
                }
                WindowActionType::SnapToPosition => {
                    let mut current_rect = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut current_rect);
                    let cur_w = (current_rect.right - current_rect.left).max(100);
                    let cur_h = (current_rect.bottom - current_rect.top).max(100);
                    let nx = x.unwrap_or(current_rect.left);
                    let ny = y.unwrap_or(current_rect.top);
                    let nw = width.unwrap_or(cur_w);
                    let nh = height.unwrap_or(cur_h);
                    let _ = SetWindowPos(hwnd, None, nx, ny, nw, nh, SWP_NOZORDER | SWP_NOACTIVATE);
                }
            }
        }
    }

    Ok(())
}

#[cfg(windows)]
fn tune_process(
    target_process: &str,
    target_pid: Option<u32>,
    priority: Option<ProcessPriorityLevel>,
    affinity_mask: Option<u64>,
    trim_memory: bool,
) -> Result<(), String> {
    let mut pids: Vec<u32> = Vec::new();
    if let Some(pid) = target_pid {
        if pid > 0 {
            pids.push(pid);
        }
    }
    if pids.is_empty() && !target_process.is_empty() {
        let snapshot = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
        let target_lower = target_process.to_lowercase();
        for (pid, name) in snapshot {
            if name == target_lower {
                pids.push(pid);
            }
        }
    }

    if pids.is_empty() {
        return Err(format!("Process '{}' not found for tuning", target_process));
    }

    for pid in pids {
        unsafe {
            let h_proc = OpenProcess(
                PROCESS_SET_INFORMATION | PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                pid,
            );
            if let Ok(handle) = h_proc {
                if let Some(prio) = priority {
                    let class = match prio {
                        ProcessPriorityLevel::Realtime => REALTIME_PRIORITY_CLASS,
                        ProcessPriorityLevel::High => HIGH_PRIORITY_CLASS,
                        ProcessPriorityLevel::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
                        ProcessPriorityLevel::Normal => NORMAL_PRIORITY_CLASS,
                        ProcessPriorityLevel::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
                        ProcessPriorityLevel::Idle => IDLE_PRIORITY_CLASS,
                    };
                    let _ = SetPriorityClass(handle, class);
                }
                if let Some(mask) = affinity_mask {
                    let _ = SetProcessAffinityMask(handle, mask as usize);
                }
                if trim_memory {
                    let _ = EmptyWorkingSet(handle);
                }
                let _ = CloseHandle(handle);
            }
        }
    }
    Ok(())
}

pub fn trim_working_set(target_process: &str, target_pid: Option<u32>) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_QUERY_LIMITED_INFORMATION};

        let mut pids: Vec<u32> = Vec::new();
        if let Some(pid) = target_pid {
            if pid > 0 {
                pids.push(pid);
            }
        }
        if pids.is_empty() && !target_process.is_empty() {
            let snapshot = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
            let target_lower = target_process.to_lowercase();
            let target_has_exe = target_lower.ends_with(".exe");
            for (pid, name) in snapshot {
                if name == target_lower || (!target_has_exe && name == format!("{target_lower}.exe")) {
                    pids.push(pid);
                }
            }
        }
        if pids.is_empty() {
            return Err(format!("Process '{}' not found for trimming working set", target_process));
        }

        for pid in pids {
            unsafe {
                if let Ok(handle) = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                    let _ = EmptyWorkingSet(handle);
                    let _ = CloseHandle(handle);
                }
            }
        }
    }
    Ok(())
}

pub fn execute_archive(
    operation: crate::models::ArchiveOperation,
    format: crate::models::ArchiveFormat,
    source: &str,
    destination: &str,
    _compression_level: Option<u8>,
    overwrite: bool,
) -> Result<(), String> {
    use std::fs::File;
    use std::path::Path;
    use crate::models::{ArchiveFormat, ArchiveOperation};

    let src_path = Path::new(source);
    let dst_path = Path::new(destination);

    match operation {
        ArchiveOperation::Pack => {
            if !src_path.exists() {
                return Err(format!("Archive source does not exist: {source}"));
            }
            if dst_path.exists() && !overwrite {
                return Err(format!("Archive destination already exists and overwrite is disabled: {destination}"));
            }
            if let Some(parent) = dst_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            match format {
                ArchiveFormat::Zip => {
                    let file = File::create(dst_path).map_err(|e| format!("Failed to create zip file: {e}"))?;
                    let mut zip = zip::ZipWriter::new(file);
                    let options = zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated);

                    if src_path.is_file() {
                        let file_name = src_path.file_name().and_then(|f| f.to_str()).unwrap_or("file");
                        zip.start_file(file_name, options).map_err(|e| e.to_string())?;
                        let mut f = File::open(src_path).map_err(|e| e.to_string())?;
                        std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
                    } else if src_path.is_dir() {
                        fn add_dir(dir: &Path, prefix: &Path, zip: &mut zip::ZipWriter<File>, options: zip::write::SimpleFileOptions) -> Result<(), String> {
                            for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
                                let entry = entry.map_err(|e| e.to_string())?;
                                let path = entry.path();
                                let rel = path.strip_prefix(prefix).map_err(|e| e.to_string())?;
                                let rel_str = rel.to_string_lossy().replace('\\', "/");
                                if path.is_dir() {
                                    zip.add_directory(&rel_str, options).map_err(|e| e.to_string())?;
                                    add_dir(&path, prefix, zip, options)?;
                                } else if path.is_file() {
                                    zip.start_file(&rel_str, options).map_err(|e| e.to_string())?;
                                    let mut f = File::open(&path).map_err(|e| e.to_string())?;
                                    std::io::copy(&mut f, zip).map_err(|e| e.to_string())?;
                                }
                            }
                            Ok(())
                        }
                        add_dir(src_path, src_path, &mut zip, options)?;
                    }
                    zip.finish().map_err(|e| format!("Failed to finalize zip archive: {e}"))?;
                    Ok(())
                }
                ArchiveFormat::Tar | ArchiveFormat::TarGz | ArchiveFormat::TarXz | ArchiveFormat::SevenZip => {
                    let src_parent = src_path.parent().unwrap_or(Path::new("."));
                    let src_name = src_path.file_name().unwrap_or_default();
                    let mut cmd = std::process::Command::new("tar.exe");
                    cmd.args(["-a", "-cf", destination, "-C", &src_parent.to_string_lossy(), &src_name.to_string_lossy()]);
                    #[cfg(windows)]
                    {
                        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(CREATE_NO_WINDOW.0);
                    }
                    let status = cmd.status().map_err(|e| format!("Failed to run tar.exe: {e}"))?;
                    if !status.success() {
                        return Err(format!("tar.exe archive creation returned exit code {:?}", status.code()));
                    }
                    Ok(())
                }
                ArchiveFormat::Rar => {
                    let rar_paths = [
                        "rar.exe",
                        r"C:\Program Files\WinRAR\rar.exe",
                        r"C:\Program Files (x86)\WinRAR\rar.exe",
                    ];
                    let mut found_cmd = None;
                    for p in rar_paths {
                        if std::process::Command::new(p).arg("-?").output().is_ok() {
                            found_cmd = Some(p);
                            break;
                        }
                    }
                    if let Some(cmd_path) = found_cmd {
                        let mut cmd = std::process::Command::new(cmd_path);
                        cmd.args(["a", "-ep1", "-r", destination, source]);
                        #[cfg(windows)]
                        {
                            use windows::Win32::System::Threading::CREATE_NO_WINDOW;
                            use std::os::windows::process::CommandExt;
                            cmd.creation_flags(CREATE_NO_WINDOW.0);
                        }
                        let status = cmd.status().map_err(|e| format!("Failed to run rar.exe: {e}"))?;
                        if !status.success() {
                            return Err(format!("rar.exe returned exit code {:?}", status.code()));
                        }
                        Ok(())
                    } else {
                        Err("Creating .rar archives requires WinRAR (rar.exe) in PATH or Program Files. Tip: Use Zip or 7z for native compression.".to_string())
                    }
                }
            }
        }
        ArchiveOperation::Unpack => {
            if !src_path.exists() {
                return Err(format!("Archive source file does not exist: {source}"));
            }
            std::fs::create_dir_all(dst_path).map_err(|e| format!("Failed to create destination folder: {e}"))?;

            match format {
                ArchiveFormat::Zip => {
                    let file = File::open(src_path).map_err(|e| format!("Failed to open zip file: {e}"))?;
                    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {e}"))?;
                    for i in 0..archive.len() {
                        let mut item = archive.by_index(i).map_err(|e| e.to_string())?;
                        let outpath = match item.enclosed_name() {
                            Some(p) => dst_path.join(p),
                            None => continue,
                        };
                        if item.is_dir() {
                            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
                        } else {
                            if let Some(p) = outpath.parent() {
                                if !p.exists() {
                                    std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                                }
                            }
                            if outpath.exists() && !overwrite {
                                continue;
                            }
                            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
                            std::io::copy(&mut item, &mut outfile).map_err(|e| e.to_string())?;
                        }
                    }
                    Ok(())
                }
                ArchiveFormat::Tar | ArchiveFormat::TarGz | ArchiveFormat::TarXz | ArchiveFormat::SevenZip | ArchiveFormat::Rar => {
                    let mut cmd = std::process::Command::new("tar.exe");
                    cmd.args(["-xf", source, "-C", destination]);
                    #[cfg(windows)]
                    {
                        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(CREATE_NO_WINDOW.0);
                    }
                    match cmd.status() {
                        Ok(status) if status.success() => Ok(()),
                        _ => {
                            if format == ArchiveFormat::Rar {
                                let mut unrar = std::process::Command::new("unrar.exe");
                                unrar.args(["x", "-o+", source, destination]);
                                #[cfg(windows)]
                                {
                                    use windows::Win32::System::Threading::CREATE_NO_WINDOW;
                                    use std::os::windows::process::CommandExt;
                                    unrar.creation_flags(CREATE_NO_WINDOW.0);
                                }
                                if let Ok(s) = unrar.status() {
                                    if s.success() {
                                        return Ok(());
                                    }
                                }
                            }
                            Err(format!("Failed to unpack {:?} archive using tar.exe or system extractors", format))
                        }
                    }
                }
            }
        }
    }
}

pub fn execute_file_system(
    operation: crate::models::FileSystemOperation,
    path: &str,
    target_path: Option<&str>,
    content: Option<&str>,
    create_parents: bool,
    overwrite: bool,
) -> Result<(), String> {
    use std::path::Path;
    use crate::models::FileSystemOperation;

    let src = Path::new(path);

    match operation {
        FileSystemOperation::CreateFile => {
            if let Some(parent) = src.parent() {
                if create_parents && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directories: {e}"))?;
                }
            }
            if src.exists() && !overwrite {
                return Ok(());
            }
            std::fs::File::create(src).map_err(|e| format!("Failed to create file {:?}: {e}", src))?;
        }
        FileSystemOperation::CreateDirectory => {
            std::fs::create_dir_all(src).map_err(|e| format!("Failed to create directory {:?}: {e}", src))?;
        }
        FileSystemOperation::Rename => {
            let target_str = target_path.ok_or_else(|| "Target path must be specified for Rename".to_string())?;
            let dst = Path::new(target_str);
            if let Some(parent) = dst.parent() {
                if create_parents && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directories: {e}"))?;
                }
            }
            std::fs::rename(src, dst).map_err(|e| format!("Failed to rename {:?} to {:?}: {e}", src, dst))?;
        }
        FileSystemOperation::Copy => {
            let target_str = target_path.ok_or_else(|| "Target path must be specified for Copy/Clone".to_string())?;
            let dst = Path::new(target_str);
            if src.is_file() {
                if dst.exists() && !overwrite {
                    return Ok(());
                }
                if let Some(parent) = dst.parent() {
                    if create_parents && !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directories: {e}"))?;
                    }
                }
                std::fs::copy(src, dst).map_err(|e| format!("Failed to copy file {:?} to {:?}: {e}", src, dst))?;
            } else if src.is_dir() {
                fn copy_dir_all(src: &Path, dst: &Path, overwrite: bool) -> Result<(), String> {
                    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
                    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
                        let entry = entry.map_err(|e| e.to_string())?;
                        let entry_path = entry.path();
                        let target_entry = dst.join(entry.file_name());
                        if entry_path.is_dir() {
                            copy_dir_all(&entry_path, &target_entry, overwrite)?;
                        } else {
                            if target_entry.exists() && !overwrite {
                                continue;
                            }
                            std::fs::copy(&entry_path, &target_entry).map_err(|e| e.to_string())?;
                        }
                    }
                    Ok(())
                }
                copy_dir_all(src, dst, overwrite)?;
            } else {
                return Err(format!("Source path does not exist: {:?}", src));
            }
        }
        FileSystemOperation::Delete => {
            if src.exists() {
                if src.is_file() {
                    std::fs::remove_file(src).map_err(|e| format!("Failed to delete file {:?}: {e}", src))?;
                } else if src.is_dir() {
                    std::fs::remove_dir_all(src).map_err(|e| format!("Failed to delete directory {:?}: {e}", src))?;
                }
            }
        }
        FileSystemOperation::WriteText => {
            if src.exists() && !overwrite {
                return Ok(());
            }
            if let Some(parent) = src.parent() {
                if create_parents && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directories: {e}"))?;
                }
            }
            let data = content.unwrap_or_default();
            std::fs::write(src, data).map_err(|e| format!("Failed to write text to {:?}: {e}", src))?;
        }
        FileSystemOperation::AppendText => {
            use std::io::Write;
            if let Some(parent) = src.parent() {
                if create_parents && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directories: {e}"))?;
                }
            }
            let data = content.unwrap_or_default();
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(src)
                .map_err(|e| format!("Failed to open file for append {:?}: {e}", src))?;
            file.write_all(data.as_bytes()).map_err(|e| format!("Failed to append to {:?}: {e}", src))?;
        }
    }
    Ok(())
}

fn random_range(min: u32, max: u32) -> u32 {
    if min >= max {
        return min;
    }
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x853c49e6748fea9b);
    let mut s = SEED.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    s ^= s >> 30;
    s = s.wrapping_mul(0xbf58476d1ce4e5b9);
    s ^= s >> 27;
    s = s.wrapping_mul(0x94d049bb133111eb);
    s ^= s >> 31;
    let range = (max - min + 1) as u64;
    min + (s % range) as u32
}

pub(crate) fn random_gaussian(mean: f64, std_dev: f64) -> f64 {
    let u1 = ((random_range(1, 10000) as f64) / 10000.0).max(1e-6);
    let u2 = ((random_range(1, 10000) as f64) / 10000.0).max(1e-6);
    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    mean + z0 * std_dev
}

pub(crate) fn calculate_sleep_ms(base_ms: u32, organic: bool, jitter_percent: u32, ch_opt: Option<char>) -> u64 {
    if !organic {
        return base_ms as u64;
    }
    let base = if base_ms == 0 { 25 } else { base_ms } as f64;
    let jitter = (jitter_percent.clamp(5, 95) as f64) / 100.0;
    let std_dev = base * (jitter / 2.3);
    let sample = random_gaussian(base, std_dev);
    let mut chosen = sample.round().max(5.0) as u64;

    // Rare biological hesitation / micro-pause: ~1.2% chance of 120-260ms delay
    if random_range(1, 1000) <= 12 {
        chosen += random_range(120, 260) as u64;
    }

    if let Some(ch) = ch_opt {
        if ch == ' ' || ch == ',' || ch == '.' || ch == '!' || ch == '?' || ch == '\n' {
            let pause = random_gaussian(45.0, 15.0).round().max(10.0) as u64;
            chosen += pause;
        }
    }
    chosen
}

#[cfg(windows)]
fn send_mouse_input(flags: MOUSE_EVENT_FLAGS, mouse_data: u32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: mouse_data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
fn smooth_mouse_move(
    target_x: i32,
    target_y: i32,
    duration_ms: u32,
    organic: bool,
) {
    let mut start_pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut start_pt);
    }
    let sx = start_pt.x as f64;
    let sy = start_pt.y as f64;
    let tx = target_x as f64;
    let ty = target_y as f64;

    let total_dist = (tx - sx).hypot(ty - sy).max(1.0);
    if total_dist < 2.0 {
        unsafe {
            let _ = SetCursorPos(target_x, target_y);
        }
        return;
    }

    let base_dur = duration_ms.max(20) as f64;
    let total_duration = if organic {
        let factor = 1.0 + (random_range(0, 30) as f64 - 15.0) / 100.0;
        base_dur * factor
    } else {
        base_dur
    };

    let step_interval_ms = 8.0;
    let steps = (total_duration / step_interval_ms).round().max(4.0) as usize;

    let (p1x, p1y, p2x, p2y) = if organic {
        let vx = tx - sx;
        let vy = ty - sy;
        let nx = -vy / total_dist;
        let ny = vx / total_dist;
        let sign = if random_range(0, 1) == 0 { 1.0 } else { -1.0 };
        let dev = total_dist * (random_range(10, 25) as f64 / 100.0) * sign;

        let p1x = sx + vx * 0.3 + nx * dev;
        let p1y = sy + vy * 0.3 + ny * dev;
        let p2x = sx + vx * 0.7 + nx * (dev * 0.7);
        let p2y = sy + vy * 0.7 + ny * (dev * 0.7);
        (p1x, p1y, p2x, p2y)
    } else {
        (sx, sy, tx, ty)
    };

    for i in 1..=steps {
        let raw_t = i as f64 / steps as f64;
        let t = if raw_t < 0.5 {
            2.0 * raw_t * raw_t
        } else {
            1.0 - (-2.0 * raw_t + 2.0).powi(2) / 2.0
        };

        let (cur_x, cur_y) = if organic {
            let u = 1.0 - t;
            let tt = t * t;
            let uu = u * u;
            let uuu = uu * u;
            let ttt = tt * t;

            let bx = uuu * sx + 3.0 * uu * t * p1x + 3.0 * u * tt * p2x + ttt * tx;
            let by = uuu * sy + 3.0 * uu * t * p1y + 3.0 * u * tt * p2y + ttt * ty;

            if i < steps.saturating_sub(2) {
                let jx = (random_range(0, 2) as i32 - 1) as f64;
                let jy = (random_range(0, 2) as i32 - 1) as f64;
                (bx + jx, by + jy)
            } else {
                (bx, by)
            }
        } else {
            let lx = sx + (tx - sx) * t;
            let ly = sy + (ty - sy) * t;
            (lx, ly)
        };

        unsafe {
            let _ = SetCursorPos(cur_x.round() as i32, cur_y.round() as i32);
        }
        std::thread::sleep(std::time::Duration::from_millis(step_interval_ms as u64));
    }

    unsafe {
        let _ = SetCursorPos(target_x, target_y);
    }
}

#[cfg(windows)]
fn send_mouse_relative_step(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
fn smooth_relative_mouse_move(
    total_dx: i32,
    total_dy: i32,
    duration_ms: u32,
    organic: bool,
) {
    if total_dx == 0 && total_dy == 0 {
        return;
    }

    let dur_ms = duration_ms.max(10);
    // For very brief or tiny movements, send immediately without slicing
    if dur_ms < 18 || (total_dx.abs() < 3 && total_dy.abs() < 3) {
        send_mouse_relative_step(total_dx, total_dy);
        return;
    }

    let base_dur = dur_ms as f64;
    let total_duration_ms = if organic {
        let factor = 1.0 + (random_range(0, 20) as f64 - 10.0) / 100.0;
        base_dur * factor
    } else {
        base_dur
    };
    let total_secs = total_duration_ms / 1000.0;

    let start = std::time::Instant::now();
    let mut sent_x = 0i32;
    let mut sent_y = 0i32;

    loop {
        let elapsed = start.elapsed().as_secs_f64();
        let t = (elapsed / total_secs).min(1.0);

        let progress = if organic {
            // Smoothstep curve for human-like organic motion
            t * t * (3.0 - 2.0 * t)
        } else {
            // Linear velocity curve - exactly matches in-game mouse sensor dynamics
            t
        };

        let target_x = (total_dx as f64 * progress).round() as i32;
        let target_y = (total_dy as f64 * progress).round() as i32;

        let step_dx = target_x - sent_x;
        let step_dy = target_y - sent_y;

        if step_dx != 0 || step_dy != 0 {
            send_mouse_relative_step(step_dx, step_dy);
            sent_x += step_dx;
            sent_y += step_dy;
        }

        if t >= 1.0 {
            break;
        }

        let remaining_ms = (total_secs - elapsed) * 1000.0;
        if remaining_ms > 5.0 {
            std::thread::sleep(std::time::Duration::from_millis(3));
        } else {
            std::thread::yield_now();
        }
    }

    // Deliver any remaining integer ticks to guarantee 0 cumulative drift
    let rem_x = total_dx - sent_x;
    let rem_y = total_dy - sent_y;
    if rem_x != 0 || rem_y != 0 {
        send_mouse_relative_step(rem_x, rem_y);
    }
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn mouse_control(
    action_type: crate::models::MouseActionType,
    button: crate::models::MouseButton,
    coord_mode: crate::models::MouseCoordMode,
    x: Option<i32>,
    y: Option<i32>,
    smooth_move: bool,
    move_duration_ms: Option<u32>,
    organic_move: bool,
    scroll_delta: Option<i32>,
    clicks: Option<u32>,
    click_delay_ms: Option<u32>,
    organic_click_delay: bool,
) -> Result<(), String> {
    unsafe {
        match coord_mode {
            crate::models::MouseCoordMode::Absolute => {
                let target_x = x.unwrap_or(0);
                let target_y = y.unwrap_or(0);
                if smooth_move {
                    smooth_mouse_move(target_x, target_y, move_duration_ms.unwrap_or(250), organic_move);
                } else {
                    let _ = SetCursorPos(target_x, target_y);
                    let sleep_d = move_duration_ms.unwrap_or(16);
                    if sleep_d > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(sleep_d as u64));
                    }
                }
            }
            crate::models::MouseCoordMode::Relative => {
                let dx = x.unwrap_or(0);
                let dy = y.unwrap_or(0);
                if smooth_move {
                    smooth_relative_mouse_move(dx, dy, move_duration_ms.unwrap_or(200), organic_move);
                } else {
                    send_mouse_relative_step(dx, dy);
                    let sleep_d = move_duration_ms.unwrap_or(16);
                    if sleep_d > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(sleep_d as u64));
                    }
                }
            }
            crate::models::MouseCoordMode::Current => {}
        }

        let (down_flag, up_flag, mouse_data) = match button {
            crate::models::MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
            crate::models::MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
            crate::models::MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
            crate::models::MouseButton::XButton1 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 1),
            crate::models::MouseButton::XButton2 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 2),
        };

        let base_delay = click_delay_ms.unwrap_or(35);

        match action_type {
            crate::models::MouseActionType::Move => {
                // Movement handled above
            }
            crate::models::MouseActionType::Click => {
                let count = clicks.unwrap_or(1).clamp(1, 100);
                for i in 0..count {
                    if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                        lock.insert(button);
                    }
                    send_mouse_input(down_flag, mouse_data);
                    let hold_ms = calculate_sleep_ms(base_delay.clamp(10, 500), organic_click_delay, 30, None);
                    std::thread::sleep(std::time::Duration::from_millis(hold_ms));
                    send_mouse_input(up_flag, mouse_data);
                    if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                        lock.remove(&button);
                    }

                    if i + 1 < count {
                        let interval_ms = calculate_sleep_ms((base_delay * 2).clamp(20, 2000), organic_click_delay, 35, None);
                        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
                    }
                }
            }
            crate::models::MouseActionType::DoubleClick => {
                if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                    lock.insert(button);
                }
                send_mouse_input(down_flag, mouse_data);
                let hold1 = calculate_sleep_ms(base_delay.clamp(10, 300), organic_click_delay, 30, None);
                std::thread::sleep(std::time::Duration::from_millis(hold1));
                send_mouse_input(up_flag, mouse_data);
                if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                    lock.remove(&button);
                }

                let pause = calculate_sleep_ms((base_delay * 2).clamp(30, 400), organic_click_delay, 30, None);
                std::thread::sleep(std::time::Duration::from_millis(pause));

                if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                    lock.insert(button);
                }
                send_mouse_input(down_flag, mouse_data);
                let hold2 = calculate_sleep_ms(base_delay.clamp(10, 300), organic_click_delay, 30, None);
                std::thread::sleep(std::time::Duration::from_millis(hold2));
                send_mouse_input(up_flag, mouse_data);
                if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                    lock.remove(&button);
                }
            }
            crate::models::MouseActionType::Down => {
                if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                    lock.insert(button);
                }
                send_mouse_input(down_flag, mouse_data);
            }
            crate::models::MouseActionType::Up => {
                if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                    lock.remove(&button);
                }
                send_mouse_input(up_flag, mouse_data);
            }
            crate::models::MouseActionType::Scroll => {
                let delta = scroll_delta.unwrap_or(120);
                send_mouse_input(MOUSEEVENTF_WHEEL, delta as u32);
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn simulate_text_typing(text: &str, delay_ms: u32, organic: bool, jitter_percent: u32) -> Result<(), String> {
    std::thread::sleep(std::time::Duration::from_millis(80));
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.starts_with('{') {
            if let Some(end_brace) = remaining.find('}') {
                let token = remaining[1..end_brace].trim();
                let lower = token.to_lowercase();
                if let Some(stripped) = lower.strip_prefix("delay ") {
                    if let Ok(ms) = stripped.trim().parse::<u64>() {
                        std::thread::sleep(std::time::Duration::from_millis(ms));
                    }
                    remaining = &remaining[end_brace + 1..];
                    continue;
                }
            }
        }

        let mut chars = remaining.chars();
        let ch = chars.next().unwrap();
        remaining = chars.as_str();

        if ch == '\r' {
            continue;
        }
        if ch == '\n' {
            send_key_event(VK_RETURN, false);
            std::thread::sleep(std::time::Duration::from_millis(35));
            send_key_event(VK_RETURN, true);
        } else {
            send_char_event(ch);
        }
        let sleep = calculate_sleep_ms(delay_ms, organic, jitter_percent, Some(ch));
        if sleep > 0 {
            std::thread::sleep(std::time::Duration::from_millis(sleep));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn parse_virtual_key(s: &str) -> Option<VIRTUAL_KEY> {
    let clean = s.trim().trim_matches(|c| c == '{' || c == '}');
    let upper = clean.to_uppercase();
    match upper.as_str() {
        "CTRL" | "CONTROL" | "LCTRL" | "RCTRL" => Some(VK_CONTROL),
        "SHIFT" | "LSHIFT" | "RSHIFT" => Some(VK_SHIFT),
        "ALT" | "MENU" | "LALT" | "RALT" => Some(VK_MENU),
        "WIN" | "LWIN" | "RWIN" | "WINDOWS" => Some(VK_LWIN),
        "ENTER" | "RETURN" => Some(VK_RETURN),
        "ESC" | "ESCAPE" => Some(VK_ESCAPE),
        "TAB" => Some(VK_TAB),
        "SPACE" | "SPACEBAR" => Some(VK_SPACE),
        "BACK" | "BACKSPACE" => Some(VK_BACK),
        "DEL" | "DELETE" => Some(VK_DELETE),
        "INS" | "INSERT" => Some(VK_INSERT),
        "HOME" => Some(VK_HOME),
        "END" => Some(VK_END),
        "PAGEUP" | "PGUP" | "PRIOR" => Some(VK_PRIOR),
        "PAGEDOWN" | "PGDN" | "NEXT" => Some(VK_NEXT),
        "UP" => Some(VK_UP),
        "DOWN" => Some(VK_DOWN),
        "LEFT" => Some(VK_LEFT),
        "RIGHT" => Some(VK_RIGHT),
        "CAPS" | "CAPSLOCK" => Some(VK_CAPITAL),
        "PRTSC" | "PRINTSCREEN" => Some(VK_SNAPSHOT),
        "F1" => Some(VK_F1),
        "F2" => Some(VK_F2),
        "F3" => Some(VK_F3),
        "F4" => Some(VK_F4),
        "F5" => Some(VK_F5),
        "F6" => Some(VK_F6),
        "F7" => Some(VK_F7),
        "F8" => Some(VK_F8),
        "F9" => Some(VK_F9),
        "F10" => Some(VK_F10),
        "F11" => Some(VK_F11),
        "F12" => Some(VK_F12),
        // Numpad
        "NUMPAD0" | "NUM0" => Some(VIRTUAL_KEY(0x60)),
        "NUMPAD1" | "NUM1" => Some(VIRTUAL_KEY(0x61)),
        "NUMPAD2" | "NUM2" => Some(VIRTUAL_KEY(0x62)),
        "NUMPAD3" | "NUM3" => Some(VIRTUAL_KEY(0x63)),
        "NUMPAD4" | "NUM4" => Some(VIRTUAL_KEY(0x64)),
        "NUMPAD5" | "NUM5" => Some(VIRTUAL_KEY(0x65)),
        "NUMPAD6" | "NUM6" => Some(VIRTUAL_KEY(0x66)),
        "NUMPAD7" | "NUM7" => Some(VIRTUAL_KEY(0x67)),
        "NUMPAD8" | "NUM8" => Some(VIRTUAL_KEY(0x68)),
        "NUMPAD9" | "NUM9" => Some(VIRTUAL_KEY(0x69)),
        "MULTIPLY" | "NUMPADMULTIPLY" => Some(VIRTUAL_KEY(0x6A)),
        "ADD" | "NUMPADADD" => Some(VIRTUAL_KEY(0x6B)),
        "SUBTRACT" | "NUMPADSUBTRACT" => Some(VIRTUAL_KEY(0x6D)),
        "DECIMAL" | "NUMPADDECIMAL" => Some(VIRTUAL_KEY(0x6E)),
        "DIVIDE" | "NUMPADDIVIDE" => Some(VIRTUAL_KEY(0x6F)),
        // Punctuation and symbols
        ";" | "SEMICOLON" => Some(VIRTUAL_KEY(0xBA)),
        "=" | "PLUS" | "EQUAL" | "EQUALS" => Some(VIRTUAL_KEY(0xBB)),
        "," | "COMMA" => Some(VIRTUAL_KEY(0xBC)),
        "-" | "MINUS" | "DASH" => Some(VIRTUAL_KEY(0xBD)),
        "." | "PERIOD" | "DOT" => Some(VIRTUAL_KEY(0xBE)),
        "/" | "SLASH" => Some(VIRTUAL_KEY(0xBF)),
        "`" | "~" | "TILDE" | "BACKTICK" => Some(VIRTUAL_KEY(0xC0)),
        "[" | "{" | "LBRACKET" => Some(VIRTUAL_KEY(0xDB)),
        "\\" | "|" | "BACKSLASH" => Some(VIRTUAL_KEY(0xDC)),
        "]" | "}" | "RBRACKET" => Some(VIRTUAL_KEY(0xDD)),
        "'" | "\"" | "QUOTE" => Some(VIRTUAL_KEY(0xDE)),
        s if s.to_uppercase().starts_with("VK_0X") => {
            let hex = &s[5..];
            u16::from_str_radix(hex, 16).ok().map(VIRTUAL_KEY)
        }
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            if ch.is_ascii_alphanumeric() {
                Some(VIRTUAL_KEY(ch.to_ascii_uppercase() as u16))
            } else {
                match ch {
                    ';' => Some(VIRTUAL_KEY(0xBA)),
                    '=' => Some(VIRTUAL_KEY(0xBB)),
                    ',' => Some(VIRTUAL_KEY(0xBC)),
                    '-' => Some(VIRTUAL_KEY(0xBD)),
                    '.' => Some(VIRTUAL_KEY(0xBE)),
                    '/' => Some(VIRTUAL_KEY(0xBF)),
                    '`' => Some(VIRTUAL_KEY(0xC0)),
                    '[' => Some(VIRTUAL_KEY(0xDB)),
                    '\\' => Some(VIRTUAL_KEY(0xDC)),
                    ']' => Some(VIRTUAL_KEY(0xDD)),
                    '\'' => Some(VIRTUAL_KEY(0xDE)),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}

#[cfg(windows)]
static ACTIVE_HELD_KEYS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<u16>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

#[cfg(windows)]
static ACTIVE_HELD_MOUSE_BUTTONS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<crate::models::MouseButton>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

#[cfg(windows)]
fn send_key_event_raw(vk: VIRTUAL_KEY, is_up: bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, MAPVK_VK_TO_VSC, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
        VK_DIVIDE, VK_LWIN, VK_RCONTROL, VK_RMENU, VK_RWIN,
    };
    let scan = unsafe { MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC) } as u16;
    let mut flags = windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0);
    if is_up {
        flags |= KEYEVENTF_KEYUP;
    }
    if scan != 0 {
        flags |= KEYEVENTF_SCANCODE;
    }
    match vk {
        VK_UP | VK_DOWN | VK_LEFT | VK_RIGHT | VK_INSERT | VK_DELETE | VK_HOME | VK_END
        | VK_PRIOR | VK_NEXT | VK_SNAPSHOT | VK_DIVIDE | VK_RCONTROL | VK_RMENU | VK_LWIN | VK_RWIN => {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        _ => {}
    }
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
fn send_key_event(vk: VIRTUAL_KEY, is_up: bool) {
    if let Ok(mut lock) = ACTIVE_HELD_KEYS.lock() {
        if is_up {
            lock.remove(&vk.0);
        } else {
            lock.insert(vk.0);
        }
    }
    send_key_event_raw(vk, is_up);
}

/// Immediately and physically releases all keyboard keys and mouse buttons that were simulated
/// in the down state by macros, ensuring game characters and inputs stop immediately upon cancel.
pub fn release_all_simulated_inputs() {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            VIRTUAL_KEY, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_XUP,
        };

        let keys_to_release: Vec<u16> = {
            if let Ok(mut lock) = ACTIVE_HELD_KEYS.lock() {
                lock.drain().collect()
            } else {
                Vec::new()
            }
        };

        for vk_code in keys_to_release {
            send_key_event_raw(VIRTUAL_KEY(vk_code), true);
        }

        let buttons_to_release: Vec<crate::models::MouseButton> = {
            if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                lock.drain().collect()
            } else {
                Vec::new()
            }
        };

        for btn in buttons_to_release {
            let (up_flag, mouse_data) = match btn {
                crate::models::MouseButton::Left => (MOUSEEVENTF_LEFTUP, 0),
                crate::models::MouseButton::Right => (MOUSEEVENTF_RIGHTUP, 0),
                crate::models::MouseButton::Middle => (MOUSEEVENTF_MIDDLEUP, 0),
                crate::models::MouseButton::XButton1 => (MOUSEEVENTF_XUP, 1),
                crate::models::MouseButton::XButton2 => (MOUSEEVENTF_XUP, 2),
            };
            send_mouse_input(up_flag, mouse_data);
        }
    }
}

#[cfg(not(windows))]
pub fn release_all_simulated_inputs() {}

#[cfg(windows)]
fn send_char_event(ch: char) {
    let mut buf = [0u16; 2];
    let encoded = ch.encode_utf16(&mut buf);
    for &code in encoded.iter() {
        let input_down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let input_up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(&[input_down, input_up], std::mem::size_of::<INPUT>() as i32);
        }
    }
}

pub async fn execute_macro_event(
    event: &crate::models::MacroEvent,
    speed: f32,
    context: &HashMap<String, String>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        match event {
            crate::models::MacroEvent::KeyDown { key } => {
                if let Some(vk) = parse_virtual_key(key) {
                    send_key_event(vk, false);
                }
            }
            crate::models::MacroEvent::KeyUp { key } => {
                if let Some(vk) = parse_virtual_key(key) {
                    send_key_event(vk, true);
                }
            }
            crate::models::MacroEvent::KeyPress { key } => {
                if let Some(vk) = parse_virtual_key(key) {
                    send_key_event(vk, false);
                    let hold_ms = random_gaussian(55.0, 12.0).round().clamp(35.0, 110.0) as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(hold_ms)).await;
                    send_key_event(vk, true);
                }
            }
            crate::models::MacroEvent::Text { text, delay_ms, organic } => {
                let interpolated = ActionRunner::interpolate_tokens(text, context);
                simulate_text_typing(&interpolated, *delay_ms, *organic, 30)?;
            }
            crate::models::MacroEvent::MouseMove { x, y, coord_mode, smooth, duration_ms, organic } => {
                let default_dur = if *smooth { 250 } else { 16 };
                let base_dur = duration_ms.unwrap_or(default_dur);
                let scaled_duration = Some((base_dur as f32 / speed.max(0.01)).round().max(1.0) as u32);
                mouse_control(
                    MouseActionType::Move,
                    MouseButton::Left,
                    *coord_mode,
                    Some(*x),
                    Some(*y),
                    *smooth,
                    scaled_duration,
                    *organic,
                    None,
                    None,
                    None,
                    false,
                )?;
            }
            crate::models::MacroEvent::MouseDown { button } => {
                mouse_control(
                    MouseActionType::Down,
                    *button,
                    MouseCoordMode::Current,
                    None, None, false, None, false, None, None, None, false,
                )?;
            }
            crate::models::MacroEvent::MouseUp { button } => {
                mouse_control(
                    MouseActionType::Up,
                    *button,
                    MouseCoordMode::Current,
                    None, None, false, None, false, None, None, None, false,
                )?;
            }
            crate::models::MacroEvent::MouseClick { button, clicks, x, y, coord_mode, smooth } => {
                let cm = if x.is_some() || y.is_some() { *coord_mode } else { MouseCoordMode::Current };
                let (disp_x, disp_y) = if let (Some(px), Some(py)) = (x, y) {
                    let jx = random_gaussian(0.0, 1.2).round() as i32;
                    let jy = random_gaussian(0.0, 1.2).round() as i32;
                    (Some(px + jx), Some(py + jy))
                } else {
                    (*x, *y)
                };
                let click_hold = random_gaussian(45.0, 10.0).round().clamp(30.0, 85.0) as u32;
                mouse_control(
                    MouseActionType::Click,
                    *button,
                    cm,
                    disp_x,
                    disp_y,
                    *smooth,
                    Some(200),
                    false,
                    None,
                    Some(*clicks),
                    Some(click_hold),
                    true,
                )?;
            }
            crate::models::MacroEvent::MouseWheel { delta } => {
                mouse_control(
                    crate::models::MouseActionType::Scroll,
                    crate::models::MouseButton::Left,
                    crate::models::MouseCoordMode::Current,
                    None, None, false, None, false,
                    Some(*delta),
                    None, None, false,
                )?;
            }
            crate::models::MacroEvent::Delay { ms, organic, jitter_percent } => {
                let eff_ms = (*ms as f32 / speed.max(0.01)).round() as u32;
                let sleep_ms = calculate_sleep_ms(eff_ms, *organic, *jitter_percent, None);
                if sleep_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (event, speed, context);
    }
    Ok(())
}

fn set_power_plan(plan: PowerPlanType) -> Result<(), String> {
    let guid = match plan {
        PowerPlanType::HighPerformance => "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
        PowerPlanType::Balanced => "381b4222-f694-41f0-9685-ff5bb577002f",
        PowerPlanType::PowerSaver => "a1841308-3541-4fab-bc81-f71556f20b4a",
    };
    let mut cmd = Command::new("powercfg");
    cmd.args(["/setactive", guid]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let _ = cmd.spawn();
    Ok(())
}

#[cfg(windows)]
extern "system" {
    fn mciSendStringW(
        lpstrcommand: windows::core::PCWSTR,
        lpstrreturnstring: windows::core::PWSTR,
        ucmbreturn: u32,
        hwndcallback: windows::Win32::Foundation::HWND,
    ) -> u32;
}

fn play_sound_file(path: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let lower = path.to_lowercase();
        if lower.ends_with(".wav") {
            let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let _ = PlaySoundW(PCWSTR(path_w.as_ptr()), None, SND_FILENAME | SND_ASYNC);
            }
        } else {
            let close_cmd: Vec<u16> = "close ph_media\0".encode_utf16().collect();
            let open_cmd_str = format!("open \"{}\" alias ph_media\0", path);
            let open_cmd: Vec<u16> = open_cmd_str.encode_utf16().collect();
            let play_cmd: Vec<u16> = "play ph_media from 0\0".encode_utf16().collect();
            unsafe {
                let _ = mciSendStringW(PCWSTR(close_cmd.as_ptr()), windows::core::PWSTR::null(), 0, HWND(std::ptr::null_mut()));
                let res = mciSendStringW(PCWSTR(open_cmd.as_ptr()), windows::core::PWSTR::null(), 0, HWND(std::ptr::null_mut()));
                if res == 0 {
                    let _ = mciSendStringW(PCWSTR(play_cmd.as_ptr()), windows::core::PWSTR::null(), 0, HWND(std::ptr::null_mut()));
                } else {
                    let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
                    let _ = PlaySoundW(PCWSTR(path_w.as_ptr()), None, SND_FILENAME | SND_ASYNC);
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
    }
    Ok(())
}

fn set_firewall_rule(proc_name: &str, block: bool) -> Result<(), String> {
    let rule_name = format!("ProcessHandler_{}", proc_name);
    let exe_path = crate::daemon::watcher::ProcessWatcher::find_exe_path_by_name(proc_name)
        .unwrap_or_else(|| proc_name.to_string());
    let mut cmd = Command::new("netsh");
    if block {
        cmd.args([
            "advfirewall", "firewall", "add", "rule",
            &format!("name={}", rule_name),
            "dir=out",
            "action=block",
            &format!("program={}", exe_path),
        ]);
    } else {
        cmd.args(["advfirewall", "firewall", "delete", "rule", &format!("name={}", rule_name)]);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let _ = cmd.spawn();
    Ok(())
}

#[cfg(windows)]
fn construct_waveformat_extensible(sample_rate: u32, bit_depth: u16, channels: u16) -> [u8; 40] {
    let mut bytes = [0u8; 40];
    let block_align = channels * (bit_depth / 8);
    let avg_bytes_per_sec = sample_rate * (block_align as u32);
    let channel_mask: u32 = if channels == 1 { 0x4 } else { 0x3 };

    bytes[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
    bytes[2..4].copy_from_slice(&channels.to_le_bytes());
    bytes[4..8].copy_from_slice(&sample_rate.to_le_bytes());
    bytes[8..12].copy_from_slice(&avg_bytes_per_sec.to_le_bytes());
    bytes[12..14].copy_from_slice(&block_align.to_le_bytes());
    bytes[14..16].copy_from_slice(&bit_depth.to_le_bytes());
    bytes[16..18].copy_from_slice(&22u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&bit_depth.to_le_bytes());
    bytes[20..24].copy_from_slice(&channel_mask.to_le_bytes());

    let pcm_guid: [u8; 16] = [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
        0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71
    ];
    bytes[24..40].copy_from_slice(&pcm_guid);
    bytes
}

#[cfg(windows)]
fn set_audio_device_format(
    target: crate::models::AudioTarget,
    sample_rate: u32,
    bit_depth: u32,
    channels: u32,
    _device_name: Option<&str>,
) -> Result<(), String> {
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};
    use windows::Win32::Media::Audio::{eCapture, eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};

    let sample_rate = sample_rate.clamp(8000, 384000);
    let bit_depth = (bit_depth as u16).clamp(8, 32);
    let channels = (channels as u16).clamp(1, 8);

    let wave_bytes = construct_waveformat_extensible(sample_rate, bit_depth, channels);

    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let must_uninit = hr.is_ok();

        let res = (|| -> Result<(), String> {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("CoCreateInstance MMDeviceEnumerator failed: {e}"))?;

            let data_flow = match target {
                crate::models::AudioTarget::Playback | crate::models::AudioTarget::Process => eRender,
                crate::models::AudioTarget::Microphone => eCapture,
            };

            let device = enumerator.GetDefaultAudioEndpoint(data_flow, eMultimedia)
                .map_err(|e| format!("GetDefaultAudioEndpoint failed: {e}"))?;

            let dev_id_pwstr = device.GetId()
                .map_err(|e| format!("GetId failed: {e}"))?;
            let dev_id = dev_id_pwstr.to_string().map_err(|e| format!("DevId parse: {e}"))?;

            let flow_dir = match target {
                crate::models::AudioTarget::Playback | crate::models::AudioTarget::Process => "Render",
                crate::models::AudioTarget::Microphone => "Capture",
            };

            let byte_list = wave_bytes.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
            let ps_script = format!(
                r#"
                $id = '{}'
                $keyPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\{}\$id\Properties"
                if (Test-Path $keyPath) {{
                    $bytes = [byte[]]@({})
                    Set-ItemProperty -Path $keyPath -Name '{{e4870e26-3cc5-4cd2-ba46-56665703c64e}},0' -Value $bytes -Type Binary -ErrorAction SilentlyContinue
                    Set-ItemProperty -Path $keyPath -Name '{{e4870e26-3cc5-4cd2-ba46-56665703c64e}},3' -Value $bytes -Type Binary -ErrorAction SilentlyContinue
                    Set-ItemProperty -Path $keyPath -Name '{{f19f064d-0824-4d22-8222-99be777a77f0}},0' -Value $bytes -Type Binary -ErrorAction SilentlyContinue
                }}
                "#,
                dev_id.replace('\'', "''"),
                flow_dir,
                byte_list
            );

            let mut cmd = std::process::Command::new("powershell");
            cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script]);
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
            let _ = cmd.output().map_err(|e| format!("Failed to apply audio format: {e}"))?;

            Ok(())
        })();

        if must_uninit {
            CoUninitialize();
        }

        res
    }
}

#[cfg(windows)]
fn set_audio_volume_or_mute(
    action_type: AudioVolumeAction,
    volume_level: u8,
    left_volume: Option<u8>,
    right_volume: Option<u8>,
) -> Result<(), String> {
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};
    use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;

    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let must_uninit = hr.is_ok();

        let res = (|| -> Result<(), String> {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("CoCreateInstance MMDeviceEnumerator failed: {e}"))?;

            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|e| format!("GetDefaultAudioEndpoint failed: {e}"))?;

            let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioEndpointVolume failed: {e}"))?;

            match action_type {
                AudioVolumeAction::Mute => {
                    endpoint_volume.SetMute(true, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                AudioVolumeAction::Unmute => {
                    endpoint_volume.SetMute(false, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                AudioVolumeAction::ToggleMute => {
                    let current = endpoint_volume.GetMute()
                        .map_err(|e| format!("GetMute failed: {e}"))?;
                    endpoint_volume.SetMute(!current.as_bool(), std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                AudioVolumeAction::SetVolume => {
                    let channel_count = endpoint_volume.GetChannelCount().unwrap_or(2);
                    if let Some(left) = left_volume {
                        if channel_count > 0 {
                            let left_scalar = (left.min(100) as f32) / 100.0;
                            let _ = endpoint_volume.SetChannelVolumeLevelScalar(0, left_scalar, std::ptr::null());
                        }
                    }
                    if let Some(right) = right_volume {
                        if channel_count > 1 {
                            let right_scalar = (right.min(100) as f32) / 100.0;
                            let _ = endpoint_volume.SetChannelVolumeLevelScalar(1, right_scalar, std::ptr::null());
                        }
                    }
                    if left_volume.is_none() && right_volume.is_none() {
                        let scalar = (volume_level.min(100) as f32) / 100.0;
                        endpoint_volume.SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                            .map_err(|e| format!("SetMasterVolumeLevelScalar failed: {e}"))?;
                    }
                }
                AudioVolumeAction::PlayBeep => {}
            }
            Ok(())
        })();

        if must_uninit {
            CoUninitialize();
        }

        res
    }
}

#[cfg(windows)]
fn set_microphone_volume_or_mute(action_type: AudioVolumeAction, volume_level: u8) -> Result<(), String> {
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};
    use windows::Win32::Media::Audio::{eCapture, eCommunications, eConsole, eMultimedia, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;

    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let must_uninit = hr.is_ok();

        let res = (|| -> Result<(), String> {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("CoCreateInstance MMDeviceEnumerator failed: {e}"))?;

            let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)
                .or_else(|_| enumerator.GetDefaultAudioEndpoint(eCapture, eCommunications))
                .or_else(|_| enumerator.GetDefaultAudioEndpoint(eCapture, eMultimedia))
                .map_err(|e| format!("GetDefaultAudioEndpoint (eCapture) failed: {e}"))?;

            let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioEndpointVolume failed: {e}"))?;

            match action_type {
                AudioVolumeAction::Mute => {
                    endpoint_volume.SetMute(true, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                AudioVolumeAction::Unmute => {
                    endpoint_volume.SetMute(false, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                AudioVolumeAction::ToggleMute => {
                    let current = endpoint_volume.GetMute()
                        .map_err(|e| format!("GetMute failed: {e}"))?;
                    endpoint_volume.SetMute(!current.as_bool(), std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                AudioVolumeAction::SetVolume => {
                    let scalar = (volume_level.min(100) as f32) / 100.0;
                    endpoint_volume.SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                        .map_err(|e| format!("SetMasterVolumeLevelScalar failed: {e}"))?;
                }
                AudioVolumeAction::PlayBeep => {}
            }
            Ok(())
        })();

        if must_uninit {
            CoUninitialize();
        }

        res
    }
}

#[cfg(windows)]
fn set_process_audio(
    target_process: &str,
    target_pid: Option<u32>,
    action_type: AudioVolumeAction,
    volume_level: u8,
) -> Result<bool, String> {
    use windows::core::Interface;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};
    use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, IAudioSessionManager2, IAudioSessionControl2, ISimpleAudioVolume};

    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let must_uninit = hr.is_ok();

        let res = (|| -> Result<bool, String> {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("CoCreateInstance MMDeviceEnumerator failed: {e}"))?;

            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|e| format!("GetDefaultAudioEndpoint failed: {e}"))?;

            let session_manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioSessionManager2 failed: {e}"))?;

            let session_enumerator = session_manager.GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator failed: {e}"))?;

            let count = session_enumerator.GetCount()
                .map_err(|e| format!("GetCount failed: {e}"))?;

            let proc_map = crate::daemon::watcher::ProcessWatcher::snapshot_process_map();
            let target_clean = target_process.trim().to_lowercase();
            let is_wildcard_or_current = target_clean == "*" || target_clean == "current" || target_clean.is_empty();

            let mut matched_any = false;

            for i in 0..count {
                let session_control = match session_enumerator.GetSession(i) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let session_control2: IAudioSessionControl2 = match session_control.cast() {
                    Ok(c2) => c2,
                    Err(_) => continue,
                };

                let pid = match session_control2.GetProcessId() {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                if pid == 0 {
                    continue;
                }

                let matches_pid = target_pid.map(|tp| tp == pid).unwrap_or(false);
                let matches_name = if !target_clean.is_empty() && target_clean != "current" {
                    proc_map.get(&pid).map(|name| {
                        name.eq_ignore_ascii_case(&target_clean)
                            || name.trim_end_matches(".exe").eq_ignore_ascii_case(target_clean.trim_end_matches(".exe"))
                    }).unwrap_or(false)
                } else {
                    false
                };

                if (is_wildcard_or_current && matches_pid) || matches_name || (target_clean == "*" && target_pid.is_none()) {
                    let volume: ISimpleAudioVolume = match session_control.cast() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    match action_type {
                        AudioVolumeAction::Mute => {
                            volume.SetMute(true, std::ptr::null())
                                .map_err(|e| format!("SetMute failed: {e}"))?;
                        }
                        AudioVolumeAction::Unmute => {
                            volume.SetMute(false, std::ptr::null())
                                .map_err(|e| format!("SetMute failed: {e}"))?;
                        }
                        AudioVolumeAction::ToggleMute => {
                            let current = volume.GetMute()
                                .map_err(|e| format!("GetMute failed: {e}"))?;
                            volume.SetMute(!current.as_bool(), std::ptr::null())
                                .map_err(|e| format!("SetMute failed: {e}"))?;
                        }
                        AudioVolumeAction::SetVolume => {
                            let scalar = (volume_level.min(100) as f32) / 100.0;
                            volume.SetMasterVolume(scalar, std::ptr::null())
                                .map_err(|e| format!("SetMasterVolume failed: {e}"))?;
                        }
                        AudioVolumeAction::PlayBeep => {}
                    }
                    matched_any = true;
                }
            }

            Ok(matched_any)
        })();

        if must_uninit {
            CoUninitialize();
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_held_inputs_and_release() {
        #[cfg(windows)]
        {
            use windows::Win32::UI::Input::KeyboardAndMouse::VK_W;
            // Simulate key down tracking
            send_key_event(VK_W, false);
            assert!(ACTIVE_HELD_KEYS.lock().unwrap().contains(&VK_W.0));

            // Track mouse button
            if let Ok(mut lock) = ACTIVE_HELD_MOUSE_BUTTONS.lock() {
                lock.insert(crate::models::MouseButton::Left);
            }
            assert!(ACTIVE_HELD_MOUSE_BUTTONS.lock().unwrap().contains(&crate::models::MouseButton::Left));

            // Release all simulated inputs immediately
            release_all_simulated_inputs();

            // Verify both sets are completely cleared
            assert!(ACTIVE_HELD_KEYS.lock().unwrap().is_empty());
            assert!(ACTIVE_HELD_MOUSE_BUTTONS.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn test_relative_mouse_control_no_panic() {
        #[cfg(windows)]
        {
            let res = mouse_control(
                crate::models::MouseActionType::Move,
                crate::models::MouseButton::Left,
                crate::models::MouseCoordMode::Relative,
                Some(50),
                Some(-30),
                true,
                Some(50),
                false,
                None,
                None,
                None,
                false,
            );
            assert!(res.is_ok());
        }
    }

    #[tokio::test]
    async fn test_if_else_branches_execution() {
        let temp_dir = std::env::temp_dir();
        let existing_path = temp_dir.join("ph_action_test_exists.tmp");
        std::fs::write(&existing_path, b"1").unwrap();
        let non_existing_path = temp_dir.join("ph_action_test_never_exists.tmp");

        let action = crate::models::Action::IfElse {
            conditions: vec![crate::models::RuleConditionItem::new(crate::models::RuleCondition::FileWatcher {
                path: non_existing_path.to_string_lossy().to_string(),
                filter_extension: None,
                must_exist: true,
            })],
            then_actions: vec![crate::models::Action::Delay { seconds: 0.001 }],
            else_if_branches: vec![crate::models::ElseIfBranch {
                conditions: vec![crate::models::RuleConditionItem::new(crate::models::RuleCondition::FileWatcher {
                    path: existing_path.to_string_lossy().to_string(),
                    filter_extension: None,
                    must_exist: true,
                })],
                actions: vec![crate::models::Action::Delay { seconds: 0.001 }],
            }],
            else_actions: vec![],
        };

        let context = std::collections::HashMap::new();
        let res = ActionRunner::execute(&action, &context).await;
        assert!(res.is_ok());

        let _ = std::fs::remove_file(&existing_path);
    }

    #[tokio::test]
    async fn test_archive_pack_unpack_zip() {
        let temp_dir = std::env::temp_dir().join(format!("ph_test_arch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src");
        let extract_dir = temp_dir.join("extracted");
        let zip_file = temp_dir.join("bundle.zip");

        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.txt"), b"Hello ProcessHandler Archive").unwrap();

        let pack_action = crate::models::Action::Archive {
            operation: crate::models::ArchiveOperation::Pack,
            format: crate::models::ArchiveFormat::Zip,
            source_path: src_dir.to_string_lossy().to_string(),
            destination_path: zip_file.to_string_lossy().to_string(),
            compression_level: None,
            overwrite: true,
        };
        let ctx = std::collections::HashMap::new();
        let pack_res = ActionRunner::execute(&pack_action, &ctx).await;
        assert!(pack_res.is_ok(), "Pack failed: {:?}", pack_res);
        assert!(zip_file.exists());

        let unpack_action = crate::models::Action::Archive {
            operation: crate::models::ArchiveOperation::Unpack,
            format: crate::models::ArchiveFormat::Zip,
            source_path: zip_file.to_string_lossy().to_string(),
            destination_path: extract_dir.to_string_lossy().to_string(),
            compression_level: None,
            overwrite: true,
        };
        let unpack_res = ActionRunner::execute(&unpack_action, &ctx).await;
        assert!(unpack_res.is_ok(), "Unpack failed: {:?}", unpack_res);
        assert!(extract_dir.join("hello.txt").exists());
        let data = std::fs::read_to_string(extract_dir.join("hello.txt")).unwrap();
        assert_eq!(data, "Hello ProcessHandler Archive");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_file_system_crud_actions() {
        let temp_dir = std::env::temp_dir().join(format!("ph_test_fs_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let ctx = std::collections::HashMap::new();

        let dir_path = temp_dir.join("folder");
        let act_mkdir = crate::models::Action::FileSystem {
            operation: crate::models::FileSystemOperation::CreateDirectory,
            path: dir_path.to_string_lossy().to_string(),
            target_path: None,
            content: None,
            create_parents: true,
            overwrite: true,
        };
        assert!(ActionRunner::execute(&act_mkdir, &ctx).await.is_ok());
        assert!(dir_path.exists());

        let file_path = dir_path.join("data.txt");
        let act_write = crate::models::Action::FileSystem {
            operation: crate::models::FileSystemOperation::WriteText,
            path: file_path.to_string_lossy().to_string(),
            target_path: None,
            content: Some("First Line\n".to_string()),
            create_parents: true,
            overwrite: true,
        };
        assert!(ActionRunner::execute(&act_write, &ctx).await.is_ok());

        let act_append = crate::models::Action::FileSystem {
            operation: crate::models::FileSystemOperation::AppendText,
            path: file_path.to_string_lossy().to_string(),
            target_path: None,
            content: Some("Second Line".to_string()),
            create_parents: true,
            overwrite: true,
        };
        assert!(ActionRunner::execute(&act_append, &ctx).await.is_ok());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "First Line\nSecond Line");

        // Verify overwrite=false does not overwrite existing file
        let act_write_no_overwrite = crate::models::Action::FileSystem {
            operation: crate::models::FileSystemOperation::WriteText,
            path: file_path.to_string_lossy().to_string(),
            target_path: None,
            content: Some("Should not overwrite".to_string()),
            create_parents: true,
            overwrite: false,
        };
        assert!(ActionRunner::execute(&act_write_no_overwrite, &ctx).await.is_ok());
        let content_after = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content_after, "First Line\nSecond Line");

        let copy_target = dir_path.join("data_copy.txt");
        let act_copy = crate::models::Action::FileSystem {
            operation: crate::models::FileSystemOperation::Copy,
            path: file_path.to_string_lossy().to_string(),
            target_path: Some(copy_target.to_string_lossy().to_string()),
            content: None,
            create_parents: true,
            overwrite: true,
        };
        assert!(ActionRunner::execute(&act_copy, &ctx).await.is_ok());
        assert!(copy_target.exists());

        let act_del = crate::models::Action::FileSystem {
            operation: crate::models::FileSystemOperation::Delete,
            path: copy_target.to_string_lossy().to_string(),
            target_path: None,
            content: None,
            create_parents: false,
            overwrite: true,
        };
        assert!(ActionRunner::execute(&act_del, &ctx).await.is_ok());
        assert!(!copy_target.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
