use serde::{Deserialize, Serialize};
use crate::config::AppConfig;
use crate::models::{Action, AnalyticsSummary, LogItem, ProcessSnapshotItem, Rule};

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
    TestStep { action: Action, context: std::collections::HashMap<String, String> },
    GetProcesses,
    KillProcess { pid: u32, force: bool },
    SetAutorun(bool),
    UpdateWindowState(crate::config::WindowState),
    ClearLogs,
    ShutdownDaemon,
    ImportRules(Vec<Rule>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcResponse {
    Ok,
    Pong,
    State(DaemonState),
    Processes(Vec<ProcessSnapshotItem>),
    TestStepResult(Result<(), String>),
    Error(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ActiveRuleExecutionState {
    pub active_count: usize,
    pub current_step: usize,
    pub total_steps: usize,
    pub is_exit: bool,
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
    #[serde(default)]
    pub active_rule_executions: std::collections::HashMap<String, ActiveRuleExecutionState>,
    #[serde(default)]
    pub analytics: AnalyticsSummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_rule_execution_state_copy_and_serde() {
        let state = ActiveRuleExecutionState {
            active_count: 3,
            current_step: 1,
            total_steps: 4,
            is_exit: false,
        };
        let copied = state;
        assert_eq!(copied.active_count, 3);
        assert_eq!(copied.current_step, 1);
        assert_eq!(copied.total_steps, 4);
        assert!(!copied.is_exit);

        let json = serde_json::to_string(&state).unwrap();
        let decoded: ActiveRuleExecutionState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, state);
    }

    #[test]
    fn test_ipc_import_rules_request_serde() {
        let req = IpcRequest::ImportRules(vec![Rule {
            id: "r_test".into(),
            name: "Imported Rule".into(),
            description: "".into(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: None,
            actions: vec![],
            on_exit_actions: vec![],
            custom_icon_path: Some("C:\\test.ico".into()),
            auto_process_icon: true,
            ..Default::default()
        }]);

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("ImportRules"));
        assert!(json.contains("Imported Rule"));
    }
}
