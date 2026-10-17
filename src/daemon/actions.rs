use std::collections::HashMap;
use std::process::Command;
use crate::models::{Action, SoundActionType, SystemPowerType};

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
