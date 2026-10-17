use serde::{Deserialize, Serialize};
use crate::config::AppConfig;
use crate::models::{LogItem, ProcessSnapshotItem, Rule};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcRequest {
    Ping,
    GetState,
    SaveConfig(AppConfig),
    SaveRule(Rule),
    DeleteRule(String),
    ToggleRule { id: String, enabled: bool },
    TestRunRule(String),
    GetProcesses,
    KillProcess { pid: u32, force: bool },
    SetAutorun(bool),
    UpdateWindowState(crate::config::WindowState),
    ClearLogs,
    ShutdownDaemon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcResponse {
    Ok,
    Pong,
    State(DaemonState),
    Processes(Vec<ProcessSnapshotItem>),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonState {
    pub autorun_enabled: bool,
    pub uptime_secs: u64,
    pub daemon_memory_bytes: u64,
    pub daemon_cpu_usage: f32,
    pub active_rules_count: usize,
    pub total_rules_count: usize,
    pub config: AppConfig,
    pub logs: Vec<LogItem>,
}
