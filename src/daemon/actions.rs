use std::collections::HashMap;
use std::process::Command;
use crate::models::{Action, MicrophoneActionType, ProcessAudioActionType, SoundActionType, SystemPowerType};

#[cfg(windows)]
use windows::Win32::System::Shutdown::LockWorkStation;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageW, HWND_BROADCAST, WM_SYSCOMMAND,
};

pub struct ActionRunner;

impl ActionRunner {
    pub async fn execute(action: &Action, context: &HashMap<String, String>) -> Result<(), String> {
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

            Action::PowerShell {
                script,
                run_in_window,
            } => {
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
                }

                let _ = cmd.spawn();
                Ok(())
            }

            Action::Cmd {
                command,
                run_in_window,
            } => {
                let cmd_text = Self::interpolate_tokens(command, context);
                let mut cmd = Command::new("cmd");
                cmd.args(["/C", &cmd_text]);

                if !run_in_window {
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x08000000);
                    }
                }

                let _ = cmd.spawn();
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

            Action::Sound { sound_type, volume_level } => {
                if *sound_type == SoundActionType::PlayBeep {
                    #[cfg(windows)]
                    unsafe {
                        let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(
                            windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE(0),
                        );
                    }
                } else {
                    #[cfg(windows)]
                    {
                        if let Err(e) = set_audio_volume_or_mute(*sound_type, *volume_level) {
                            eprintln!("Audio action error: {e}");
                        }
                    }
                }
                Ok(())
            }

            Action::Microphone { action_type, volume_level } => {
                #[cfg(windows)]
                {
                    if let Err(e) = set_microphone_volume_or_mute(*action_type, *volume_level) {
                        eprintln!("Microphone action error: {e}");
                    }
                }
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

            Action::ProcessAudio {
                process_name,
                action_type,
                volume_level,
            } => {
                let proc_name = Self::interpolate_tokens(process_name, context);
                let target_pid = context.get("ProcessId").and_then(|p| p.parse::<u32>().ok());

                #[cfg(windows)]
                {
                    // Retry up to 15 times with 150ms delay (~2.2s total) in case audio session is not yet registered
                    let mut matched = false;
                    for attempt in 0..15 {
                        match set_process_audio(&proc_name, target_pid, *action_type, *volume_level) {
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
                    if !matched {
                        return Err(format!("No audio session found for process '{}'", proc_name));
                    }
                }
                Ok(())
            }

            Action::Delay { seconds } => {
                tokio::time::sleep(std::time::Duration::from_secs_f64(*seconds)).await;
                Ok(())
            }
        }
    }

    fn interpolate_tokens(text: &str, context: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (k, v) in context {
            result = result.replace(&format!("{{{}}}", k), v);
        }
        result
    }
}

#[cfg(windows)]
fn set_audio_volume_or_mute(sound_type: SoundActionType, volume_level: u8) -> Result<(), String> {
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

            match sound_type {
                SoundActionType::Mute => {
                    endpoint_volume.SetMute(true, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                SoundActionType::Unmute => {
                    endpoint_volume.SetMute(false, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                SoundActionType::ToggleMute => {
                    let current = endpoint_volume.GetMute()
                        .map_err(|e| format!("GetMute failed: {e}"))?;
                    endpoint_volume.SetMute(!current.as_bool(), std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                SoundActionType::SetVolume => {
                    let scalar = (volume_level.min(100) as f32) / 100.0;
                    endpoint_volume.SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                        .map_err(|e| format!("SetMasterVolumeLevelScalar failed: {e}"))?;
                }
                SoundActionType::PlayBeep => {}
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
fn set_microphone_volume_or_mute(action_type: MicrophoneActionType, volume_level: u8) -> Result<(), String> {
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
                MicrophoneActionType::Mute => {
                    endpoint_volume.SetMute(true, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                MicrophoneActionType::Unmute => {
                    endpoint_volume.SetMute(false, std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                MicrophoneActionType::ToggleMute => {
                    let current = endpoint_volume.GetMute()
                        .map_err(|e| format!("GetMute failed: {e}"))?;
                    endpoint_volume.SetMute(!current.as_bool(), std::ptr::null())
                        .map_err(|e| format!("SetMute failed: {e}"))?;
                }
                MicrophoneActionType::SetVolume => {
                    let scalar = (volume_level.min(100) as f32) / 100.0;
                    endpoint_volume.SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                        .map_err(|e| format!("SetMasterVolumeLevelScalar failed: {e}"))?;
                }
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
    action_type: ProcessAudioActionType,
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
                        ProcessAudioActionType::Mute => {
                            volume.SetMute(true, std::ptr::null())
                                .map_err(|e| format!("SetMute failed: {e}"))?;
                        }
                        ProcessAudioActionType::Unmute => {
                            volume.SetMute(false, std::ptr::null())
                                .map_err(|e| format!("SetMute failed: {e}"))?;
                        }
                        ProcessAudioActionType::ToggleMute => {
                            let current = volume.GetMute()
                                .map_err(|e| format!("GetMute failed: {e}"))?;
                            volume.SetMute(!current.as_bool(), std::ptr::null())
                                .map_err(|e| format!("SetMute failed: {e}"))?;
                        }
                        ProcessAudioActionType::SetVolume => {
                            let scalar = (volume_level.min(100) as f32) / 100.0;
                            volume.SetMasterVolume(scalar, std::ptr::null())
                                .map_err(|e| format!("SetMasterVolume failed: {e}"))?;
                        }
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
