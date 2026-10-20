use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tokio::sync::mpsc;
use crate::config::AppConfig;
use crate::ipc::{DaemonState, IpcRequest, IpcResponse};
use crate::models::{
    Action, AnalyticsSummary, LogItem, ProcessMatchMode,
    Rule, ScheduleType, SystemEventType, Trigger,
};
use crate::daemon::actions::ActionRunner;
use crate::daemon::watcher::{ProcessEvent, ProcessWatcher};

#[derive(Debug, Clone)]
pub struct BoundProcess {
    pub rule_id: String,
    pub primary_process_name: String,
    pub primary_pid: u32,
    pub target_executable: String,
}

#[derive(Debug, Clone)]
pub struct ActiveExecutionItem {
    pub rule_id: String,
    pub step_index: usize,
    pub total_steps: usize,
    pub is_exit: bool,
    pub started_at: Instant,
}

#[derive(Debug)]
pub enum ActionCompletionEvent {
    Log {
        level: String,
        category: String,
        message: String,
    },
    AddBoundProcess(BoundProcess),
    RuleExecutionStarted {
        execution_id: u64,
        rule_id: String,
        step_index: usize,
        total_steps: usize,
        is_exit: bool,
    },
    RuleExecutionStep {
        execution_id: u64,
        _rule_id: String,
        step_index: usize,
        total_steps: usize,
        is_exit: bool,
    },
    RuleExecutionFinished {
        execution_id: u64,
        _rule_id: String,
    },
    SetRuleCustomIcon {
        rule_id: String,
        icon_path: String,
    },
}

pub struct DaemonEngine {
    pub config: AppConfig,
    pub start_time: Instant,
    pub logs: Vec<LogItem>,
    pub watcher: ProcessWatcher,
    pub active_bound_processes: Vec<BoundProcess>,
    pub rule_last_triggered: HashMap<String, Instant>,
    pub active_rule_pids: HashMap<String, HashSet<u32>>,
    pub active_executions: HashMap<u64, ActiveExecutionItem>,
    pub analytics: AnalyticsSummary,
    pub last_foreground_pid: Option<u32>,
    pub hotkey_handle: Option<crate::daemon::hotkeys::HotkeyHandle>,
    pub resource_threshold_tracker: HashMap<(String, u32), Instant>,
    pub execution_abort_handles: HashMap<u64, (String, tokio::task::AbortHandle)>,
    action_tx: mpsc::UnboundedSender<ActionCompletionEvent>,
    action_rx: mpsc::UnboundedReceiver<ActionCompletionEvent>,
}

impl DaemonEngine {
    pub const MAX_LOGS: usize = 500;

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
            active_executions: HashMap::new(),
            analytics: AnalyticsSummary::default(),
            last_foreground_pid: None,
            hotkey_handle: None,
            resource_threshold_tracker: HashMap::new(),
            execution_abort_handles: HashMap::new(),
            action_tx,
            action_rx,
        }
    }

    pub fn is_rule_executing(&self, rule_id: &str) -> bool {
        self.active_executions.values().any(|e| e.rule_id == rule_id)
            || self.execution_abort_handles.values().any(|(r_id, _)| r_id == rule_id)
    }

    pub fn stop_rule_execution(&mut self, rule_id: &str) {
        let to_abort: Vec<u64> = self.execution_abort_handles
            .iter()
            .filter_map(|(&id, (r_id, _))| if r_id == rule_id { Some(id) } else { None })
            .collect();

        for id in to_abort {
            if let Some((_, abort_handle)) = self.execution_abort_handles.remove(&id) {
                abort_handle.abort();
                self.active_executions.remove(&id);
                self.log("INFO", "RULE", &format!("Stopped execution of rule: {}", rule_id));
            }
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
                ActionCompletionEvent::RuleExecutionStarted { execution_id, rule_id, step_index, total_steps, is_exit } => {
                    self.active_executions.insert(execution_id, ActiveExecutionItem {
                        rule_id,
                        step_index,
                        total_steps,
                        is_exit,
                        started_at: Instant::now(),
                    });
                }
                ActionCompletionEvent::RuleExecutionStep { execution_id, step_index, total_steps, is_exit, .. } => {
                    if let Some(item) = self.active_executions.get_mut(&execution_id) {
                        item.step_index = step_index;
                        item.total_steps = total_steps;
                        item.is_exit = is_exit;
                    }
                }
                ActionCompletionEvent::RuleExecutionFinished { execution_id, .. } => {
                    self.active_executions.remove(&execution_id);
                    self.execution_abort_handles.remove(&execution_id);
                }
                ActionCompletionEvent::SetRuleCustomIcon { rule_id, icon_path } => {
                    let mut updated_name = None;
                    if let Some(r) = self.config.rules.iter_mut().find(|r| r.id == rule_id) {
                        if r.custom_icon_path.is_none() {
                            r.custom_icon_path = Some(icon_path.clone());
                            updated_name = Some(r.name.clone());
                        }
                    }
                    if let Some(name) = updated_name {
                        let _ = self.config.save_to_file(AppConfig::config_path());
                        self.log("INFO", "RULE", &format!("Set custom icon for rule '{}' from process: {}", name, icon_path));
                    }
                }
            }
        }
    }

    pub fn log(&mut self, level: &str, category: &str, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        while self.logs.len() >= Self::MAX_LOGS {
            // Prioritize preserving RULE and ERROR logs: evict oldest entry that is neither RULE nor ERROR
            if let Some(pos) = self.logs.iter().position(|l| !l.category.eq_ignore_ascii_case("RULE") && !l.level.eq_ignore_ascii_case("ERROR")) {
                self.logs.remove(pos);
            } else {
                self.logs.remove(0);
            }
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

        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
            let hwnd = unsafe { GetForegroundWindow() };
            if hwnd.0 != std::ptr::null_mut() {
                let mut fg_pid = 0u32;
                unsafe { GetWindowThreadProcessId(hwnd, Some(&mut fg_pid)) };
                if fg_pid != 0 && self.last_foreground_pid != Some(fg_pid) {
                    let old_pid = self.last_foreground_pid;
                    self.last_foreground_pid = Some(fg_pid);

                    let snap = ProcessWatcher::snapshot_process_map();
                    if let Some(fg_name) = snap.get(&fg_pid).cloned() {
                        for rule in &rules {
                            if !rule.is_enabled {
                                continue;
                            }
                            if let Some(Trigger::WindowFocus {
                                process_name,
                                on_focus_gained,
                                ..
                            }) = &rule.trigger
                            {
                                if *on_focus_gained
                                    && Self::matches_process(
                                        process_name,
                                        &fg_name,
                                        ProcessMatchMode::Exact,
                                    )
                                {
                                    let _ = self
                                        .try_execute_rule(
                                            rule,
                                            &format!("WindowFocus Gained {}", fg_name),
                                            fg_pid,
                                        )
                                        .await;
                                }
                            }
                        }
                    }

                    if let Some(old_id) = old_pid {
                        if let Some(old_name) = snap.get(&old_id).cloned() {
                            for rule in &rules {
                                if !rule.is_enabled {
                                    continue;
                                }
                                if let Some(Trigger::WindowFocus {
                                    process_name,
                                    on_focus_lost,
                                    ..
                                }) = &rule.trigger
                                {
                                    if *on_focus_lost
                                        && Self::matches_process(
                                            process_name,
                                            &old_name,
                                            ProcessMatchMode::Exact,
                                        )
                                    {
                                        let _ = self
                                            .try_execute_rule(
                                                rule,
                                                &format!("WindowFocus Lost {}", old_name),
                                                old_id,
                                            )
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Evaluate ResourceThreshold rules
        let has_resource_rules = rules.iter().any(|r| r.is_enabled && matches!(r.trigger, Some(Trigger::ResourceThreshold { .. })));
        if has_resource_rules {
            let snap = ProcessWatcher::get_detailed_snapshot();
            for rule in &rules {
                if !rule.is_enabled {
                    continue;
                }
                if let Some(Trigger::ResourceThreshold { process_name, ram_mb, cpu_percent: _, duration_sec }) = &rule.trigger {
                    for proc in &snap {
                        if Self::matches_process(process_name, &proc.name, ProcessMatchMode::Exact) {
                            let mem_mb = proc.memory_bytes / (1024 * 1024);
                            let over_mem = ram_mb.map_or(false, |limit| mem_mb >= limit);
                            let key = (rule.id.clone(), proc.pid);

                            if over_mem {
                                let start = self.resource_threshold_tracker.entry(key).or_insert_with(Instant::now);
                                if start.elapsed().as_secs() >= *duration_sec {
                                    let _ = self.try_execute_rule(
                                        rule,
                                        &format!("Resource Threshold: {} ({} MB)", proc.name, mem_mb),
                                        proc.pid,
                                    ).await;
                                }
                            } else {
                                self.resource_threshold_tracker.remove(&key);
                            }
                        }
                    }
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
        self.try_execute_rule_with_context(rule, context_name, pid, HashMap::new()).await
    }

    pub async fn try_execute_rule_with_context(
        &mut self,
        rule: &Rule,
        context_name: &str,
        pid: u32,
        extra_ctx: HashMap<String, String>,
    ) -> Option<tokio::task::JoinHandle<()>> {
        self.drain_action_events();

        if let Some(limit) = rule.max_executions {
            if rule.execution_count >= limit {
                self.log(
                    "WARN",
                    "RULE",
                    &format!("Rule '{}' reached execution limit ({}/{}). Pausing.", rule.name, rule.execution_count, limit),
                );
                if let Some(r) = self.config.rules.iter_mut().find(|r| r.id == rule.id) {
                    r.is_enabled = false;
                    let _ = self.config.save_to_file(AppConfig::config_path());
                }
                return None;
            }
        }

        if let Some(last) = self.rule_last_triggered.get(&rule.id) {
            if last.elapsed().as_secs_f64() < rule.cooldown_seconds && context_name != "MANUAL_TEST" {
                return None;
            }
        }

        self.rule_last_triggered.insert(rule.id.clone(), Instant::now());
        self.log("TRIGGER", "RULE", &format!("Rule '{}' triggered by {}", rule.name, context_name));

        if rule.auto_process_icon && rule.custom_icon_path.is_none() {
            let exe_path = if pid > 0 {
                ProcessWatcher::get_process_image_path(pid)
            } else {
                ProcessWatcher::find_exe_path_by_name(context_name)
            };
            if let Some(path) = exe_path {
                let _ = self.action_tx.send(ActionCompletionEvent::SetRuleCustomIcon {
                    rule_id: rule.id.clone(),
                    icon_path: path,
                });
            }
        }

        if rule.actions.is_empty() {
            return None;
        }

        let mut ctx = HashMap::new();
        ctx.insert("ProcessName".into(), context_name.into());
        ctx.insert("ProcessId".into(), pid.to_string());
        ctx.insert("Timestamp".into(), chrono::Local::now().to_rfc3339());

        if pid > 0 {
            if let Some(path) = ProcessWatcher::get_process_image_path(pid) {
                ctx.insert("ProcessPath".into(), path.clone());
                if let Some(dir) = std::path::Path::new(&path).parent().and_then(|p| p.to_str()) {
                    ctx.insert("ProcessDir".into(), dir.to_string());
                }
            }
        }

        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
            let hwnd = unsafe { GetForegroundWindow() };
            if !hwnd.0.is_null() {
                let mut buf = [0u16; 512];
                let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
                if len > 0 {
                    let title = String::from_utf16_lossy(&buf[..len as usize]);
                    ctx.insert("WindowTitle".into(), title);
                }
            }
        }

        ctx.extend(extra_ctx);

        let actions = rule.actions.clone();
        let rule_name = rule.name.clone();
        let rule_id = rule.id.clone();
        let stop_on_error = rule.stop_on_error;
        let context_name = context_name.to_string();
        let tx = self.action_tx.clone();

        static EXECUTION_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let execution_id = EXECUTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let total_steps = actions.len();

        let mut quota_msg = None;
        if let Some(r) = self.config.rules.iter_mut().find(|r| r.id == rule.id) {
            r.execution_count += 1;
            if let Some(limit) = r.max_executions {
                if r.execution_count >= limit {
                    r.is_enabled = false;
                    quota_msg = Some(format!("Rule '{}' completed quota ({}/{}) and has been auto-paused.", r.name, r.execution_count, limit));
                }
            }
            let _ = self.config.save_to_file(AppConfig::config_path());
        }
        if let Some(msg) = quota_msg {
            self.log("INFO", "RULE", &msg);
        }

        self.analytics.total_executions += 1;

        let _ = tx.send(ActionCompletionEvent::RuleExecutionStarted {
            execution_id,
            rule_id: rule_id.clone(),
            step_index: 0,
            total_steps,
            is_exit: false,
        });

        let execution_rule_id = rule_id.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let spawn_rule_id = rule_id.clone();
            let join_handle = handle.spawn(async move {
                struct ExecGuard {
                    id: u64,
                    r_id: String,
                    tx: mpsc::UnboundedSender<ActionCompletionEvent>,
                }
                impl Drop for ExecGuard {
                    fn drop(&mut self) {
                        let _ = self.tx.send(ActionCompletionEvent::RuleExecutionFinished {
                            execution_id: self.id,
                            _rule_id: self.r_id.clone(),
                        });
                    }
                }
                let _guard = ExecGuard { id: execution_id, r_id: spawn_rule_id.clone(), tx: tx.clone() };
                let mut ctx = ctx;

                for (step_idx, action) in actions.iter().enumerate() {
                    let _ = tx.send(ActionCompletionEvent::RuleExecutionStep {
                        execution_id,
                        _rule_id: spawn_rule_id.clone(),
                        step_index: step_idx,
                        total_steps,
                        is_exit: false,
                    });

                    match ActionRunner::execute_with_output(action, &ctx).await {
                        Ok(out_opt) => {
                            if let Some(out) = out_opt {
                                ctx.insert("PrevOutput".to_string(), out.clone());
                                ctx.insert(format!("Step{}.Output", step_idx + 1), out);
                            }
                            if let Action::StartProcess { file_path, close_on_exit: true, .. } = action {
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
                            if stop_on_error {
                                let _ = tx.send(ActionCompletionEvent::Log {
                                    level: "WARN".to_string(),
                                    category: "RULE".to_string(),
                                    message: format!("Pipeline stopped for rule '{}' at step {} due to error", rule_name, step_idx + 1),
                                });
                                break;
                            }
                        }
                    }
                }
            });
            self.execution_abort_handles.insert(execution_id, (execution_rule_id, join_handle.abort_handle()));
            Some(join_handle)
        } else {
            let _ = tx.send(ActionCompletionEvent::RuleExecutionFinished {
                execution_id,
                _rule_id: rule_id,
            });
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

        if pid > 0 {
            if let Some(path) = ProcessWatcher::get_process_image_path(pid) {
                ctx.insert("ProcessPath".into(), path.clone());
                if let Some(dir) = std::path::Path::new(&path).parent().and_then(|p| p.to_str()) {
                    ctx.insert("ProcessDir".into(), dir.to_string());
                }
            }
        }

        let actions = rule.on_exit_actions.clone();
        let rule_name = rule.name.clone();
        let rule_id = rule.id.clone();
        let stop_on_error = rule.stop_on_error;
        let tx = self.action_tx.clone();

        static EXIT_EXECUTION_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let execution_id = EXIT_EXECUTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let total_steps = actions.len();

        let _ = tx.send(ActionCompletionEvent::RuleExecutionStarted {
            execution_id,
            rule_id: rule_id.clone(),
            step_index: 0,
            total_steps,
            is_exit: true,
        });

        let execution_rule_id = rule_id.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let spawn_rule_id = rule_id.clone();
            let join_handle = handle.spawn(async move {
                struct ExecGuard {
                    id: u64,
                    r_id: String,
                    tx: mpsc::UnboundedSender<ActionCompletionEvent>,
                }
                impl Drop for ExecGuard {
                    fn drop(&mut self) {
                        let _ = self.tx.send(ActionCompletionEvent::RuleExecutionFinished {
                            execution_id: self.id,
                            _rule_id: self.r_id.clone(),
                        });
                    }
                }
                let _guard = ExecGuard { id: execution_id, r_id: spawn_rule_id.clone(), tx: tx.clone() };
                let mut ctx = ctx;

                for (step_idx, action) in actions.iter().enumerate() {
                    let _ = tx.send(ActionCompletionEvent::RuleExecutionStep {
                        execution_id,
                        _rule_id: spawn_rule_id.clone(),
                        step_index: step_idx,
                        total_steps,
                        is_exit: true,
                    });

                    match ActionRunner::execute_with_output(action, &ctx).await {
                        Ok(out_opt) => {
                            if let Some(out) = out_opt {
                                ctx.insert("PrevOutput".to_string(), out.clone());
                                ctx.insert(format!("Step{}.Output", step_idx + 1), out);
                            }
                        }
                        Err(err) => {
                            let _ = tx.send(ActionCompletionEvent::Log {
                                level: "ERROR".to_string(),
                                category: "ACTION".to_string(),
                                message: format!("Exit action failed in '{}': {}", rule_name, err),
                            });
                            if stop_on_error {
                                break;
                            }
                        }
                    }
                }
            });
            self.execution_abort_handles.insert(execution_id, (execution_rule_id, join_handle.abort_handle()));
            Some(join_handle)
        } else {
            let _ = tx.send(ActionCompletionEvent::RuleExecutionFinished {
                execution_id,
                _rule_id: rule_id,
            });
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
                if let Some(h) = &self.hotkey_handle {
                    h.reload(&self.config.rules);
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
                if let Some(h) = &self.hotkey_handle {
                    h.reload(&self.config.rules);
                }
                IpcResponse::Ok
            }

            IpcRequest::DeleteRule(id) => {
                self.config.rules.retain(|r| r.id != id);
                let _ = self.config.save_to_file(AppConfig::config_path());
                if let Some(h) = &self.hotkey_handle {
                    h.reload(&self.config.rules);
                }
                IpcResponse::Ok
            }

            IpcRequest::ToggleRule { id, enabled } => {
                if let Some(rule) = self.config.rules.iter_mut().find(|r| r.id == id) {
                    rule.is_enabled = enabled;
                    let _ = self.config.save_to_file(AppConfig::config_path());
                }
                if let Some(h) = &self.hotkey_handle {
                    h.reload(&self.config.rules);
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

            IpcRequest::ImportRules(imported) => {
                let mut count = 0;
                for rule in imported {
                    if let Some(existing) = self.config.rules.iter_mut().find(|r| r.id == rule.id) {
                        *existing = rule;
                    } else {
                        self.config.rules.push(rule);
                    }
                    count += 1;
                }
                let _ = self.config.save_to_file(AppConfig::config_path());
                if let Some(h) = &self.hotkey_handle {
                    h.reload(&self.config.rules);
                }
                self.log("INFO", "CONFIG", &format!("Imported {count} automations"));
                IpcResponse::Ok
            }

            IpcRequest::TestStep { action, context } => {
                let res = ActionRunner::execute(&action, &context).await.map_err(|e| e.to_string());
                match &res {
                    Ok(()) => {
                        self.log("INFO", "TEST", "Step test executed successfully");
                    }
                    Err(err) => {
                        self.log("ERROR", "TEST", &format!("Step test failed: {err}"));
                    }
                }
                IpcResponse::TestStepResult(res)
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

        let mut active_rule_executions = HashMap::new();
        let mut grouped: HashMap<String, Vec<&ActiveExecutionItem>> = HashMap::new();
        for item in self.active_executions.values() {
            grouped.entry(item.rule_id.clone()).or_default().push(item);
        }
        for (r_id, items) in grouped {
            let active_count = items.len();
            if let Some(latest) = items.into_iter().max_by_key(|i| i.started_at) {
                active_rule_executions.insert(r_id, crate::ipc::ActiveRuleExecutionState {
                    active_count,
                    current_step: latest.step_index,
                    total_steps: latest.total_steps,
                    is_exit: latest.is_exit,
                });
            }
        }

        DaemonState {
            autorun_enabled,
            uptime_secs: self.start_time.elapsed().as_secs(),
            daemon_memory_bytes: mem,
            daemon_cpu_usage: 0.0,
            active_rules_count: active_rules,
            total_rules_count: total_rules,
            config: self.config.clone(),
            logs: self.logs.clone(),
            active_rule_executions,
            analytics: self.analytics.clone(),
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
    fn test_log_limit_500() {
        let mut engine = DaemonEngine::new(AppConfig::default());
        assert_eq!(engine.logs.len(), 0);

        for i in 1..=600 {
            engine.log("INFO", "TEST", &format!("Log message {}", i));
        }

        assert_eq!(engine.logs.len(), 500);
        assert_eq!(engine.logs[0].message, "Log message 101");
        assert_eq!(engine.logs[499].message, "Log message 600");
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

    #[test]
    fn test_process_audio_rule_serde() {
        use crate::models::ProcessAudioActionType;

        let json = r#"{
            "id": "audio_rule_1",
            "name": "Game Audio Intro Muter",
            "is_enabled": true,
            "actions": [
                {
                    "$action_type": "process_audio",
                    "process_name": "game.exe",
                    "action_type": "mute",
                    "volume_level": 100
                },
                {
                    "$action_type": "delay",
                    "seconds": 5.0
                },
                {
                    "$action_type": "process_audio",
                    "process_name": "{ProcessName}",
                    "action_type": "unmute",
                    "volume_level": 100
                }
            ]
        }"#;

        let rule: Rule = serde_json::from_str(json).expect("deserialize process audio rule");
        assert_eq!(rule.name, "Game Audio Intro Muter");
        assert_eq!(rule.actions.len(), 3);

        if let Action::ProcessAudio { process_name, action_type, .. } = &rule.actions[0] {
            assert_eq!(process_name, "game.exe");
            assert_eq!(*action_type, ProcessAudioActionType::Mute);
        } else {
            panic!("Expected ProcessAudio Mute");
        }

        if let Action::Delay { seconds } = &rule.actions[1] {
            assert_eq!(*seconds, 5.0);
        } else {
            panic!("Expected Delay");
        }

        if let Action::ProcessAudio { process_name, action_type, .. } = &rule.actions[2] {
            assert_eq!(process_name, "{ProcessName}");
            assert_eq!(*action_type, ProcessAudioActionType::Unmute);
        } else {
            panic!("Expected ProcessAudio Unmute");
        }

        let serialized = serde_json::to_string(&rule).expect("serialize rule");
        assert!(serialized.contains("process_audio"));
        assert!(serialized.contains("game.exe"));
    }

    #[test]
    fn test_microphone_rule_serde() {
        use crate::models::MicrophoneActionType;

        let json = r#"{
            "id": "mic_rule_1",
            "name": "Mic Muter",
            "is_enabled": true,
            "actions": [
                {
                    "$action_type": "microphone",
                    "action_type": "toggle_mute",
                    "volume_level": 80
                }
            ]
        }"#;

        let rule: Rule = serde_json::from_str(json).expect("deserialize mic rule");
        assert_eq!(rule.name, "Mic Muter");
        assert_eq!(rule.actions.len(), 1);

        if let Action::Microphone { action_type, volume_level } = &rule.actions[0] {
            assert_eq!(*action_type, MicrophoneActionType::ToggleMute);
            assert_eq!(*volume_level, 80);
        } else {
            panic!("Expected Action::Microphone");
        }

        let serialized = serde_json::to_string(&rule).expect("serialize mic rule");
        assert!(serialized.contains("microphone"));
        assert!(serialized.contains("toggle_mute"));
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
        });

        let mut engine = DaemonEngine::new(config);
        let tick_now = chrono::Local::now();
        let tick_time_str = format!("{:02}:{:02}:{:02}", tick_now.hour(), tick_now.minute(), tick_now.second());
        if let Some(r) = engine.config.rules.first_mut() {
            if let Some(Trigger::Schedule { daily_time, .. }) = &mut r.trigger {
                *daily_time = Some(tick_time_str);
            }
        }
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
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

    #[tokio::test]
    async fn test_rule_execution_tracking_concurrent_and_step() {
        let config = AppConfig::default();
        let rule = Rule {
            id: "tracked_rule".into(),
            name: "Tracked Multi-Step Rule".into(),
            description: "Testing execution tracking".into(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            trigger: None,
            actions: vec![
                Action::Delay { seconds: 0.2 },
                Action::Delay { seconds: 0.2 },
            ],
            on_exit_actions: Vec::new(),
            execution_count: 0,
            custom_icon_path: None,
            auto_process_icon: false,
            ..Default::default()
        };

        let mut engine = DaemonEngine::new(config);

        // Initially no active executions
        let state0 = engine.get_current_state();
        assert!(!state0.active_rule_executions.contains_key("tracked_rule"));

        // Launch instance 1
        let h1 = engine.try_execute_rule(&rule, "MANUAL_TEST", 0).await.unwrap();
        // Give tokio runtime a brief tick to process channel event
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let state1 = engine.get_current_state();
        assert!(state1.active_rule_executions.contains_key("tracked_rule"));
        let exec1 = &state1.active_rule_executions["tracked_rule"];
        assert_eq!(exec1.active_count, 1);
        assert_eq!(exec1.current_step, 0);
        assert_eq!(exec1.total_steps, 2);

        // Launch instance 2 concurrently
        let h2 = engine.try_execute_rule(&rule, "MANUAL_TEST", 0).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let state2 = engine.get_current_state();
        let exec2 = &state2.active_rule_executions["tracked_rule"];
        assert_eq!(exec2.active_count, 2, "Must report 2 concurrent active executions");

        // Wait for both to finish
        let _ = h1.await;
        let _ = h2.await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let state_done = engine.get_current_state();
        assert!(!state_done.active_rule_executions.contains_key("tracked_rule"));
    }
}

