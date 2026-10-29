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
    TestRunRule {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rule: Option<Box<Rule>>,
    },
    StopRuleExecution(String),
    TestStep { action: Action, context: std::collections::HashMap<String, String> },
    GetProcesses,
    KillProcess { pid: u32, force: bool },
    SetAutorun(bool),
    UpdateWindowState(crate::config::WindowState),
    ClearLogs,
    ShutdownDaemon,
    ImportRules(Vec<Rule>),
    SaveOverlay(crate::models::OverlayDefinition),
    DeleteOverlay(String),
    ShowOverlayPreview { overlay: crate::models::OverlayDefinition, calibration_mode: bool },
    HideOverlayPreview(String),
    GetOverlayPosition(String),
    UpdateOverlayPosition { id: String, x: f32, y: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
#[allow(clippy::large_enum_variant)]
pub enum IpcResponse {
    Ok,
    Pong,
    State(DaemonState),
    Processes(Vec<ProcessSnapshotItem>),
    TestStepResult(Result<(), String>),
    OverlayPosition(Option<(f32, f32)>),
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
    pub active_overlay_ids: std::collections::HashSet<String>,
    #[serde(default)]
    pub active_test_rule_ids: std::collections::HashSet<String>,
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
    fn test_ipc_overlay_position_serde() {
        let req = IpcRequest::GetOverlayPosition("test_ov".into());
        let json_req = serde_json::to_string(&req).unwrap();
        let dec_req: IpcRequest = serde_json::from_str(&json_req).unwrap();
        match dec_req {
            IpcRequest::GetOverlayPosition(id) => assert_eq!(id, "test_ov"),
            _ => panic!("wrong req variant"),
        }

        let resp = IpcResponse::OverlayPosition(Some((100.0, 200.0)));
        let json_resp = serde_json::to_string(&resp).unwrap();
        let dec_resp: IpcResponse = serde_json::from_str(&json_resp).unwrap();
        match dec_resp {
            IpcResponse::OverlayPosition(Some((x, y))) => {
                assert_eq!(x, 100.0);
                assert_eq!(y, 200.0);
            }
            _ => panic!("wrong resp variant"),
        }
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

    #[test]
    fn test_ipc_test_run_and_stop_serde() {
        let req = IpcRequest::TestRunRule {
            id: "rule_1".into(),
            rule: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let dec: IpcRequest = serde_json::from_str(&json).unwrap();
        match dec {
            IpcRequest::TestRunRule { id, rule } => {
                assert_eq!(id, "rule_1");
                assert!(rule.is_none());
            }
            _ => panic!("wrong variant"),
        }

        let stop_req = IpcRequest::StopRuleExecution("rule_1".into());
        let json_stop = serde_json::to_string(&stop_req).unwrap();
        let dec_stop: IpcRequest = serde_json::from_str(&json_stop).unwrap();
        match dec_stop {
            IpcRequest::StopRuleExecution(id) => assert_eq!(id, "rule_1"),
            _ => panic!("wrong variant"),
        }
    }
}
