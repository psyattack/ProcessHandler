use std::time::Instant;
use crate::ipc::{DaemonState, IpcClient, IpcRequest, IpcResponse};
use crate::models::{ProcessSnapshotItem, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Rules,
    Processes,
    LiveStream,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessSortColumn {
    #[default]
    Name,
    Pid,
    Memory,
    WindowTitle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessCategoryFilter {
    #[default]
    All,
    WindowedOnly,
    BackgroundOnly,
}

#[derive(Debug, Clone)]
pub struct ExpandedScriptEditorState {
    pub step_idx: usize,
    pub is_exit_pipeline: bool,
    pub language: crate::gui::components::script_highlighter::ScriptLanguage,
}

pub struct GuiState {
    pub active_view: ActiveView,
    pub search_query: String,
    pub daemon_state: Option<DaemonState>,
    pub cached_processes: Vec<ProcessSnapshotItem>,
    pub process_sort_column: ProcessSortColumn,
    pub process_sort_ascending: bool,
    pub process_category_filter: ProcessCategoryFilter,
    pub selected_process: Option<ProcessSnapshotItem>,
    pub selected_rule_id: Option<String>,
    pub is_creating_new_rule: bool,
    pub editing_rule: Option<Rule>,
    pub expanded_script_editor: Option<ExpandedScriptEditorState>,
    pub last_script_editor: Option<ExpandedScriptEditorState>,
    pub last_sync: Instant,
    pub toast: Option<(String, Instant)>,
    pub is_daemon_connected: bool,
    pub icon_cache: crate::gui::components::icons::ProcessIconCache,
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            active_view: ActiveView::Rules,
            search_query: String::new(),
            daemon_state: None,
            cached_processes: Vec::new(),
            process_sort_column: ProcessSortColumn::Name,
            process_sort_ascending: true,
            process_category_filter: ProcessCategoryFilter::All,
            selected_process: None,
            selected_rule_id: None,
            is_creating_new_rule: false,
            editing_rule: None,
            expanded_script_editor: None,
            last_script_editor: None,
            last_sync: Instant::now() - std::time::Duration::from_secs(10),
            toast: None,
            is_daemon_connected: false,
            icon_cache: crate::gui::components::icons::ProcessIconCache::new(),
        }
    }

    pub fn open_script_editor(
        &mut self,
        step_idx: usize,
        is_exit_pipeline: bool,
        language: crate::gui::components::script_highlighter::ScriptLanguage,
    ) {
        let editor = ExpandedScriptEditorState {
            step_idx,
            is_exit_pipeline,
            language,
        };
        self.expanded_script_editor = Some(editor.clone());
        self.last_script_editor = Some(editor);
    }

    pub fn close_script_editor(&mut self) {
        self.expanded_script_editor = None;
    }

    pub fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    pub fn toggle_autorun(&mut self, enable: bool) {
        match IpcClient::call(&IpcRequest::SetAutorun(enable)) {
            Ok(IpcResponse::Ok) => {
                self.set_toast(if enable { "Autorun enabled in Windows Registry" } else { "Autorun removed from Windows Registry" });
            }
            Ok(IpcResponse::Error(err)) => {
                self.set_toast(format!("Autorun error: {err}"));
            }
            _ => {}
        }
        self.sync_with_daemon();
    }

    pub fn sync_with_daemon(&mut self) {
        if self.last_sync.elapsed().as_millis() < 400 {
            return;
        }
        self.last_sync = Instant::now();

        match IpcClient::call(&IpcRequest::GetState) {
            Ok(IpcResponse::State(state)) => {
                self.daemon_state = Some(state);
                self.is_daemon_connected = true;
            }
            _ => {
                self.is_daemon_connected = false;
            }
        }

        if self.active_view == ActiveView::Processes || self.editing_rule.is_some() || !self.search_query.is_empty() {
            if let Ok(IpcResponse::Processes(procs)) = IpcClient::call(&IpcRequest::GetProcesses) {
                self.cached_processes = procs;
            }
        }
    }

    pub fn ensure_processes(&mut self) {
        if self.cached_processes.is_empty() {
            if let Ok(IpcResponse::Processes(procs)) = IpcClient::call(&IpcRequest::GetProcesses) {
                self.cached_processes = procs;
            }
        }
    }


    pub fn save_rule(&mut self, rule: Rule) {
        let _ = IpcClient::call(&IpcRequest::SaveRule(rule));
        self.editing_rule = None;
        self.is_creating_new_rule = false;
        self.set_toast("Rule saved successfully");
        self.sync_with_daemon();
    }

    pub fn delete_rule(&mut self, id: &str) {
        let _ = IpcClient::call(&IpcRequest::DeleteRule(id.to_string()));
        if self.selected_rule_id.as_deref() == Some(id) {
            self.selected_rule_id = None;
            self.editing_rule = None;
        }
        self.set_toast("Rule deleted");
        self.sync_with_daemon();
    }

    pub fn clone_rule(&mut self, rule: &Rule) {
        let cloned = rule.clone_with_new_id();
        let new_id = cloned.id.clone();
        let new_name = cloned.name.clone();

        let _ = IpcClient::call(&IpcRequest::SaveRule(cloned.clone()));
        self.selected_rule_id = Some(new_id);
        if self.editing_rule.is_some() {
            self.editing_rule = Some(cloned);
            self.is_creating_new_rule = false;
        }
        self.set_toast(format!("Cloned rule (paused): {}", new_name));
        self.sync_with_daemon();
    }

    pub fn toggle_rule(&mut self, id: &str, enabled: bool) {
        let _ = IpcClient::call(&IpcRequest::ToggleRule {
            id: id.to_string(),
            enabled,
        });
        self.sync_with_daemon();
    }

    pub fn test_run_rule(&mut self, id: &str) {
        let _ = IpcClient::call(&IpcRequest::TestRunRule(id.to_string()));
        self.set_toast("Test execution triggered");
        self.sync_with_daemon();
    }
}
