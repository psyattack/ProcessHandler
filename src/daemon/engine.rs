use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tokio::sync::mpsc;
use crate::config::AppConfig;
use crate::ipc::{DaemonState, IpcRequest, IpcResponse};
use crate::models::{Action, LogItem, ProcessMatchMode, Rule, ScheduleType, SystemEventType, Trigger};
use crate::daemon::actions::ActionRunner;
use crate::daemon::watcher::{ProcessEvent, ProcessWatcher};

#[derive(Debug, Clone)]
pub struct BoundProcess {
    pub rule_id: String,
    pub primary_process_name: String,
    pub primary_pid: u32,
    pub target_executable: String,
}

#[derive(Debug)]
pub enum ActionCompletionEvent {
    Log {
        level: String,
        category: String,
        message: String,
    },
    AddBoundProcess(BoundProcess),
}

pub struct DaemonEngine {
    pub config: AppConfig,
    pub start_time: Instant,
    pub logs: Vec<LogItem>,
    pub watcher: ProcessWatcher,
    pub active_bound_processes: Vec<BoundProcess>,
    pub rule_last_triggered: HashMap<String, Instant>,
    pub active_rule_pids: HashMap<String, HashSet<u32>>,
    action_tx: mpsc::UnboundedSender<ActionCompletionEvent>,
    action_rx: mpsc::UnboundedReceiver<ActionCompletionEvent>,
}

impl DaemonEngine {
    pub const MAX_LOGS: usize = 100;

    pub fn new(config: AppConfig) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Self {
            config,
            start_time: Instant::now(),
            logs: Vec::new(),
            watcher: ProcessWatcher::new(),
            active_bound_processes: Vec::new(),
            rule_last_triggered: HashMap::new(),
            active_rule_pids: HashMap::new(),
            action_tx,
            action_rx,
        }
    }

    pub fn drain_action_events(&mut self) {
        while let Ok(event) = self.action_rx.try_recv() {
            match event {
                ActionCompletionEvent::Log { level, category, message } => {
                    self.log(&level, &category, &message);
                }
                ActionCompletionEvent::AddBoundProcess(bp) => {
                    self.active_bound_processes.push(bp);
                }
            }
        }
    }

    pub fn log(&mut self, level: &str, category: &str, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        while self.logs.len() >= Self::MAX_LOGS {
            self.logs.remove(0);
        }
        self.logs.push(LogItem {
            timestamp,
            level: level.to_string(),
            category: category.to_string(),
            message: message.to_string(),
        });
    }

    pub async fn tick_process_events(&mut self) {
        self.drain_action_events();
        let events = self.watcher.poll_changes();
        for event in events {
            self.handle_process_event(event).await;
        }
    }

    pub fn matches_process(pattern: &str, actual: &str, mode: ProcessMatchMode) -> bool {
        if pattern == "*" {
            return true;
        }
        match mode {
            ProcessMatchMode::Exact => pattern.eq_ignore_ascii_case(actual),
            ProcessMatchMode::Wildcard => {
                let pat = pattern.to_lowercase();
                let act = actual.to_lowercase();
                if pat.starts_with('*') && pat.ends_with('*') && pat.len() >= 2 {
                    act.contains(&pat[1..pat.len() - 1])
                } else if let Some(stripped) = pat.strip_prefix('*') {
                    act.ends_with(stripped)
                } else if let Some(stripped) = pat.strip_suffix('*') {
                    act.starts_with(stripped)
                } else {
                    act == pat
                }
            }
            ProcessMatchMode::Regex => {
                actual.to_lowercase().contains(&pattern.to_lowercase())
            }
        }
    }

    pub async fn handle_process_event(&mut self, event: ProcessEvent) {
        let action_str = if event.is_started { "started" } else { "stopped" };
        self.log(
            "INFO",
            "PROCESS",
            &format!("Process {} (PID: {}) {}", event.process_name, event.pid, action_str),
        );

        if event.is_started {
            // Check rules on Process Started
            let rules = self.config.rules.clone();
            for rule in &rules {
                if !rule.is_enabled {
                    continue;
                }

                if let Some(Trigger::Process { process_name, match_mode, single_instance }) = &rule.trigger {
                    if Self::matches_process(process_name, &event.process_name, *match_mode) {
                        if *single_instance {
                            let pids = self.active_rule_pids.entry(rule.id.clone()).or_default();
                            let is_first = pids.is_empty();
                            pids.insert(event.pid);

                            if is_first {
                                if !rule.actions.is_empty() {
                                    let _ = self.try_execute_rule(rule, &event.process_name, event.pid).await;
                                }
                            } else if !rule.actions.is_empty() {
                                self.log(
                                    "INFO",
                                    "RULE",
                                    &format!(
                                        "Rule '{}' skipped (Single Instance): another instance of {} is already running (PID: {})",
                                        rule.name, event.process_name, event.pid
                                    ),
                                );
                            }
                        } else if !rule.actions.is_empty() {
                            let _ = self.try_execute_rule(rule, &event.process_name, event.pid).await;
                        }
                    }
                }
            }
        } else {
            // Process STOPPED:
            let rules = self.config.rules.clone();
            let rules_map: HashMap<String, Rule> = rules.iter().map(|r| (r.id.clone(), r.clone())).collect();

            // Track which single_instance rules have completely terminated (all instances closed)
            let mut single_instance_rules_exited: HashSet<String> = HashSet::new();
            for rule in &rules {
                if let Some(Trigger::Process { process_name, match_mode, single_instance }) = &rule.trigger {
                    if *single_instance && Self::matches_process(process_name, &event.process_name, *match_mode) {
                        if let Some(pids) = self.active_rule_pids.get_mut(&rule.id) {
                            pids.remove(&event.pid);
                            if pids.is_empty() {
                                single_instance_rules_exited.insert(rule.id.clone());
                            }
                        }
                    }
                }
            }
            for rule_id in &single_instance_rules_exited {
                self.active_rule_pids.remove(rule_id);
            }

            // 1. Auto-close any bound processes that were launched for this process (Companion Mode in Rules)
            let to_kill: Vec<BoundProcess> = self
                .active_bound_processes
                .iter()
                .filter(|bp| {
                    let is_match = bp.primary_pid == event.pid
                        || bp.primary_process_name.eq_ignore_ascii_case(&event.process_name);
                    if !is_match {
                        return false;
                    }
                    // If the rule has single_instance enabled, only kill if all instances have exited
                    if let Some(rule) = rules_map.get(&bp.rule_id) {
                        if let Some(Trigger::Process { single_instance: true, .. }) = &rule.trigger {
                            return single_instance_rules_exited.contains(&bp.rule_id);
                        }
                    }
                    true
                })
                .cloned()
                .collect();

            self.active_bound_processes.retain(|bp| {
                !to_kill.iter().any(|k| k.rule_id == bp.rule_id && k.target_executable == bp.target_executable)
            });

            for bp in to_kill {
                let target_name = std::path::Path::new(&bp.target_executable)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(&bp.target_executable)
                    .to_string();

                self.log(
                    "ACTION",
                    "RULE",
                    &format!(
                        "Auto-closing bound process '{}' (rule: {}) because '{}' terminated",
                        target_name, bp.rule_id, event.process_name
                    ),
                );

                let action = Action::KillProcess {
                    process_name: target_name,
                    kill_tree: true,
                    force: true,
                };
                let ctx = HashMap::new();
                let _ = ActionRunner::execute(&action, &ctx).await;
            }

            // 2. Check rules with on_exit_actions
            for rule in &rules {
                if !rule.is_enabled || rule.on_exit_actions.is_empty() {
                    continue;
                }

                if let Some(Trigger::Process { process_name, match_mode, single_instance }) = &rule.trigger {
                    if Self::matches_process(process_name, &event.process_name, *match_mode) {
                        if *single_instance {
                            if single_instance_rules_exited.contains(&rule.id) {
                                let _ = self.try_execute_exit_actions(rule, &event.process_name, event.pid).await;
                            }
                        } else {
                            let _ = self.try_execute_exit_actions(rule, &event.process_name, event.pid).await;
                        }
                    }
                }
            }
        }
    }

    pub async fn handle_system_event(&mut self, event: SystemEventType) {
        self.drain_action_events();
        let event_name = format!("{:?}", event);
        self.log("INFO", "SYSTEM", &format!("System Event: {}", event_name));

        let rules = self.config.rules.clone();
        for rule in &rules {
            if !rule.is_enabled {
                continue;
            }

            if let Some(Trigger::SystemEvent { event_type, .. }) = &rule.trigger {
                if *event_type == event {
                    let _ = self.try_execute_rule(rule, &event_name, 0).await;
                }
            }
        }
    }

    pub async fn tick_schedule(&mut self) {
        self.drain_action_events();
        let now_instant = Instant::now();
        let now_local = chrono::Local::now();
        let rules = self.config.rules.clone();

        for rule in &rules {
            if !rule.is_enabled {
                continue;
            }

            if let Some(Trigger::Schedule {
                schedule_type,
                interval_sec,
                daily_time,
                days_of_week,
                target_datetime,
                ..
            }) = &rule.trigger
            {
                match schedule_type {
                    ScheduleType::Interval => {
                        let interval = *interval_sec;
                        if interval == 0 {
                            continue;
                        }

                        // If a specific first execution date & time is set, ensure current time >= target
                        if let Some(target) = target_datetime {
                            let cur_datetime = now_local.format("%Y-%m-%d %H:%M:%S").to_string();
                            let target_cmp = if target.len() == 16 {
                                format!("{}:00", target)
                            } else {
                                target.clone()
                            };

                            if cur_datetime < target_cmp && !self.rule_last_triggered.contains_key(&rule.id) {
                                continue;
                            }
                        }

                        let should_run = match self.rule_last_triggered.get(&rule.id) {
                            Some(last) => last.elapsed().as_secs() >= interval,
                            None => {
                                if target_datetime.is_some() {
                                    true
                                } else {
                                    self.rule_last_triggered.insert(rule.id.clone(), now_instant);
                                    false
                                }
                            }
                        };

                        if should_run {
                            let _ = self.try_execute_rule(rule, &format!("Schedule Interval {}s", interval), 0).await;
                        }
                    }
                    ScheduleType::Daily => {
                        if let Some(t_str) = daily_time {
                            use chrono::Datelike;
                            let current_dow = now_local.weekday().number_from_monday() as u8;
                            if !days_of_week.is_empty() && !days_of_week.contains(&current_dow) {
                                continue;
                            }

                            let cur_time_hms = now_local.format("%H:%M:%S").to_string();
                            let cur_time_hm = now_local.format("%H:%M").to_string();
                            let target_matches = cur_time_hms == *t_str
                                || cur_time_hm == *t_str
                                || format!("{}:00", cur_time_hm) == *t_str;
                            if target_matches {
                                let already_ran = match self.rule_last_triggered.get(&rule.id) {
                                    Some(last) => last.elapsed().as_secs() < 3,
                                    None => false,
                                };
                                if !already_ran {
                                    let _ = self.try_execute_rule(rule, &format!("Daily Schedule {}", t_str), 0).await;
                                }
                            }
                        }
                    }
                    ScheduleType::OneShot => {
                        if let Some(target) = target_datetime {
                            let cur_datetime = now_local.format("%Y-%m-%d %H:%M:%S").to_string();
                            let target_cmp = if target.len() == 16 {
                                format!("{}:00", target)
                            } else {
                                target.clone()
                            };
                            if cur_datetime >= target_cmp {
                                let already_ran = self.rule_last_triggered.contains_key(&rule.id);
                                if !already_ran {
                                    let _ = self.try_execute_rule(rule, &format!("One-Shot Schedule {}", target), 0).await;
                                    if let Some(stored) = self.config.rules.iter_mut().find(|r| r.id == rule.id) {
                                        stored.is_enabled = false;
                                        let _ = self.config.save_to_file(AppConfig::config_path());
                                    }
                                }
                            }
                        }
                    }
                    ScheduleType::AppStartup => {
                        // Handled on initial startup
                    }
                    _ => {}
                }
            }
        }
    }

    pub async fn try_execute_rule(
        &mut self,
        rule: &Rule,
        context_name: &str,
        pid: u32,
    ) -> Option<tokio::task::JoinHandle<()>> {
        self.drain_action_events();

        if let Some(last) = self.rule_last_triggered.get(&rule.id) {
            if last.elapsed().as_secs_f64() < rule.cooldown_seconds && context_name != "MANUAL_TEST" {
                return None;
            }
        }

        self.rule_last_triggered.insert(rule.id.clone(), Instant::now());
        self.log("TRIGGER", "RULE", &format!("Rule '{}' triggered by {}", rule.name, context_name));

        if rule.actions.is_empty() {
            return None;
        }

        let mut ctx = HashMap::new();
        ctx.insert("ProcessName".into(), context_name.into());
        ctx.insert("ProcessId".into(), pid.to_string());
        ctx.insert("Timestamp".into(), chrono::Local::now().to_rfc3339());

        let actions = rule.actions.clone();
        let rule_name = rule.name.clone();
        let rule_id = rule.id.clone();
        let context_name = context_name.to_string();
        let tx = self.action_tx.clone();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let join_handle = handle.spawn(async move {
                for action in actions {
                    match ActionRunner::execute(&action, &ctx).await {
                        Ok(()) => {
                            if let Action::StartProcess { file_path, close_on_exit: true, .. } = &action {
                                let _ = tx.send(ActionCompletionEvent::AddBoundProcess(BoundProcess {
                                    rule_id: rule_id.clone(),
                                    primary_process_name: context_name.clone(),
                                    primary_pid: pid,
                                    target_executable: file_path.clone(),
                                }));
                            }
                        }
                        Err(err) => {
                            let _ = tx.send(ActionCompletionEvent::Log {
                                level: "ERROR".to_string(),
                                category: "ACTION".to_string(),
                                message: format!("Action failed in '{}': {}", rule_name, err),
                            });
                        }
                    }
                }
            });
            Some(join_handle)
        } else {
            None
        }
    }

    pub async fn try_execute_exit_actions(
        &mut self,
        rule: &Rule,
        context_name: &str,
        pid: u32,
    ) -> Option<tokio::task::JoinHandle<()>> {
        self.drain_action_events();
        self.log("TRIGGER", "RULE", &format!("Rule '{}' exit actions triggered by {}", rule.name, context_name));

        if rule.on_exit_actions.is_empty() {
            return None;
        }

        let mut ctx = HashMap::new();
        ctx.insert("ProcessName".into(), context_name.into());
        ctx.insert("ProcessId".into(), pid.to_string());
        ctx.insert("Timestamp".into(), chrono::Local::now().to_rfc3339());

        let actions = rule.on_exit_actions.clone();
        let rule_name = rule.name.clone();
        let tx = self.action_tx.clone();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let join_handle = handle.spawn(async move {
                for action in actions {
                    if let Err(err) = ActionRunner::execute(&action, &ctx).await {
                        let _ = tx.send(ActionCompletionEvent::Log {
                            level: "ERROR".to_string(),
                            category: "ACTION".to_string(),
                            message: format!("Exit action failed in '{}': {}", rule_name, err),
                        });
                    }
                }
            });
            Some(join_handle)
        } else {
            None
        }
    }

    pub async fn handle_request(&mut self, req: IpcRequest) -> IpcResponse {
        self.drain_action_events();
        match req {
            IpcRequest::Ping => IpcResponse::Pong,

            IpcRequest::GetState => {
                let state = self.get_current_state();
                IpcResponse::State(state)
            }


            IpcRequest::SaveConfig(cfg) => {
                let tray_changed = self.config.tray_icon_path != cfg.tray_icon_path;
                let new_tray_path = cfg.tray_icon_path.clone();
                self.config = cfg;
                let _ = self.config.save_to_file(AppConfig::config_path());
                if tray_changed {
                    crate::daemon::tray::update_tray_icon(new_tray_path);
                }
                self.log("INFO", "CONFIG", "Configuration updated and saved");
                IpcResponse::Ok
            }

            IpcRequest::SaveRule(rule) => {
                if let Some(existing) = self.config.rules.iter_mut().find(|r| r.id == rule.id) {
                    *existing = rule;
                } else {
                    self.config.rules.push(rule);
                }
                let _ = self.config.save_to_file(AppConfig::config_path());
                IpcResponse::Ok
            }

            IpcRequest::DeleteRule(id) => {
                self.config.rules.retain(|r| r.id != id);
                let _ = self.config.save_to_file(AppConfig::config_path());
                IpcResponse::Ok
            }

            IpcRequest::ToggleRule { id, enabled } => {
                if let Some(rule) = self.config.rules.iter_mut().find(|r| r.id == id) {
                    rule.is_enabled = enabled;
                    let _ = self.config.save_to_file(AppConfig::config_path());
                }
                IpcResponse::Ok
            }

            IpcRequest::TestRunRule(id) => {
                if let Some(rule) = self.config.rules.iter().find(|r| r.id == id).cloned() {
                    let _ = self.try_execute_rule(&rule, "MANUAL_TEST", 0).await;
                    IpcResponse::Ok
                } else {
                    IpcResponse::Error("Rule not found".into())
                }
            }

            IpcRequest::GetProcesses => {
                let list = ProcessWatcher::get_detailed_snapshot();
                IpcResponse::Processes(list)
            }

            IpcRequest::KillProcess { pid, force } => {
                let mut cmd = std::process::Command::new("taskkill");
                if force {
                    cmd.arg("/F");
                }
                cmd.args(["/PID", &pid.to_string()]);
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    cmd.creation_flags(0x08000000);
                }
                let _ = cmd.spawn();
                IpcResponse::Ok
            }

            IpcRequest::SetAutorun(enable) => {
                match crate::autorun::set_autorun(crate::autorun::APP_NAME, enable) {
                    Ok(()) => {
                        self.config.autorun_configured = true;
                        let _ = self.config.save_to_file(AppConfig::config_path());
                        self.log("INFO", "CONFIG", &format!("Autorun set to: {enable}"));
                        IpcResponse::Ok
                    }
                    Err(err) => {
                        self.log("ERROR", "CONFIG", &format!("Failed to set autorun: {err}"));
                        IpcResponse::Error(err)
                    }
                }
            }

            IpcRequest::UpdateWindowState(ws) => {
                self.config.window = Some(ws);
                let _ = self.config.save_to_file(AppConfig::config_path());
                IpcResponse::Ok
            }

            IpcRequest::ClearLogs => {
                self.logs.clear();
                IpcResponse::Ok
            }

            IpcRequest::ShutdownDaemon => {
                self.execute_shutdown_rules().await;
                std::process::exit(0);
            }
        }
    }

    pub async fn execute_shutdown_rules(&mut self) {
        let rules = self.config.rules.clone();
        let mut handles = Vec::new();
        for rule in &rules {
            if rule.is_enabled {
                if let Some(Trigger::Schedule {
                    schedule_type: ScheduleType::AppShutdown,
                    ..
                }) = &rule.trigger
                {
                    if let Some(handle) = self.try_execute_rule(rule, "App Shutdown", 0).await {
                        handles.push(handle);
                    }
                }
            }
        }
        for handle in handles {
            let _ = handle.await;
        }
        self.drain_action_events();
    }

    pub fn get_current_state(&mut self) -> DaemonState {
        self.drain_action_events();
        let active_rules = self.config.rules.iter().filter(|r| r.is_enabled).count();
        let total_rules = self.config.rules.len();

        let mem = Self::get_daemon_memory();
        let autorun_enabled = crate::autorun::is_autorun_enabled(crate::autorun::APP_NAME);

        DaemonState {
            autorun_enabled,
            uptime_secs: self.start_time.elapsed().as_secs(),
            daemon_memory_bytes: mem,
            daemon_cpu_usage: 0.0,
            active_rules_count: active_rules,
            total_rules_count: total_rules,
            config: self.config.clone(),
            logs: self.logs.clone(),
        }
    }

    fn get_daemon_memory() -> u64 {
        #[cfg(windows)]
        unsafe {
            use windows::Win32::System::Threading::GetCurrentProcess;
            use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
            let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb).as_bool() {
                return pmc.WorkingSetSize as u64;
            }
        }
        2 * 1024 * 1024 // ~2MB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_log_limit_100() {
        let mut engine = DaemonEngine::new(AppConfig::default());
        assert_eq!(engine.logs.len(), 0);

        for i in 1..=150 {
            engine.log("INFO", "TEST", &format!("Log message {}", i));
        }

        assert_eq!(engine.logs.len(), 100);
        assert_eq!(engine.logs[0].message, "Log message 51");
        assert_eq!(engine.logs[99].message, "Log message 150");
    }

    #[test]
    fn test_matches_process() {
        assert!(DaemonEngine::matches_process("notepad.exe", "notepad.exe", ProcessMatchMode::Exact));
        assert!(DaemonEngine::matches_process("NOTEPAD.EXE", "notepad.exe", ProcessMatchMode::Exact));
        assert!(!DaemonEngine::matches_process("notepad.exe", "calc.exe", ProcessMatchMode::Exact));

        assert!(DaemonEngine::matches_process("*game*", "my_game_client.exe", ProcessMatchMode::Wildcard));
        assert!(DaemonEngine::matches_process("*.exe", "anything.exe", ProcessMatchMode::Wildcard));
        assert!(DaemonEngine::matches_process("steam*", "steamwebhelper.exe", ProcessMatchMode::Wildcard));
        assert!(!DaemonEngine::matches_process("*.exe", "anything.dll", ProcessMatchMode::Wildcard));

        assert!(DaemonEngine::matches_process("chrome", "google-chrome-stable", ProcessMatchMode::Regex));
        assert!(!DaemonEngine::matches_process("firefox", "google-chrome-stable", ProcessMatchMode::Regex));
    }

    #[test]
    fn test_rule_with_companion_mode_and_exit_actions_serde() {
        let json = r#"{
            "id": "rule_1",
            "name": "Game with Companion",
            "is_enabled": true,
            "trigger": {
                "$trigger_type": "process",
                "process_name": "game.exe",
                "is_started": true,
                "match_mode": "exact"
            },
            "actions": [
                {
                    "$action_type": "start_process",
                    "file_path": "companion.exe",
                    "close_on_exit": true,
                    "run_hidden": true
                }
            ],
            "on_exit_actions": [
                {
                    "$action_type": "notification",
                    "title": "Game Over",
                    "message": "Game exited"
                }
            ]
        }"#;

        let rule: Rule = serde_json::from_str(json).expect("deserialize rule");
        assert_eq!(rule.name, "Game with Companion");
        assert_eq!(rule.actions.len(), 1);
        if let Action::StartProcess { close_on_exit, run_hidden, .. } = &rule.actions[0] {
            assert!(*close_on_exit);
            assert!(*run_hidden);
        } else {
            panic!("Expected StartProcess");
        }

        assert_eq!(rule.on_exit_actions.len(), 1);
        if let Action::Notification { title, .. } = &rule.on_exit_actions[0] {
            assert_eq!(title, "Game Over");
        } else {
            panic!("Expected Notification");
        }
    }

    #[tokio::test]
    async fn test_schedule_trigger_interval() {
        let mut config = AppConfig::default();
        config.rules.push(Rule {
            id: "sched_rule".into(),
            name: "Interval Rule".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: Some(Trigger::Schedule {
                schedule_type: ScheduleType::Interval,
                interval_sec: 2,
                daily_time: None,
                days_of_week: Vec::new(),
                delay_sec: 0,
                target_datetime: None,
            }),
            actions: vec![Action::Notification {
                title: "Timer".into(),
                message: "Tick".into(),
            }],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);
        // First tick sets initial timestamp
        engine.tick_schedule().await;
        assert_eq!(engine.logs.len(), 0);

        // Artificially advance timer past the 2 second interval
        let past = Instant::now() - std::time::Duration::from_secs(3);
        engine.rule_last_triggered.insert("sched_rule".into(), past);

        // Next tick triggers rule execution
        engine.tick_schedule().await;
        assert!(engine.logs.iter().any(|l| l.category == "RULE" && l.message.contains("triggered")));
    }

    #[tokio::test]
    async fn test_schedule_trigger_daily_with_seconds() {
        let mut config = AppConfig::default();
        let now = chrono::Local::now();
        let time_str = format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second());

        config.rules.push(Rule {
            id: "daily_rule".into(),
            name: "Daily Rule".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: Some(Trigger::Schedule {
                schedule_type: ScheduleType::Daily,
                interval_sec: 0,
                daily_time: Some(time_str),
                days_of_week: Vec::new(),
                delay_sec: 0,
                target_datetime: None,
            }),
            actions: vec![Action::Notification {
                title: "Daily".into(),
                message: "Alarm".into(),
            }],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);
        engine.tick_schedule().await;
        assert!(engine.logs.iter().any(|l| l.category == "RULE" && l.message.contains("Daily Rule")));
    }

    #[tokio::test]
    async fn test_schedule_trigger_oneshot_with_seconds() {
        let mut config = AppConfig::default();
        let past = chrono::Local::now() - chrono::Duration::seconds(5);
        let dt_str = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            past.year(), past.month(), past.day(), past.hour(), past.minute(), past.second()
        );

        config.rules.push(Rule {
            id: "oneshot_rule".into(),
            name: "OneShot Rule".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: Some(Trigger::Schedule {
                schedule_type: ScheduleType::OneShot,
                interval_sec: 0,
                daily_time: None,
                days_of_week: Vec::new(),
                delay_sec: 0,
                target_datetime: Some(dt_str),
            }),
            actions: vec![Action::Notification {
                title: "OneShot".into(),
                message: "Fired".into(),
            }],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);
        engine.tick_schedule().await;
        assert!(engine.logs.iter().any(|l| l.category == "RULE" && l.message.contains("OneShot Rule")));
        assert!(!engine.config.rules[0].is_enabled); // OneShot automatically deactivates after execution
    }

    #[tokio::test]
    async fn test_schedule_trigger_interval_with_start_datetime() {
        let mut config = AppConfig::default();
        // Set future start datetime (1 hour in future)
        let future = chrono::Local::now() + chrono::Duration::hours(1);
        let future_str = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            future.year(), future.month(), future.day(), future.hour(), future.minute(), future.second()
        );

        config.rules.push(Rule {
            id: "future_interval_rule".into(),
            name: "Future Interval Rule".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: Some(Trigger::Schedule {
                schedule_type: ScheduleType::Interval,
                interval_sec: 10,
                daily_time: None,
                days_of_week: Vec::new(),
                delay_sec: 0,
                target_datetime: Some(future_str),
            }),
            actions: vec![Action::Notification {
                title: "Timer".into(),
                message: "Tick".into(),
            }],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);
        engine.tick_schedule().await;
        // Should not have run because future time has not arrived
        assert!(!engine.logs.iter().any(|l| l.message.contains("Future Interval Rule")));

        // Now set past start datetime (5 seconds ago)
        let past = chrono::Local::now() - chrono::Duration::seconds(5);
        let past_str = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            past.year(), past.month(), past.day(), past.hour(), past.minute(), past.second()
        );
        if let Some(Trigger::Schedule { target_datetime, .. }) = &mut engine.config.rules[0].trigger {
            *target_datetime = Some(past_str);
        }

        engine.tick_schedule().await;
        // Should have run immediately on first execution after start time
        assert!(engine.logs.iter().any(|l| l.message.contains("Future Interval Rule")));
    }

    #[tokio::test]
    async fn test_execute_shutdown_rules() {
        let mut config = AppConfig::default();
        config.rules.push(Rule {
            id: "shutdown_rule".into(),
            name: "Shutdown Cleanup".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: Some(Trigger::Schedule {
                schedule_type: ScheduleType::AppShutdown,
                interval_sec: 0,
                daily_time: None,
                days_of_week: Vec::new(),
                delay_sec: 0,
                target_datetime: None,
            }),
            actions: vec![Action::Notification {
                title: "Bye".into(),
                message: "Closing".into(),
            }],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);
        engine.execute_shutdown_rules().await;
        assert!(engine.logs.iter().any(|l| l.category == "RULE" && l.message.contains("Shutdown Cleanup")));
    }

    #[tokio::test]
    async fn test_single_instance_process_lifecycle() {
        let mut config = AppConfig::default();
        config.rules.push(Rule {
            id: "si_rule".into(),
            name: "Single Instance App".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: Some(Trigger::Process {
                process_name: "testapp.exe".into(),
                match_mode: ProcessMatchMode::Exact,
                single_instance: true,
            }),
            actions: vec![Action::Notification {
                title: "Started".into(),
                message: "App started".into(),
            }],
            on_exit_actions: vec![Action::Notification {
                title: "Exited".into(),
                message: "All instances exited".into(),
            }],
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);

        // 1. First process starts (PID 1000) -> triggers rule
        engine.handle_process_event(ProcessEvent {
            pid: 1000,
            process_name: "testapp.exe".into(),
            is_started: true,
        }).await;

        let start_logs: Vec<_> = engine.logs.iter()
            .filter(|l| l.category == "RULE" && l.message.contains("triggered by testapp.exe") && !l.message.contains("exit actions"))
            .collect();
        assert_eq!(start_logs.len(), 1, "First instance must trigger the rule");

        // 2. Second process of same exe starts (PID 1001) -> must be SKIPPED
        engine.handle_process_event(ProcessEvent {
            pid: 1001,
            process_name: "testapp.exe".into(),
            is_started: true,
        }).await;

        let start_logs_after_second: Vec<_> = engine.logs.iter()
            .filter(|l| l.category == "RULE" && l.message.contains("triggered by testapp.exe") && !l.message.contains("exit actions"))
            .collect();
        assert_eq!(start_logs_after_second.len(), 1, "Second instance must NOT trigger the rule");
        assert!(engine.logs.iter().any(|l| l.message.contains("skipped (Single Instance)")));

        // 3. First process exits (PID 1000), but second process (PID 1001) is still active
        // -> on_exit_actions must NOT run yet
        engine.handle_process_event(ProcessEvent {
            pid: 1000,
            process_name: "testapp.exe".into(),
            is_started: false,
        }).await;

        let exit_logs: Vec<_> = engine.logs.iter()
            .filter(|l| l.category == "RULE" && l.message.contains("exit actions triggered"))
            .collect();
        assert_eq!(exit_logs.len(), 0, "Exit actions must NOT trigger while another instance is running");

        // 4. Second process exits (PID 1001) -> all instances closed -> on_exit_actions MUST trigger!
        engine.handle_process_event(ProcessEvent {
            pid: 1001,
            process_name: "testapp.exe".into(),
            is_started: false,
        }).await;

        let exit_logs_after_all: Vec<_> = engine.logs.iter()
            .filter(|l| l.category == "RULE" && l.message.contains("exit actions triggered"))
            .collect();
        assert_eq!(exit_logs_after_all.len(), 1, "Exit actions MUST trigger when last instance closes");

        // 5. Subsequent new launch (PID 2000) -> should trigger again as new first instance!
        engine.handle_process_event(ProcessEvent {
            pid: 2000,
            process_name: "testapp.exe".into(),
            is_started: true,
        }).await;

        let final_start_logs: Vec<_> = engine.logs.iter()
            .filter(|l| l.category == "RULE" && l.message.contains("triggered by testapp.exe") && !l.message.contains("exit actions"))
            .collect();
        assert_eq!(final_start_logs.len(), 2, "Subsequent launch after all instances closed must trigger again");
    }

    #[tokio::test]
    async fn test_process_termination_only_rule() {
        let mut config = AppConfig::default();
        config.rules.push(Rule {
            id: "exit_only_rule".into(),
            name: "Exit Only".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: Some(Trigger::Process {
                process_name: "exitapp.exe".into(),
                match_mode: ProcessMatchMode::Exact,
                single_instance: true,
            }),
            actions: Vec::new(),
            on_exit_actions: vec![Action::Notification {
                title: "Exited".into(),
                message: "Closed".into(),
            }],
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);

        // Process starts: actions is empty, so no rule trigger log
        engine.handle_process_event(ProcessEvent {
            pid: 500,
            process_name: "exitapp.exe".into(),
            is_started: true,
        }).await;

        assert!(!engine.logs.iter().any(|l| l.category == "RULE" && l.message.contains("triggered by exitapp.exe")));

        // Process terminates: exit actions trigger!
        engine.handle_process_event(ProcessEvent {
            pid: 500,
            process_name: "exitapp.exe".into(),
            is_started: false,
        }).await;

        assert!(engine.logs.iter().any(|l| l.category == "RULE" && l.message.contains("exit actions triggered by exitapp.exe")));
    }

    #[tokio::test]
    async fn test_rule_delay_does_not_block_engine() {
        let mut config = AppConfig::default();
        config.rules.push(Rule {
            id: "delayed_rule".into(),
            name: "Delayed Automation".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: None,
            actions: vec![
                Action::Delay { seconds: 0.5 },
                Action::Notification {
                    title: "Test".into(),
                    message: "Completed".into(),
                },
            ],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        });

        let mut engine = DaemonEngine::new(config);
        let start = Instant::now();

        // 1. try_execute_rule MUST return immediately without blocking on the 0.5s delay
        let handle = engine.try_execute_rule(&engine.config.rules[0].clone(), "MANUAL_TEST", 0).await;
        let elapsed_on_return = start.elapsed();
        assert!(
            elapsed_on_return < std::time::Duration::from_millis(150),
            "try_execute_rule took too long to return: {:?}",
            elapsed_on_return
        );

        // 2. The trigger log was generated immediately
        assert!(engine.logs.iter().any(|l| l.category == "RULE" && l.message.contains("Delayed Automation")));

        // 3. Engine is free to perform other operations immediately (e.g. handle requests, state query)
        let resp = engine.handle_request(IpcRequest::Ping).await;
        assert!(matches!(resp, IpcResponse::Pong));

        // 4. Awaiting the task handle waits for the actual background delay to complete
        assert!(handle.is_some());
        let _ = handle.unwrap().await;
        assert!(start.elapsed() >= std::time::Duration::from_millis(450));
    }

    #[tokio::test]
    async fn test_concurrent_delays_do_not_block_each_other() {
        let config = AppConfig::default();
        let rule_long = Rule {
            id: "long_rule".into(),
            name: "Long Rule".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: None,
            actions: vec![Action::Delay { seconds: 0.4 }],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        };
        let rule_short = Rule {
            id: "short_rule".into(),
            name: "Short Rule".into(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: None,
            actions: vec![Action::Delay { seconds: 0.1 }],
            on_exit_actions: Vec::new(),
            execution_count: 0,
        };

        let mut engine = DaemonEngine::new(config);
        let start = Instant::now();

        // Launch long rule first, then short rule immediately after
        let handle_long = engine.try_execute_rule(&rule_long, "Test Concurrent", 0).await.unwrap();
        let handle_short = engine.try_execute_rule(&rule_short, "Test Concurrent", 0).await.unwrap();

        // The short rule must complete before the long rule, without being blocked by it
        handle_short.await.unwrap();
        let short_duration = start.elapsed();
        assert!(short_duration < std::time::Duration::from_millis(300), "Short rule was blocked: {:?}", short_duration);

        handle_long.await.unwrap();
        let total_duration = start.elapsed();
        assert!(total_duration >= std::time::Duration::from_millis(380), "Long rule finished too early: {:?}", total_duration);
    }
}

