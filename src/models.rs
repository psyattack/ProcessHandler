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
    #[serde(rename = "window_focus")]
    WindowFocus {
        process_name: String,
        #[serde(default = "default_true")]
        on_focus_gained: bool,
        #[serde(default)]
        on_focus_lost: bool,
    },
    #[serde(rename = "resource")]
    ResourceThreshold {
        process_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cpu_percent: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ram_mb: Option<u64>,
        #[serde(default = "default_threshold_duration")]
        duration_sec: u64,
    },
    #[serde(rename = "hotkey")]
    GlobalHotkey {
        key_combination: String,
        #[serde(default)]
        hold_mode: bool,
        #[serde(default)]
        repeat_while_held: bool,
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
    #[serde(rename = "file_watcher")]
    FileWatcher {
        folder_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter_extension: Option<String>,
        #[serde(default = "default_true")]
        on_created: bool,
        #[serde(default)]
        on_modified: bool,
        #[serde(default)]
        on_deleted: bool,
    },
    #[serde(rename = "webhook")]
    Webhook {
        #[serde(default)]
        endpoint_path: String,
    },
}

fn default_threshold_duration() -> u64 {
    5
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    #[default]
    AppStartup,
    AppShutdown,
    Interval,
    Daily,
    OneShot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(from = "RuleConditionItemHelper", into = "RuleConditionItemHelper")]
pub struct RuleConditionItem {
    pub operator: ConditionOperator,
    pub condition: RuleCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum RuleConditionItemHelper {
    Item {
        #[serde(default)]
        operator: ConditionOperator,
        condition: RuleCondition,
    },
    Bare(RuleCondition),
}

impl From<RuleConditionItemHelper> for RuleConditionItem {
    fn from(helper: RuleConditionItemHelper) -> Self {
        match helper {
            RuleConditionItemHelper::Item { operator, condition } => Self { operator, condition },
            RuleConditionItemHelper::Bare(condition) => Self {
                operator: ConditionOperator::And,
                condition,
            },
        }
    }
}

impl From<RuleConditionItem> for RuleConditionItemHelper {
    fn from(item: RuleConditionItem) -> Self {
        RuleConditionItemHelper::Item {
            operator: item.operator,
            condition: item.condition,
        }
    }
}

impl RuleConditionItem {
    pub fn new(condition: RuleCondition) -> Self {
        Self {
            operator: ConditionOperator::And,
            condition,
        }
    }

    pub fn with_operator(operator: ConditionOperator, condition: RuleCondition) -> Self {
        Self {
            operator,
            condition,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "$condition_type", rename_all = "snake_case")]
pub enum RuleCondition {
    #[serde(rename = "process_running")]
    ProcessRunning {
        process_name: String,
        #[serde(default)]
        match_mode: ProcessMatchMode,
        #[serde(default = "default_true")]
        must_be_running: bool,
    },
    #[serde(rename = "window_focused")]
    WindowFocused {
        process_name: String,
        #[serde(default)]
        match_mode: ProcessMatchMode,
        #[serde(default = "default_true")]
        must_be_focused: bool,
    },
    #[serde(rename = "time_range")]
    TimeRange {
        start_time: String, // "HH:MM"
        end_time: String,   // "HH:MM"
        #[serde(default)]
        days_of_week: Vec<u8>, // 1 = Monday .. 7 = Sunday
    },
    #[serde(rename = "power_source")]
    PowerSource {
        #[serde(default = "default_true")]
        require_ac: bool,
    },
    #[serde(rename = "file_exists")]
    FileExists {
        path: String,
        #[serde(default = "default_true")]
        must_exist: bool,
    },
}

impl RuleCondition {
    pub fn default_process() -> Self {
        Self::ProcessRunning {
            process_name: String::new(),
            match_mode: ProcessMatchMode::Exact,
            must_be_running: true,
        }
    }

    pub fn default_window() -> Self {
        Self::WindowFocused {
            process_name: String::new(),
            match_mode: ProcessMatchMode::Exact,
            must_be_focused: true,
        }
    }

    pub fn default_time() -> Self {
        Self::TimeRange {
            start_time: "09:00".to_string(),
            end_time: "18:00".to_string(),
            days_of_week: vec![1, 2, 3, 4, 5],
        }
    }

    pub fn default_power() -> Self {
        Self::PowerSource {
            require_ac: true,
        }
    }

    pub fn default_file() -> Self {
        Self::FileExists {
            path: String::new(),
            must_exist: true,
        }
    }

    pub fn icon_glyph(&self) -> &'static str {
        match self {
            Self::ProcessRunning { .. } => egui_phosphor::regular::CPU,
            Self::WindowFocused { .. } => egui_phosphor::regular::APP_WINDOW,
            Self::TimeRange { .. } => egui_phosphor::regular::CLOCK,
            Self::PowerSource { .. } => egui_phosphor::regular::PLUG,
            Self::FileExists { .. } => egui_phosphor::regular::FILE,
        }
    }
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
    #[serde(rename = "window")]
    Window {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_process: Option<String>,
        window_action: WindowActionType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transparency: Option<u8>,
    },
    #[serde(rename = "process_tuning")]
    ProcessTuning {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_process: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        priority: Option<ProcessPriorityLevel>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        affinity_mask: Option<u64>,
        #[serde(default)]
        trim_memory: bool,
    },
    #[serde(rename = "power_plan")]
    PowerPlan {
        plan: PowerPlanType,
    },
    #[serde(rename = "play_sound")]
    PlaySound {
        file_path: String,
    },
    #[serde(rename = "lua")]
    LuaScript {
        script: String,
    },
    #[serde(rename = "firewall")]
    Firewall {
        process_name: String,
        block: bool,
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
    #[serde(rename = "audio_control")]
    AudioControl {
        #[serde(default)]
        target: AudioTarget,
        action_type: AudioVolumeAction,
        #[serde(default)]
        volume_level: u8, // 0..100
    },
    #[serde(rename = "default_device")]
    DefaultDevice {
        #[serde(default)]
        target: AudioTarget,
        device_name: String,
    },
    #[serde(rename = "execute_script")]
    ExecuteScript {
        #[serde(default)]
        interpreter: ScriptType,
        script: String,
        #[serde(default)]
        run_in_window: bool,
    },
    #[serde(rename = "macro")]
    Macro {
        #[serde(default)]
        events: Vec<MacroEvent>,
        #[serde(default = "default_repeat_one")]
        repeat: u32,
        #[serde(default = "default_speed_one")]
        speed: f32,
        #[serde(default)]
        infinite: bool,
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
    #[serde(rename = "display")]
    Display {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refresh_rate: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        monitor_index: Option<u32>,
    },
    #[serde(rename = "audio_device")]
    AudioDevice {
        device_name: String,
    },
    #[serde(rename = "microphone_device")]
    MicrophoneDevice {
        device_name: String,
    },
    #[serde(rename = "if_else")]
    IfElse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<RuleCondition>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        conditions: Vec<RuleConditionItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        then_actions: Vec<Action>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        else_if_branches: Vec<ElseIfBranch>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        else_actions: Vec<Action>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ElseIfBranch {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<RuleConditionItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

impl ElseIfBranch {
    pub fn new() -> Self {
        Self {
            conditions: vec![RuleConditionItem::new(RuleCondition::default_process())],
            actions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum AudioTarget {
    #[default]
    Playback,
    Microphone,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum AudioVolumeAction {
    #[default]
    Mute,
    Unmute,
    ToggleMute,
    SetVolume,
    PlayBeep,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScriptType {
    #[default]
    PowerShell,
    Batch,
    Lua,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MacroEvent {
    KeyDown {
        key: String,
    },
    KeyUp {
        key: String,
    },
    KeyPress {
        key: String,
    },
    Text {
        text: String,
        #[serde(default = "default_text_delay")]
        delay_ms: u32,
        #[serde(default)]
        organic: bool,
    },
    MouseMove {
        x: i32,
        y: i32,
        #[serde(default)]
        coord_mode: MouseCoordMode,
        #[serde(default)]
        smooth: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u32>,
        #[serde(default)]
        organic: bool,
    },
    MouseDown {
        button: MouseButton,
    },
    MouseUp {
        button: MouseButton,
    },
    MouseClick {
        button: MouseButton,
        #[serde(default = "default_click_count")]
        clicks: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<i32>,
        #[serde(default)]
        coord_mode: MouseCoordMode,
        #[serde(default)]
        smooth: bool,
    },
    MouseWheel {
        delta: i32,
    },
    Delay {
        ms: u32,
        #[serde(default)]
        organic: bool,
        #[serde(default = "default_jitter_percent")]
        jitter_percent: u32,
    },
}

pub fn default_repeat_one() -> u32 {
    1
}

pub fn default_speed_one() -> f32 {
    1.0
}

pub fn default_click_count() -> u32 {
    1
}

pub fn default_text_delay() -> u32 {
    20
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum MouseActionType {
    #[default]
    Click,
    DoubleClick,
    Down,
    Up,
    Move,
    Scroll,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
    XButton1,
    XButton2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum MouseCoordMode {
    #[default]
    Current,
    Absolute,
    Relative,
}

fn default_jitter_percent() -> u32 {
    35
}

fn default_http_method() -> String {
    "POST".to_string()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowActionType {
    #[default]
    Minimize,
    Maximize,
    Restore,
    Hide,
    Show,
    Close,
    ToggleAlwaysOnTop,
    AlwaysOnTop,
    RemoveAlwaysOnTop,
    SetTransparency,
    SnapToPosition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProcessPriorityLevel {
    #[default]
    Normal,
    Realtime,
    High,
    AboveNormal,
    BelowNormal,
    Idle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum PowerPlanType {
    #[default]
    Balanced,
    HighPerformance,
    PowerSaver,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum MicrophoneActionType {
    #[default]
    Mute,
    Unmute,
    ToggleMute,
    SetVolume,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
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
    pub max_executions: Option<u64>,
    #[serde(default)]
    pub execution_count: u64,
    #[serde(default = "default_true")]
    pub stop_on_error: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Trigger>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<RuleConditionItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_exit_actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_icon_path: Option<String>,
    #[serde(default)]
    pub auto_process_icon: bool,
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            max_executions: None,
            execution_count: 0,
            stop_on_error: true,
            trigger: None,
            conditions: Vec::new(),
            actions: Vec::new(),
            on_exit_actions: Vec::new(),
            custom_icon_path: None,
            auto_process_icon: false,
        }
    }
}

impl Action {
    pub fn migrate_to_unified(&mut self) {
        match self {
            Action::Sound { sound_type, volume_level } => {
                let act = match sound_type {
                    SoundActionType::Mute => AudioVolumeAction::Mute,
                    SoundActionType::Unmute => AudioVolumeAction::Unmute,
                    SoundActionType::ToggleMute => AudioVolumeAction::ToggleMute,
                    SoundActionType::SetVolume => AudioVolumeAction::SetVolume,
                    SoundActionType::PlayBeep => AudioVolumeAction::PlayBeep,
                };
                *self = Action::AudioControl {
                    target: AudioTarget::Playback,
                    action_type: act,
                    volume_level: *volume_level,
                };
            }
            Action::Microphone { action_type, volume_level } => {
                let act = match action_type {
                    MicrophoneActionType::Mute => AudioVolumeAction::Mute,
                    MicrophoneActionType::Unmute => AudioVolumeAction::Unmute,
                    MicrophoneActionType::ToggleMute => AudioVolumeAction::ToggleMute,
                    MicrophoneActionType::SetVolume => AudioVolumeAction::SetVolume,
                };
                *self = Action::AudioControl {
                    target: AudioTarget::Microphone,
                    action_type: act,
                    volume_level: *volume_level,
                };
            }
            Action::AudioDevice { device_name } => {
                *self = Action::DefaultDevice {
                    target: AudioTarget::Playback,
                    device_name: device_name.clone(),
                };
            }
            Action::MicrophoneDevice { device_name } => {
                *self = Action::DefaultDevice {
                    target: AudioTarget::Microphone,
                    device_name: device_name.clone(),
                };
            }
            Action::PowerShell { script, run_in_window } => {
                *self = Action::ExecuteScript {
                    interpreter: ScriptType::PowerShell,
                    script: script.clone(),
                    run_in_window: *run_in_window,
                };
            }
            Action::Cmd { command, run_in_window } => {
                *self = Action::ExecuteScript {
                    interpreter: ScriptType::Batch,
                    script: command.clone(),
                    run_in_window: *run_in_window,
                };
            }
            Action::LuaScript { script } => {
                *self = Action::ExecuteScript {
                    interpreter: ScriptType::Lua,
                    script: script.clone(),
                    run_in_window: false,
                };
            }
            Action::IfElse { condition, conditions, .. } if conditions.is_empty() => {
                if let Some(c) = condition.take() {
                    conditions.push(RuleConditionItem::new(c));
                }
            }
            _ => {}
        }
    }
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
        cloned.stop_on_error = self.stop_on_error;
        cloned.custom_icon_path = self.custom_icon_path.clone();
        cloned.auto_process_icon = self.auto_process_icon;
        cloned
    }

    pub fn default_icon_glyph(&self) -> &'static str {
        if let Some(action) = self.actions.first() {
            match action {
                Action::AudioControl { target, .. } => match target {
                    AudioTarget::Playback => egui_phosphor::regular::SPEAKER_HIGH,
                    AudioTarget::Microphone => egui_phosphor::regular::MICROPHONE,
                },
                Action::DefaultDevice { target, .. } => match target {
                    AudioTarget::Playback => egui_phosphor::regular::HEADPHONES,
                    AudioTarget::Microphone => egui_phosphor::regular::MICROPHONE,
                },
                Action::ExecuteScript { interpreter, .. } => match interpreter {
                    ScriptType::PowerShell | ScriptType::Lua => egui_phosphor::regular::CODE,
                    ScriptType::Batch => egui_phosphor::regular::TERMINAL_WINDOW,
                },
                Action::Macro { .. } => egui_phosphor::regular::KEYBOARD,
                Action::StartProcess { .. } => egui_phosphor::regular::ROCKET,
                Action::KillProcess { .. } => egui_phosphor::regular::STOP,
                Action::Window { .. } => egui_phosphor::regular::APP_WINDOW,
                Action::ProcessTuning { .. } => egui_phosphor::regular::GAUGE,
                Action::PowerPlan { .. } => egui_phosphor::regular::PLUG,
                Action::PlaySound { .. } => egui_phosphor::regular::MUSIC_NOTES,
                Action::LuaScript { .. } => egui_phosphor::regular::CODE,
                Action::Firewall { .. } => egui_phosphor::regular::SHIELD,
                Action::PowerShell { .. } => egui_phosphor::regular::CODE,
                Action::Cmd { .. } => egui_phosphor::regular::TERMINAL_WINDOW,
                Action::Notification { .. } => egui_phosphor::regular::BELL,
                Action::Sound { .. } => egui_phosphor::regular::SPEAKER_HIGH,
                Action::Microphone { .. } => egui_phosphor::regular::MICROPHONE,
                Action::ProcessAudio { .. } => egui_phosphor::regular::FADERS,
                Action::SystemPower { .. } => egui_phosphor::regular::POWER,
                Action::Delay { .. } => egui_phosphor::regular::HOURGLASS,
                Action::HttpWebhook { .. } => egui_phosphor::regular::GLOBE,
                Action::Display { .. } => egui_phosphor::regular::MONITOR,
                Action::AudioDevice { .. } => egui_phosphor::regular::HEADPHONES,
                Action::MicrophoneDevice { .. } => egui_phosphor::regular::MICROPHONE,
                Action::IfElse { .. } => egui_phosphor::regular::GIT_FORK,
            }
        } else if let Some(trigger) = &self.trigger {
            match trigger {
                Trigger::Process { .. } => egui_phosphor::regular::CPU,
                Trigger::WindowFocus { .. } => egui_phosphor::regular::EYE,
                Trigger::ResourceThreshold { .. } => egui_phosphor::regular::CHART_LINE,
                Trigger::GlobalHotkey { .. } => egui_phosphor::regular::COMMAND,
                Trigger::SystemEvent { .. } => egui_phosphor::regular::LIGHTNING,
                Trigger::Schedule { .. } => egui_phosphor::regular::CALENDAR,
                Trigger::FileWatcher { .. } => egui_phosphor::regular::FOLDER_NOTCH_OPEN,
                Trigger::Webhook { .. } => egui_phosphor::regular::GLOBE_HEMISPHERE_WEST,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AnalyticsSummary {
    #[serde(default)]
    pub total_executions: u64,
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
            max_executions: Some(10),
            execution_count: 42,
            stop_on_error: true,
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
            custom_icon_path: Some("C:\\game.exe".into()),
            auto_process_icon: true,
            conditions: vec![],
        };

        let cloned = original.clone_with_new_id();
        assert_ne!(cloned.id, original.id);
        assert_eq!(cloned.name, "Game Optimizer (Copy)");
        assert_eq!(cloned.description, "Some memo");
        assert!(!cloned.is_enabled); // Paused on clone
        assert_eq!(cloned.cooldown_seconds, 1.5);
        assert_eq!(cloned.max_executions, Some(10));
        assert_eq!(cloned.trigger, original.trigger);
        assert_eq!(cloned.actions, original.actions);
        assert_eq!(cloned.execution_count, 0); // reset to 0
        assert_eq!(cloned.custom_icon_path, Some("C:\\game.exe".into()));
        assert!(cloned.auto_process_icon);
        assert!(cloned.stop_on_error);
    }

    #[test]
    fn test_rule_default_icon_glyph_per_action_and_trigger() {
        let mut rule = Rule {
            id: "r1".into(),
            name: "Test".into(),
            description: "".into(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            max_executions: None,
            execution_count: 0,
            stop_on_error: true,
            trigger: None,
            actions: vec![],
            on_exit_actions: vec![],
            custom_icon_path: None,
            auto_process_icon: false,
            conditions: vec![],
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

    #[test]
    fn test_new_actions_and_conditions_serde() {
        let rule = Rule {
            id: "rule_advanced".into(),
            name: "Industrial Workstation Setup".into(),
            description: "Applies windows, tuning and scripts".into(),
            is_enabled: true,
            cooldown_seconds: 1.0,
            max_executions: Some(5),
            execution_count: 2,
            stop_on_error: true,
            trigger: Some(Trigger::Webhook {
                endpoint_path: "studio_start".into(),
            }),
            actions: vec![
                Action::Window {
                    target_process: Some("Code.exe".into()),
                    window_action: WindowActionType::Maximize,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    transparency: Some(240),
                },
                Action::ProcessTuning {
                    target_process: Some("Code.exe".into()),
                    priority: Some(ProcessPriorityLevel::High),
                    affinity_mask: Some(0xFF),
                    trim_memory: true,
                },
                Action::Macro {
                    events: vec![
                        MacroEvent::KeyDown { key: "Shift".into() },
                        MacroEvent::KeyPress { key: "P".into() },
                        MacroEvent::KeyUp { key: "Shift".into() },
                    ],
                    repeat: 1,
                    speed: 1.0,
                    infinite: false,
                },
                Action::PowerPlan {
                    plan: PowerPlanType::HighPerformance,
                },
                Action::PlaySound {
                    file_path: "C:\\Windows\\Media\\chimes.wav".into(),
                },
                Action::LuaScript {
                    script: "ph.log('info', 'Workstation setup complete!')".into(),
                },
                Action::Firewall {
                    process_name: "telemetry.exe".into(),
                    block: true,
                },
                Action::Display {
                    width: Some(1920),
                    height: Some(1080),
                    refresh_rate: Some(144),
                    monitor_index: Some(1),
                },
                Action::AudioDevice {
                    device_name: "Headphones".into(),
                },
                Action::MicrophoneDevice {
                    device_name: "Microphone".into(),
                },
                Action::IfElse {
                    condition: None,
                    conditions: vec![RuleConditionItem::new(RuleCondition::ProcessRunning {
                        process_name: "game.exe".into(),
                        match_mode: ProcessMatchMode::Exact,
                        must_be_running: true,
                    })],
                    then_actions: vec![Action::Delay { seconds: 1.0 }],
                    else_if_branches: vec![ElseIfBranch {
                        conditions: vec![RuleConditionItem::new(RuleCondition::PowerSource {
                            require_ac: true,
                        })],
                        actions: vec![Action::Delay { seconds: 2.0 }],
                    }],
                    else_actions: vec![],
                },
            ],
            on_exit_actions: vec![],
            custom_icon_path: None,
            auto_process_icon: true,
            conditions: vec![
                RuleConditionItem::new(RuleCondition::WindowFocused {
                    process_name: "game.exe".into(),
                    match_mode: ProcessMatchMode::Exact,
                    must_be_focused: true,
                }),
                RuleConditionItem::with_operator(
                    ConditionOperator::Or,
                    RuleCondition::PowerSource {
                        require_ac: true,
                    },
                ),
            ],
        };

        let json = serde_json::to_string(&rule).expect("Serialization must succeed");
        let decoded: Rule = serde_json::from_str(&json).expect("Deserialization must succeed");

        assert_eq!(decoded.id, rule.id);
        assert_eq!(decoded.max_executions, Some(5));
        assert_eq!(decoded.execution_count, 2);
        assert!(decoded.stop_on_error);
        assert_eq!(decoded.actions.len(), 11);
        assert_eq!(decoded.conditions.len(), 2);
        assert_eq!(decoded, rule);
    }

    #[test]
    fn test_unified_actions_and_migration() {
        // 1. Test serde roundtrip of all 4 unified actions
        let unified_actions = vec![
            Action::AudioControl {
                target: AudioTarget::Playback,
                action_type: AudioVolumeAction::SetVolume,
                volume_level: 75,
            },
            Action::DefaultDevice {
                target: AudioTarget::Microphone,
                device_name: "USB Mic".into(),
            },
            Action::ExecuteScript {
                interpreter: ScriptType::PowerShell,
                script: "Get-Process".into(),
                run_in_window: true,
            },
            Action::Macro {
                events: vec![
                    MacroEvent::KeyDown { key: "Shift".into() },
                    MacroEvent::KeyPress { key: "A".into() },
                    MacroEvent::KeyUp { key: "Shift".into() },
                    MacroEvent::Text { text: "Hello".into(), delay_ms: 20, organic: false },
                    MacroEvent::MouseMove {
                        x: 100,
                        y: 200,
                        smooth: true,
                        duration_ms: Some(150),
                        organic: true,
                        coord_mode: MouseCoordMode::Absolute,
                    },
                    MacroEvent::MouseClick {
                        button: MouseButton::Left,
                        clicks: 2,
                        x: None,
                        y: None,
                        coord_mode: MouseCoordMode::Absolute,
                        smooth: false,
                    },
                    MacroEvent::Delay {
                        ms: 250,
                        organic: true,
                        jitter_percent: 30,
                    },
                ],
                repeat: 3,
                speed: 1.5,
                infinite: false,
            },
        ];

        let json = serde_json::to_string(&unified_actions).expect("Must serialize");
        let decoded: Vec<Action> = serde_json::from_str(&json).expect("Must deserialize");
        assert_eq!(decoded, unified_actions);

        // 2. Test migration of legacy Sound & Mic
        let mut sound = Action::Sound {
            sound_type: SoundActionType::SetVolume,
            volume_level: 80,
        };
        sound.migrate_to_unified();
        assert_eq!(
            sound,
            Action::AudioControl {
                target: AudioTarget::Playback,
                action_type: AudioVolumeAction::SetVolume,
                volume_level: 80,
            }
        );

        let mut mic = Action::Microphone {
            action_type: MicrophoneActionType::Mute,
            volume_level: 0,
        };
        mic.migrate_to_unified();
        assert_eq!(
            mic,
            Action::AudioControl {
                target: AudioTarget::Microphone,
                action_type: AudioVolumeAction::Mute,
                volume_level: 0,
            }
        );

        // 3. Test migration of legacy Default devices
        let mut dev = Action::AudioDevice { device_name: "Speakers".into() };
        dev.migrate_to_unified();
        assert_eq!(
            dev,
            Action::DefaultDevice { target: AudioTarget::Playback, device_name: "Speakers".into() }
        );

        let mut mic_dev = Action::MicrophoneDevice { device_name: "Headset".into() };
        mic_dev.migrate_to_unified();
        assert_eq!(
            mic_dev,
            Action::DefaultDevice { target: AudioTarget::Microphone, device_name: "Headset".into() }
        );

        // 4. Test migration of scripts
        let mut ps = Action::PowerShell { script: "dir".into(), run_in_window: false };
        ps.migrate_to_unified();
        assert_eq!(
            ps,
            Action::ExecuteScript {
                interpreter: ScriptType::PowerShell,
                script: "dir".into(),
                run_in_window: false,
            }
        );

        let mut cmd = Action::Cmd { command: "echo 1".into(), run_in_window: true };
        cmd.migrate_to_unified();
        assert_eq!(
            cmd,
            Action::ExecuteScript {
                interpreter: ScriptType::Batch,
                script: "echo 1".into(),
                run_in_window: true,
            }
        );

        let mut lua = Action::LuaScript { script: "print(1)".into() };
        lua.migrate_to_unified();
        assert_eq!(
            lua,
            Action::ExecuteScript {
                interpreter: ScriptType::Lua,
                script: "print(1)".into(),
                run_in_window: false,
            }
        );
    }

    #[test]
    fn test_global_hotkey_hold_mode_and_repeat_serde() {
        // Test backwards compatibility: json without repeat_while_held defaults to false
        let json = r#"{"$trigger_type":"hotkey","key_combination":"Ctrl+Shift+H","hold_mode":true}"#;
        let trigger: Trigger = serde_json::from_str(json).unwrap();
        assert_eq!(
            trigger,
            Trigger::GlobalHotkey {
                key_combination: "Ctrl+Shift+H".to_string(),
                hold_mode: true,
                repeat_while_held: false,
            }
        );

        // Test roundtrip with repeat_while_held: true
        let trigger_loop = Trigger::GlobalHotkey {
            key_combination: "M4".to_string(),
            hold_mode: true,
            repeat_while_held: true,
        };
        let serialized = serde_json::to_string(&trigger_loop).unwrap();
        let deserialized: Trigger = serde_json::from_str(&serialized).unwrap();
        assert_eq!(trigger_loop, deserialized);
    }
}
