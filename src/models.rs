use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "$trigger_type", rename_all = "snake_case")]
pub enum Trigger {
    #[serde(rename = "manual")]
    Manual,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        day_of_month: Option<u8>,   // 1..31 for Monthly schedule
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
    Monthly,
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
    #[serde(rename = "event")]
    Event {
        event_type: SystemEventType,
        #[serde(default = "default_idle_sec")]
        idle_duration_sec: u32,
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
    #[serde(rename = "file_watcher")]
    FileWatcher {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter_extension: Option<String>,
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

    pub fn default_event() -> Self {
        Self::Event {
            event_type: SystemEventType::SessionLock,
            idle_duration_sec: 300,
        }
    }

    pub fn default_resource() -> Self {
        Self::ResourceThreshold {
            process_name: String::new(),
            cpu_percent: Some(80.0),
            ram_mb: None,
            duration_sec: 5,
        }
    }

    pub fn default_file() -> Self {
        Self::FileWatcher {
            path: String::new(),
            filter_extension: None,
            must_exist: true,
        }
    }

    pub fn icon_glyph(&self) -> &'static str {
        match self {
            Self::ProcessRunning { .. } => egui_phosphor::regular::CPU,
            Self::WindowFocused { .. } => egui_phosphor::regular::APP_WINDOW,
            Self::TimeRange { .. } => egui_phosphor::regular::CLOCK,
            Self::Event { .. } => egui_phosphor::regular::BELL,
            Self::ResourceThreshold { .. } => egui_phosphor::regular::CHART_LINE,
            Self::FileWatcher { .. } => egui_phosphor::regular::FILE,
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
        #[serde(default)]
        file_path: String,
        #[serde(default)]
        play_beep: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        beep_frequency: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        beep_duration_ms: Option<u32>,
    },
    #[serde(rename = "firewall")]
    Firewall {
        process_name: String,
        block: bool,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        process_name: Option<String>,
        action_type: AudioVolumeAction,
        #[serde(default)]
        volume_level: u8, // 0..100
        #[serde(default, skip_serializing_if = "Option::is_none")]
        left_volume: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        right_volume: Option<u8>,
    },
    #[serde(rename = "audio_format")]
    AudioFormat {
        #[serde(default)]
        target: AudioTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device_name: Option<String>,
        #[serde(default = "default_sample_rate")]
        sample_rate: u32,
        #[serde(default = "default_bit_depth")]
        bit_depth: u32,
        #[serde(default = "default_channels_two")]
        channels: u32,
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
    #[serde(rename = "if_else")]
    IfElse {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        conditions: Vec<RuleConditionItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        then_actions: Vec<Action>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        else_if_branches: Vec<ElseIfBranch>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        else_actions: Vec<Action>,
    },
    #[serde(rename = "trim_working_set")]
    TrimWorkingSet {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_process: Option<String>,
    },
    #[serde(rename = "discord_rpc_set")]
    DiscordRpcSet {
        app_id: String,
        #[serde(default)]
        details: String,
        #[serde(default)]
        state: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        large_image: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        large_text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        small_image: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        small_text: Option<String>,
        #[serde(default = "default_true")]
        show_elapsed: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        buttons: Vec<DiscordRpcButton>,
        #[serde(default)]
        auto_clear_on_process_exit: bool,
    },
    #[serde(rename = "discord_rpc_clear")]
    DiscordRpcClear,
    #[serde(rename = "archive")]
    Archive {
        operation: ArchiveOperation,
        #[serde(default)]
        format: ArchiveFormat,
        source_path: String,
        destination_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        compression_level: Option<u8>,
        #[serde(default = "default_true")]
        overwrite: bool,
    },
    #[serde(rename = "file_system")]
    FileSystem {
        operation: FileSystemOperation,
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(default = "default_true")]
        create_parents: bool,
        #[serde(default = "default_true")]
        overwrite: bool,
    },
    #[serde(rename = "overlay")]
    Overlay {
        overlay_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        custom_overlay: Option<Box<OverlayDefinition>>,
        operation: OverlayOperation,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        override_text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DiscordRpcButton {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveOperation {
    #[default]
    Pack,
    Unpack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    #[default]
    Zip,
    Tar,
    TarGz,
    TarXz,
    SevenZip,
    Rar,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileSystemOperation {
    #[default]
    CreateFile,
    CreateDirectory,
    Rename,
    Copy,
    Delete,
    WriteText,
    AppendText,
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
    Process,
}

pub fn default_sample_rate() -> u32 {
    48000
}

pub fn default_bit_depth() -> u32 {
    24
}

pub fn default_channels_two() -> u32 {
    2
}

pub fn default_idle_sec() -> u32 {
    300
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_executed_at: Option<String>,
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
            trigger: Some(Trigger::Manual),
            conditions: Vec::new(),
            actions: Vec::new(),
            on_exit_actions: Vec::new(),
            custom_icon_path: None,
            auto_process_icon: false,
            last_executed_at: None,
        }
    }
}

impl Action {
    pub fn refresh_overlay_ids(&mut self) {
        match self {
            Action::Overlay { overlay_id, custom_overlay, .. } => {
                static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let fresh_id = format!(
                    "overlay_{:x}_{:x}",
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos(),
                    count
                );
                *overlay_id = fresh_id.clone();
                if let Some(def) = custom_overlay.as_deref_mut() {
                    def.id = fresh_id;
                }
            }
            Action::IfElse { then_actions, else_if_branches, else_actions, .. } => {
                for a in then_actions {
                    a.refresh_overlay_ids();
                }
                for b in else_if_branches {
                    for a in &mut b.actions {
                        a.refresh_overlay_ids();
                    }
                }
                for a in else_actions {
                    a.refresh_overlay_ids();
                }
            }
            _ => {}
        }
    }

    pub fn duplicate_with_fresh_ids(&self) -> Self {
        let mut cloned = self.clone();
        cloned.refresh_overlay_ids();
        cloned
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
        cloned.last_executed_at = None;
        cloned.stop_on_error = self.stop_on_error;
        cloned.custom_icon_path = self.custom_icon_path.clone();
        cloned.auto_process_icon = self.auto_process_icon;

        // Ensure cloned overlay actions get their own fresh unique IDs so they don't collide with original rule
        for action in cloned.actions.iter_mut() {
            action.refresh_overlay_ids();
        }
        for action in cloned.on_exit_actions.iter_mut() {
            action.refresh_overlay_ids();
        }

        cloned
    }

    pub fn default_icon_glyph(&self) -> &'static str {
        if let Some(action) = self.actions.first() {
            match action {
                Action::AudioControl { target, .. } => match target {
                    AudioTarget::Playback => egui_phosphor::regular::SPEAKER_HIGH,
                    AudioTarget::Microphone => egui_phosphor::regular::MICROPHONE,
                    AudioTarget::Process => egui_phosphor::regular::FADERS,
                },
                Action::AudioFormat { .. } => egui_phosphor::regular::EQUALIZER,
                Action::DefaultDevice { target, .. } => match target {
                    AudioTarget::Playback => egui_phosphor::regular::HEADPHONES,
                    AudioTarget::Microphone => egui_phosphor::regular::MICROPHONE,
                    AudioTarget::Process => egui_phosphor::regular::FADERS,
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
                Action::Firewall { .. } => egui_phosphor::regular::SHIELD,
                Action::Notification { .. } => egui_phosphor::regular::BELL,
                Action::SystemPower { .. } => egui_phosphor::regular::POWER,
                Action::Delay { .. } => egui_phosphor::regular::HOURGLASS,
                Action::HttpWebhook { .. } => egui_phosphor::regular::GLOBE,
                Action::Display { .. } => egui_phosphor::regular::MONITOR,
                Action::IfElse { .. } => egui_phosphor::regular::GIT_FORK,
                Action::TrimWorkingSet { .. } => egui_phosphor::regular::BROOM,
                Action::DiscordRpcSet { .. } => egui_phosphor::regular::GAME_CONTROLLER,
                Action::DiscordRpcClear => egui_phosphor::regular::X_CIRCLE,
                Action::Archive { .. } => egui_phosphor::regular::ARCHIVE,
                Action::FileSystem { .. } => egui_phosphor::regular::FOLDER,
                Action::Overlay { .. } => egui_phosphor::regular::FRAME_CORNERS,
            }
        } else if let Some(trigger) = &self.trigger {
            match trigger {
                Trigger::Manual => egui_phosphor::regular::PLAY,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverlayOperation {
    #[default]
    Show,
    Hide,
    Toggle,
    Flash,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverlayAnchor {
    #[default]
    ScreenCenter,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Custom,
    WindowRelative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for OverlayRect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 260.0,
            height: 70.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayStyle {
    #[serde(default = "default_overlay_bg")]
    pub bg_color: [u8; 4],
    #[serde(default = "default_overlay_border")]
    pub border_color: [u8; 4],
    #[serde(default = "default_border_width")]
    pub border_width: f32,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f32,
    #[serde(default)]
    pub glow: bool,
}

pub fn default_overlay_bg() -> [u8; 4] {
    [16, 17, 22, 220]
}

pub fn default_overlay_border() -> [u8; 4] {
    [255, 255, 255, 30]
}

pub fn default_border_width() -> f32 {
    1.0
}

pub fn default_corner_radius() -> f32 {
    8.0
}

impl Default for OverlayStyle {
    fn default() -> Self {
        Self {
            bg_color: default_overlay_bg(),
            border_color: default_overlay_border(),
            border_width: default_border_width(),
            corner_radius: default_corner_radius(),
            glow: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    #[default]
    Rectangle,
    Circle,
    Arrow,
    Line,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum CrosshairStyle {
    #[default]
    Cross,
    Dot,
    Circle,
    TShaped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum OverlayClickAction {
    TriggerRule { rule_id: String },
    ToggleRule { rule_id: String },
    CloseOverlay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OverlayElement {
    Text {
        #[serde(default)]
        id: String,
        text: String,
        #[serde(default = "default_font_size")]
        font_size: f32,
        #[serde(default = "default_white_color")]
        color: [u8; 4],
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<f32>,
        #[serde(default)]
        lock_aspect: bool,
        #[serde(default)]
        bold: bool,
        #[serde(default = "default_font_family")]
        font_family: String,
        #[serde(default)]
        rotation: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_click: Option<OverlayClickAction>,
    },
    Image {
        #[serde(default)]
        id: String,
        path: String,
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default = "default_image_size")]
        width: f32,
        #[serde(default = "default_image_size")]
        height: f32,
        #[serde(default = "default_opacity_one")]
        opacity: f32,
        #[serde(default = "default_true")]
        lock_aspect: bool,
        #[serde(default)]
        rotation: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_click: Option<OverlayClickAction>,
    },
    Crosshair {
        #[serde(default)]
        id: String,
        #[serde(default)]
        style: CrosshairStyle,
        #[serde(default = "default_crosshair_size")]
        size: f32,
        #[serde(default = "default_crosshair_color")]
        color: [u8; 4],
        #[serde(default = "default_true")]
        center_dot: bool,
        #[serde(default = "default_crosshair_thickness")]
        thickness: f32,
        #[serde(default = "default_crosshair_gap")]
        gap: f32,
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default)]
        rotation: f32,
    },
    ProgressBar {
        #[serde(default)]
        id: String,
        #[serde(default)]
        label: String,
        #[serde(default)]
        progress: f32,
        #[serde(default = "default_accent_color")]
        fill_color: [u8; 4],
        #[serde(default = "default_dark_color")]
        bg_color: [u8; 4],
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default = "default_progress_width")]
        width: f32,
        #[serde(default = "default_progress_height")]
        height: f32,
        #[serde(default)]
        rotation: f32,
    },
    Badge {
        #[serde(default)]
        id: String,
        text: String,
        #[serde(default = "default_accent_color")]
        color: [u8; 4],
        #[serde(default = "default_badge_bg")]
        bg_color: [u8; 4],
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default)]
        rotation: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_click: Option<OverlayClickAction>,
    },
    Shape {
        #[serde(default)]
        id: String,
        #[serde(default)]
        shape_type: ShapeKind,
        #[serde(default)]
        x: f32,
        #[serde(default)]
        y: f32,
        #[serde(default = "default_shape_size")]
        width: f32,
        #[serde(default = "default_shape_size")]
        height: f32,
        #[serde(default = "default_shape_thickness")]
        thickness: f32,
        #[serde(default = "default_shape_color")]
        color: [u8; 4],
        #[serde(default)]
        filled: bool,
        #[serde(default = "default_shape_fill_color")]
        fill_color: [u8; 4],
        #[serde(default = "default_shape_corner_radius")]
        corner_radius: f32,
        #[serde(default = "default_true")]
        lock_aspect: bool,
        #[serde(default)]
        rotation: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_click: Option<OverlayClickAction>,
    },
}

impl OverlayElement {
    pub fn rotation(&self) -> f32 {
        match self {
            OverlayElement::Text { rotation, .. }
            | OverlayElement::Crosshair { rotation, .. }
            | OverlayElement::Image { rotation, .. }
            | OverlayElement::Shape { rotation, .. }
            | OverlayElement::ProgressBar { rotation, .. }
            | OverlayElement::Badge { rotation, .. } => *rotation,
        }
    }

    pub fn rotation_mut(&mut self) -> &mut f32 {
        match self {
            OverlayElement::Text { rotation, .. }
            | OverlayElement::Crosshair { rotation, .. }
            | OverlayElement::Image { rotation, .. }
            | OverlayElement::Shape { rotation, .. }
            | OverlayElement::ProgressBar { rotation, .. }
            | OverlayElement::Badge { rotation, .. } => rotation,
        }
    }

    pub fn x(&self) -> f32 {
        match self {
            OverlayElement::Text { x, .. }
            | OverlayElement::Crosshair { x, .. }
            | OverlayElement::Image { x, .. }
            | OverlayElement::Shape { x, .. }
            | OverlayElement::ProgressBar { x, .. }
            | OverlayElement::Badge { x, .. } => *x,
        }
    }

    pub fn y(&self) -> f32 {
        match self {
            OverlayElement::Text { y, .. }
            | OverlayElement::Crosshair { y, .. }
            | OverlayElement::Image { y, .. }
            | OverlayElement::Shape { y, .. }
            | OverlayElement::ProgressBar { y, .. }
            | OverlayElement::Badge { y, .. } => *y,
        }
    }

    pub fn x_mut(&mut self) -> &mut f32 {
        match self {
            OverlayElement::Text { x, .. }
            | OverlayElement::Crosshair { x, .. }
            | OverlayElement::Image { x, .. }
            | OverlayElement::Shape { x, .. }
            | OverlayElement::ProgressBar { x, .. }
            | OverlayElement::Badge { x, .. } => x,
        }
    }

    pub fn y_mut(&mut self) -> &mut f32 {
        match self {
            OverlayElement::Text { y, .. }
            | OverlayElement::Crosshair { y, .. }
            | OverlayElement::Image { y, .. }
            | OverlayElement::Shape { y, .. }
            | OverlayElement::ProgressBar { y, .. }
            | OverlayElement::Badge { y, .. } => y,
        }
    }
}

pub fn default_shape_size() -> f32 {
    48.0
}
pub fn default_shape_thickness() -> f32 {
    2.0
}
pub fn default_shape_color() -> [u8; 4] {
    [80, 160, 255, 255]
}
pub fn default_shape_fill_color() -> [u8; 4] {
    [80, 160, 255, 60]
}
pub fn default_shape_corner_radius() -> f32 {
    4.0
}

pub fn default_font_family() -> String {
    "Segoe UI".to_string()
}
pub fn default_font_size() -> f32 {
    13.0
}
pub fn default_white_color() -> [u8; 4] {
    [240, 242, 248, 255]
}
pub fn default_image_size() -> f32 {
    32.0
}
pub fn default_opacity_one() -> f32 {
    1.0
}
pub fn default_crosshair_size() -> f32 {
    14.0
}
pub fn default_crosshair_color() -> [u8; 4] {
    [0, 255, 136, 255]
}
pub fn default_crosshair_thickness() -> f32 {
    2.0
}
pub fn default_crosshair_gap() -> f32 {
    4.0
}
pub fn default_accent_color() -> [u8; 4] {
    [70, 160, 255, 255]
}
pub fn default_dark_color() -> [u8; 4] {
    [30, 32, 40, 220]
}
pub fn default_badge_bg() -> [u8; 4] {
    [70, 160, 255, 45]
}
pub fn default_progress_width() -> f32 {
    180.0
}
pub fn default_progress_height() -> f32 {
    8.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub rect: OverlayRect,
    #[serde(default)]
    pub anchor: OverlayAnchor,
    #[serde(default)]
    pub click_through: bool,
    #[serde(default)]
    pub draggable: bool,
    #[serde(default = "default_opacity_one")]
    pub opacity: f32,
    #[serde(default)]
    pub style: OverlayStyle,
    #[serde(default)]
    pub elements: Vec<OverlayElement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_hide_seconds: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_process: Option<String>,
    #[serde(default)]
    pub hide_from_capture: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out_ms: Option<u32>,
}

impl Default for OverlayDefinition {
    fn default() -> Self {
        Self::default_new_overlay()
    }
}

impl OverlayDefinition {
    pub fn default_new_overlay() -> Self {
        Self {
            id: "overlay_custom".to_string(),
            name: "Custom Overlay".to_string(),
            rect: OverlayRect { x: 0.0, y: 0.0, width: 440.0, height: 200.0 },
            anchor: OverlayAnchor::ScreenCenter,
            click_through: false,
            draggable: false,
            opacity: 0.95,
            style: OverlayStyle {
                bg_color: [14, 15, 20, 230],
                border_color: [80, 160, 255, 120],
                border_width: 1.0,
                corner_radius: 8.0,
                glow: true,
            },
            elements: vec![],
            auto_hide_seconds: None,
            target_process: None,
            hide_from_capture: false,
            fade_in_ms: None,
            fade_out_ms: None,
        }
    }

    pub fn default_crosshair() -> Self {
        Self {
            id: "overlay_crosshair".to_string(),
            name: "Gaming Crosshair".to_string(),
            rect: OverlayRect { x: 0.0, y: 0.0, width: 48.0, height: 48.0 },
            anchor: OverlayAnchor::ScreenCenter,
            click_through: true,
            draggable: false,
            opacity: 1.0,
            style: OverlayStyle {
                bg_color: [0, 0, 0, 0],
                border_color: [0, 0, 0, 0],
                border_width: 0.0,
                corner_radius: 0.0,
                glow: false,
            },
            elements: vec![
                OverlayElement::Crosshair {
                    id: "cross_1".to_string(),
                    style: CrosshairStyle::Cross,
                    size: 14.0,
                    color: [0, 255, 136, 255],
                    center_dot: true,
                    thickness: 2.0,
                    gap: 4.0,
                    x: 24.0,
                    y: 24.0,
                    rotation: 0.0,
                }
            ],
            auto_hide_seconds: None,
            target_process: None,
            hide_from_capture: false,
            fade_in_ms: None,
            fade_out_ms: None,
        }
    }

    pub fn default_macro_badge() -> Self {
        Self {
            id: "overlay_macro_badge".to_string(),
            name: "Macro Status Badge".to_string(),
            rect: OverlayRect { x: 24.0, y: 24.0, width: 220.0, height: 44.0 },
            anchor: OverlayAnchor::TopRight,
            click_through: false,
            draggable: false,
            opacity: 0.95,
            style: OverlayStyle {
                bg_color: [14, 15, 20, 230],
                border_color: [80, 160, 255, 120],
                border_width: 1.0,
                corner_radius: 6.0,
                glow: true,
            },
            elements: vec![
                OverlayElement::Text {
                    id: "txt_badge".to_string(),
                    text: "MACRO ACTIVE".to_string(),
                    font_size: 13.0,
                    color: [80, 160, 255, 255],
                    x: 14.0,
                    y: 12.0,
                    width: None,
                    height: None,
                    lock_aspect: true,
                    bold: true,
                    font_family: "Segoe UI".to_string(),
                    rotation: 0.0,
                    on_click: None,
                },
                OverlayElement::Text {
                    id: "txt_hint".to_string(),
                    text: "[HOLD]".to_string(),
                    font_size: 11.5,
                    color: [160, 170, 190, 255],
                    x: 140.0,
                    y: 13.0,
                    width: None,
                    height: None,
                    lock_aspect: true,
                    bold: true,
                    font_family: "Segoe UI".to_string(),
                    rotation: 0.0,
                    on_click: None,
                }
            ],
            auto_hide_seconds: None,
            target_process: None,
            hide_from_capture: false,
            fade_in_ms: None,
            fade_out_ms: None,
        }
    }

    pub fn default_telemetry() -> Self {
        Self {
            id: "overlay_telemetry".to_string(),
            name: "Process Telemetry HUD".to_string(),
            rect: OverlayRect { x: 24.0, y: 80.0, width: 260.0, height: 74.0 },
            anchor: OverlayAnchor::TopRight,
            click_through: false,
            draggable: false,
            opacity: 0.9,
            style: OverlayStyle {
                bg_color: [16, 17, 24, 220],
                border_color: [255, 255, 255, 30],
                border_width: 1.0,
                corner_radius: 6.0,
                glow: false,
            },
            elements: vec![
                OverlayElement::Text {
                    id: "proc_title".to_string(),
                    text: "{ProcessName}".to_string(),
                    font_size: 13.0,
                    color: [255, 255, 255, 255],
                    x: 14.0,
                    y: 12.0,
                    width: None,
                    height: None,
                    lock_aspect: true,
                    bold: true,
                    font_family: "Segoe UI".to_string(),
                    rotation: 0.0,
                    on_click: None,
                },
                OverlayElement::Text {
                    id: "proc_stats".to_string(),
                    text: "Time: {Time} | Active".to_string(),
                    font_size: 11.5,
                    color: [140, 150, 170, 255],
                    x: 14.0,
                    y: 38.0,
                    width: None,
                    height: None,
                    lock_aspect: true,
                    bold: false,
                    font_family: "Segoe UI".to_string(),
                    rotation: 0.0,
                    on_click: None,
                }
            ],
            auto_hide_seconds: None,
            target_process: None,
            hide_from_capture: false,
            fade_in_ms: None,
            fade_out_ms: None,
        }
    }
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
            last_executed_at: None,
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
    fn test_action_duplicate_with_fresh_ids() {
        let overlay_act = Action::Overlay {
            overlay_id: "overlay_initial_123".to_string(),
            custom_overlay: Some(Box::new(OverlayDefinition {
                id: "overlay_initial_123".to_string(),
                ..Default::default()
            })),
            operation: OverlayOperation::Toggle,
            override_text: None,
            duration_ms: None,
        };

        let duplicated = overlay_act.duplicate_with_fresh_ids();
        if let Action::Overlay { overlay_id, custom_overlay, .. } = duplicated {
            assert_ne!(overlay_id, "overlay_initial_123");
            assert_eq!(custom_overlay.as_ref().map(|o| o.id.as_str()), Some(overlay_id.as_str()));
        } else {
            panic!("Expected Action::Overlay");
        }
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
            last_executed_at: None,
        };

        // Fallback when empty
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::GEAR);

        // Manual trigger icon
        rule.trigger = Some(Trigger::Manual);
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::PLAY);

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

        rule.actions = vec![Action::ExecuteScript {
            interpreter: ScriptType::PowerShell,
            script: "echo 1".into(),
            run_in_window: false,
        }];
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::CODE);

        rule.actions = vec![Action::Delay { seconds: 5.0 }];
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::HOURGLASS);

        rule.actions = vec![Action::AudioControl {
            target: AudioTarget::Microphone,
            process_name: None,
            action_type: AudioVolumeAction::Mute,
            volume_level: 100,
            left_volume: None,
            right_volume: None,
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
                    play_beep: false,
                    beep_frequency: None,
                    beep_duration_ms: None,
                },
                Action::ExecuteScript {
                    interpreter: ScriptType::Lua,
                    script: "ph.log('info', 'Workstation setup complete!')".into(),
                    run_in_window: false,
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
                Action::DefaultDevice {
                    target: AudioTarget::Playback,
                    device_name: "Headphones".into(),
                },
                Action::DefaultDevice {
                    target: AudioTarget::Microphone,
                    device_name: "Microphone".into(),
                },
                Action::IfElse {
                    conditions: vec![RuleConditionItem::new(RuleCondition::ProcessRunning {
                        process_name: "game.exe".into(),
                        match_mode: ProcessMatchMode::Exact,
                        must_be_running: true,
                    })],
                    then_actions: vec![Action::Delay { seconds: 1.0 }],
                    else_if_branches: vec![ElseIfBranch {
                        conditions: vec![RuleConditionItem::new(RuleCondition::Event {
                            event_type: SystemEventType::PowerSourceAc,
                            idle_duration_sec: 300,
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
                    RuleCondition::FileWatcher {
                        path: "C:\\log.txt".into(),
                        filter_extension: None,
                        must_exist: true,
                    },
                ),
            ],
            last_executed_at: None,
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
    fn test_unified_actions_serde() {
        let unified_actions = vec![
            Action::AudioControl {
                target: AudioTarget::Playback,
                process_name: None,
                action_type: AudioVolumeAction::SetVolume,
                volume_level: 75,
                left_volume: Some(70),
                right_volume: Some(80),
            },
            Action::AudioFormat {
                target: AudioTarget::Playback,
                device_name: None,
                sample_rate: 48000,
                bit_depth: 24,
                channels: 2,
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
    }

    #[test]
    fn test_new_v23_actions_serde() {
        let actions = vec![
            Action::TrimWorkingSet {
                target_process: Some("game.exe".to_string()),
            },
            Action::DiscordRpcSet {
                app_id: "123456789".to_string(),
                details: "Playing Game".to_string(),
                state: "In Mission".to_string(),
                large_image: Some("logo".to_string()),
                large_text: Some("Game Logo".to_string()),
                small_image: Some("status".to_string()),
                small_text: Some("Online".to_string()),
                show_elapsed: true,
                buttons: vec![DiscordRpcButton {
                    label: "Join".to_string(),
                    url: "https://example.com".to_string(),
                }],
                auto_clear_on_process_exit: true,
            },
            Action::DiscordRpcClear,
            Action::Archive {
                operation: ArchiveOperation::Pack,
                format: ArchiveFormat::Zip,
                source_path: "C:\\source".to_string(),
                destination_path: "C:\\out.zip".to_string(),
                compression_level: Some(6),
                overwrite: true,
            },
            Action::FileSystem {
                operation: FileSystemOperation::CreateFile,
                path: "C:\\file.txt".to_string(),
                target_path: None,
                content: Some("text".to_string()),
                create_parents: true,
                overwrite: true,
            },
        ];

        let json = serde_json::to_string(&actions).expect("Must serialize");
        let decoded: Vec<Action> = serde_json::from_str(&json).expect("Must deserialize");
        assert_eq!(decoded, actions);
    }

    #[test]
    fn test_manual_trigger_serde() {
        let trigger = Trigger::Manual;
        let serialized = serde_json::to_string(&trigger).unwrap();
        assert_eq!(serialized, r#"{"$trigger_type":"manual"}"#);
        let deserialized: Trigger = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, Trigger::Manual);

        let rule = Rule {
            trigger: Some(Trigger::Manual),
            ..Default::default()
        };
        assert_eq!(rule.default_icon_glyph(), egui_phosphor::regular::PLAY);
    }

    #[test]
    fn test_global_hotkey_hold_mode_and_repeat_serde() {
        // Test optional field default: json without repeat_while_held defaults to false
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

    #[test]
    fn test_precondition_resource_and_event_serde() {
        // 1. ResourceThreshold roundtrip
        let res_cond = RuleCondition::ResourceThreshold {
            process_name: "game.exe".to_string(),
            cpu_percent: Some(75.5),
            ram_mb: Some(2048),
            duration_sec: 10,
        };
        let serialized = serde_json::to_string(&res_cond).unwrap();
        let deserialized: RuleCondition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(res_cond, deserialized);

        // 2. Event condition roundtrip
        let event_cond = RuleCondition::Event {
            event_type: SystemEventType::SessionLock,
            idle_duration_sec: 300,
        };
        let serialized = serde_json::to_string(&event_cond).unwrap();
        let deserialized: RuleCondition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event_cond, deserialized);

        // 3. FileWatcher condition roundtrip
        let file_cond = RuleCondition::FileWatcher {
            path: "C:\\test.log".to_string(),
            filter_extension: Some("log".to_string()),
            must_exist: true,
        };
        let serialized = serde_json::to_string(&file_cond).unwrap();
        let deserialized: RuleCondition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(file_cond, deserialized);
    }

    #[test]
    fn test_overlay_models_and_action_serde() {
        let action = Action::Overlay {
            overlay_id: "overlay_crosshair".to_string(),
            custom_overlay: None,
            operation: OverlayOperation::Toggle,
            override_text: Some("Aim Active".to_string()),
            duration_ms: Some(3000),
        };
        let json = serde_json::to_string(&action).expect("Serialize Action::Overlay");
        let decoded: Action = serde_json::from_str(&json).expect("Deserialize Action::Overlay");
        assert_eq!(decoded, action);

        // Test with custom embedded overlay
        let custom_action = Action::Overlay {
            overlay_id: "custom_rule_hud".to_string(),
            custom_overlay: Some(Box::new(OverlayDefinition::default_crosshair())),
            operation: OverlayOperation::Show,
            override_text: None,
            duration_ms: None,
        };
        let custom_json = serde_json::to_string(&custom_action).unwrap();
        let custom_decoded: Action = serde_json::from_str(&custom_json).unwrap();
        assert_eq!(custom_decoded, custom_action);

        let overlay = OverlayDefinition::default_crosshair();
        let json_ov = serde_json::to_string(&overlay).expect("Serialize OverlayDefinition");
        let decoded_ov: OverlayDefinition = serde_json::from_str(&json_ov).expect("Deserialize OverlayDefinition");
        assert_eq!(decoded_ov, overlay);

        let macro_ov = OverlayDefinition::default_macro_badge();
        let json_macro = serde_json::to_string(&macro_ov).unwrap();
        let decoded_macro: OverlayDefinition = serde_json::from_str(&json_macro).unwrap();
        assert_eq!(decoded_macro, macro_ov);

        // Test Image element defaults: lock_aspect defaults to true
        let raw_img_json = r#"{"type": "image", "path": "test.png"}"#;
        let img_elem: OverlayElement = serde_json::from_str(raw_img_json).unwrap();
        if let OverlayElement::Image { lock_aspect, .. } = img_elem {
            assert!(lock_aspect, "lock_aspect must default to true");
        } else {
            panic!("Expected Image element");
        }

        // Test fade_in_ms and fade_out_ms serde
        let mut fade_overlay = OverlayDefinition::default_crosshair();
        fade_overlay.fade_in_ms = Some(250);
        fade_overlay.fade_out_ms = Some(350);
        let fade_json = serde_json::to_string(&fade_overlay).unwrap();
        assert!(fade_json.contains("\"fade_in_ms\":250"));
        assert!(fade_json.contains("\"fade_out_ms\":350"));
        let decoded_fade: OverlayDefinition = serde_json::from_str(&fade_json).unwrap();
        assert_eq!(decoded_fade.fade_in_ms, Some(250));
        assert_eq!(decoded_fade.fade_out_ms, Some(350));

        // When omitted, both must default to None and click_through to false
        let raw_no_fade = r#"{"id":"test","name":"Test"}"#;
        let decoded_no_fade: OverlayDefinition = serde_json::from_str(raw_no_fade).unwrap();
        assert_eq!(decoded_no_fade.fade_in_ms, None);
        assert_eq!(decoded_no_fade.fade_out_ms, None);
        assert!(!decoded_no_fade.click_through, "click_through must default to false");
        assert!(!OverlayDefinition::default_new_overlay().click_through, "new overlay click_through must be false by default");
    }
}
