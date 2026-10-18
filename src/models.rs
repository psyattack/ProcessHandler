use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "$trigger_type", rename_all = "snake_case")]
pub enum Trigger {
    #[serde(rename = "process")]
    Process {
        process_name: String,
        #[serde(default)]
        match_mode: ProcessMatchMode,
        #[serde(default)]
        single_instance: bool,
    },
    #[serde(rename = "system")]
    SystemEvent {
        event_type: SystemEventType,
        #[serde(default)]
        idle_duration_sec: u32,
        #[serde(default)]
        device_id_filter: Option<String>,
    },
    #[serde(rename = "schedule")]
    Schedule {
        schedule_type: ScheduleType,
        #[serde(default)]
        interval_sec: u64,
        #[serde(default)]
        daily_time: Option<String>, // "HH:MM:SS"
        #[serde(default)]
        days_of_week: Vec<u8>,      // 1 = Monday .. 7 = Sunday
        #[serde(default)]
        delay_sec: u64,
        #[serde(default)]
        target_datetime: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProcessMatchMode {
    #[default]
    Exact,
    Wildcard,
    Regex,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemEventType {
    SessionLock,
    SessionUnlock,
    SystemSleep,
    SystemResume,
    PowerSourceAc,
    PowerSourceBattery,
    UserIdle,
    UserActive,
    DeviceConnected,
    DeviceDisconnected,
    MonitorOn,
    MonitorOff,
    NetworkConnected,
    NetworkDisconnected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    #[default]
    AppStartup,
    AppShutdown,
    Interval,
    Daily,
    OneShot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "$action_type", rename_all = "snake_case")]
pub enum Action {
    #[serde(rename = "start_process")]
    StartProcess {
        file_path: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        arguments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_directory: Option<String>,
        #[serde(default)]
        run_as_admin: bool,
        #[serde(default)]
        only_if_not_running: bool,
        #[serde(default)]
        run_hidden: bool,
        #[serde(default)]
        close_on_exit: bool,
    },
    #[serde(rename = "kill_process")]
    KillProcess {
        process_name: String,
        #[serde(default)]
        kill_tree: bool,
        #[serde(default)]
        force: bool,
    },
    #[serde(rename = "powershell")]
    PowerShell {
        script: String,
        #[serde(default)]
        run_in_window: bool,
    },
    #[serde(rename = "cmd")]
    Cmd {
        command: String,
        #[serde(default)]
        run_in_window: bool,
    },
    #[serde(rename = "notification")]
    Notification {
        title: String,
        message: String,
    },
    #[serde(rename = "http_webhook")]
    HttpWebhook {
        url: String,
        #[serde(default = "default_http_method")]
        method: String,
        #[serde(default)]
        headers: Option<std::collections::HashMap<String, String>>,
        #[serde(default)]
        payload: Option<String>,
    },
    #[serde(rename = "sound")]
    Sound {
        sound_type: SoundActionType,
        #[serde(default)]
        volume_level: u8, // 0..100
    },
    #[serde(rename = "microphone")]
    Microphone {
        action_type: MicrophoneActionType,
        #[serde(default)]
        volume_level: u8, // 0..100
    },
    #[serde(rename = "process_audio")]
    ProcessAudio {
        process_name: String,
        action_type: ProcessAudioActionType,
        #[serde(default)]
        volume_level: u8, // 0..100
    },
    #[serde(rename = "system_power")]
    SystemPower {
        power_type: SystemPowerType,
    },
    #[serde(rename = "delay")]
    Delay {
        seconds: f64,
    },
}

fn default_http_method() -> String {
    "POST".to_string()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MicrophoneActionType {
    #[default]
    Mute,
    Unmute,
    ToggleMute,
    SetVolume,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProcessAudioActionType {
    #[default]
    Mute,
    Unmute,
    ToggleMute,
    SetVolume,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundActionType {
    Mute,
    Unmute,
    ToggleMute,
    SetVolume,
    PlayBeep,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemPowerType {
    Lock,
    Sleep,
    TurnScreenOff,
    Restart,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default = "default_true")]
    pub is_enabled: bool,
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Trigger>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_exit_actions: Vec<Action>,
    #[serde(skip)]
    pub execution_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_icon_path: Option<String>,
    #[serde(default)]
    pub auto_process_icon: bool,
}

impl Rule {
    pub fn clone_with_new_id(&self) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let new_id = format!(
            "rule_{:x}_{:x}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            count
        );
        let new_name = generate_clone_name(&self.name);

        let mut cloned = self.clone();
        cloned.id = new_id;
        cloned.name = new_name;
        cloned.is_enabled = false;
        cloned.execution_count = 0;
        cloned.custom_icon_path = self.custom_icon_path.clone();
        cloned.auto_process_icon = self.auto_process_icon;
        cloned
    }

    pub fn default_icon_glyph(&self) -> &'static str {
        if let Some(action) = self.actions.first() {
            match action {
                Action::StartProcess { .. } => egui_phosphor::regular::ROCKET,
                Action::KillProcess { .. } => egui_phosphor::regular::STOP,
                Action::PowerShell { .. } => egui_phosphor::regular::CODE,
                Action::Cmd { .. } => egui_phosphor::regular::TERMINAL_WINDOW,
                Action::Notification { .. } => egui_phosphor::regular::BELL,
                Action::Sound { .. } => egui_phosphor::regular::SPEAKER_HIGH,
                Action::Microphone { .. } => egui_phosphor::regular::MICROPHONE,
                Action::ProcessAudio { .. } => egui_phosphor::regular::FADERS,
                Action::SystemPower { .. } => egui_phosphor::regular::POWER,
                Action::Delay { .. } => egui_phosphor::regular::HOURGLASS,
                Action::HttpWebhook { .. } => egui_phosphor::regular::GLOBE,
            }
        } else if let Some(trigger) = &self.trigger {
            match trigger {
                Trigger::Process { .. } => egui_phosphor::regular::CPU,
                Trigger::SystemEvent { .. } => egui_phosphor::regular::LIGHTNING,
                Trigger::Schedule { .. } => egui_phosphor::regular::CALENDAR,
            }
        } else {
            egui_phosphor::regular::GEAR
        }
    }
}

pub fn generate_clone_name(name: &str) -> String {
    if let Some(base) = name.strip_suffix(" (Copy)") {
        format!("{} (Copy 2)", base)
    } else if let Some((base, num_str)) = name.rsplit_once(" (Copy ") {
        if let Some(num_trimmed) = num_str.strip_suffix(')') {
            if let Ok(n) = num_trimmed.parse::<u32>() {
                format!("{} (Copy {})", base, n + 1)
            } else {
                format!("{} (Copy)", name)
            }
        } else {
            format!("{} (Copy)", name)
        }
    } else {
        format!("{} (Copy)", name)
    }
}

fn default_true() -> bool {
    true
}

fn default_cooldown() -> f64 {
    0.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessSnapshotItem {
    pub pid: u32,
    pub name: String,
    pub window_title: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogItem {
    pub timestamp: String,
    pub level: String,
    pub category: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_clone_name() {
        assert_eq!(generate_clone_name("Discord Mute"), "Discord Mute (Copy)");
        assert_eq!(generate_clone_name("Discord Mute (Copy)"), "Discord Mute (Copy 2)");
        assert_eq!(generate_clone_name("Discord Mute (Copy 2)"), "Discord Mute (Copy 3)");
        assert_eq!(generate_clone_name("Discord Mute (Copy 99)"), "Discord Mute (Copy 100)");
    }

    #[test]
    fn test_rule_clone_with_new_id() {
        let original = Rule {
            id: "rule_orig_123".into(),
            name: "Game Optimizer".into(),
            description: "Some memo".into(),
            is_enabled: true,
            cooldown_seconds: 1.5,
            trigger: Some(Trigger::Process {
                process_name: "game.exe".into(),
                match_mode: ProcessMatchMode::Exact,
                single_instance: true,
            }),
            actions: vec![Action::Notification {
                title: "Alert".into(),
                message: "Starting".into(),
            }],
            on_exit_actions: vec![],
            execution_count: 42,
            custom_icon_path: Some("C:\\game.exe".into()),
            auto_process_icon: true,
        };

        let cloned = original.clone_with_new_id();
        assert_ne!(cloned.id, original.id);
        assert_eq!(cloned.name, "Game Optimizer (Copy)");
        assert_eq!(cloned.description, "Some memo");
        assert!(!cloned.is_enabled); // Paused on clone
        assert_eq!(cloned.cooldown_seconds, 1.5);
        assert_eq!(cloned.trigger, original.trigger);
        assert_eq!(cloned.actions, original.actions);
        assert_eq!(cloned.execution_count, 0); // reset to 0
        assert_eq!(cloned.custom_icon_path, Some("C:\\game.exe".into()));
        assert!(cloned.auto_process_icon);
    }

    #[test]
    fn test_rule_default_icon_glyph_per_action_and_trigger() {
        let mut rule = Rule {
            id: "r1".into(),
            name: "Test".into(),
            description: "".into(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: None,
            actions: vec![],
            on_exit_actions: vec![],
            execution_count: 0,
            custom_icon_path: None,
            auto_process_icon: false,
        };

        // Fallback when empty
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::GEAR);

        // Action icons
        rule.actions = vec![Action::StartProcess {
            file_path: "app.exe".into(),
            arguments: "".into(),
            working_directory: None,
            run_as_admin: false,
            only_if_not_running: false,
            run_hidden: false,
            close_on_exit: false,
        }];
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::ROCKET);

        rule.actions = vec![Action::KillProcess {
            process_name: "app.exe".into(),
            kill_tree: true,
            force: false,
        }];
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::STOP);

        rule.actions = vec![Action::PowerShell {
            script: "echo 1".into(),
            run_in_window: false,
        }];
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::CODE);

        rule.actions = vec![Action::Delay { seconds: 5.0 }];
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::HOURGLASS);

        rule.actions = vec![Action::Microphone {
            action_type: MicrophoneActionType::Mute,
            volume_level: 100,
        }];
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::MICROPHONE);

        // Trigger icons when actions empty
        rule.actions.clear();
        rule.trigger = Some(Trigger::Process {
            process_name: "notepad.exe".into(),
            match_mode: ProcessMatchMode::Exact,
            single_instance: false,
        });
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::CPU);

        rule.trigger = Some(Trigger::SystemEvent {
            event_type: SystemEventType::SessionLock,
            idle_duration_sec: 0,
            device_id_filter: None,
        });
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::LIGHTNING);
    }
}
