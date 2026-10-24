use std::time::Instant;
use crate::ipc::{DaemonState, IpcClient, IpcRequest, IpcResponse};
use crate::models::{ProcessSnapshotItem, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Rules,
    Analytics,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessSortColumn {
    #[default]
    Name,
    Pid,
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

#[derive(Debug, Clone)]
pub struct MacroEditorModalState {
    pub step_idx: usize,
    pub is_exit_pipeline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPickerTarget {
    Trigger,
    Action(usize),
    ExitAction(usize),
    Condition(usize),
    ActionCondition { action_idx: usize, cond_idx: usize, is_exit: bool },
    ActionElseIfCondition { action_idx: usize, branch_idx: usize, cond_idx: usize, is_exit: bool },
    IfElseThen { parent_idx: usize, sub_idx: usize, is_exit: bool },
    IfElseElseIf { parent_idx: usize, branch_idx: usize, sub_idx: usize, is_exit: bool },
    IfElseElse { parent_idx: usize, sub_idx: usize, is_exit: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfElseCatalogTarget {
    Then(usize),
    ElseIf { parent_idx: usize, branch_idx: usize },
    Else(usize),
}

#[derive(Debug, Clone)]
pub struct ActionCatalogModalState {
    pub is_exit_pipeline: bool,
    pub insert_idx: Option<usize>,
    pub selected_category: usize,
    pub search_query: String,
    pub ifelse_target: Option<IfElseCatalogTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Appearance,
    Engine,
    Updates,
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
    pub builder_draft: Option<Rule>,
    pub picker_target: Option<ProcessPickerTarget>,
    pub last_picker_target: Option<ProcessPickerTarget>,
    pub process_search_query: String,
    pub expanded_script_editor: Option<ExpandedScriptEditorState>,
    pub last_script_editor: Option<ExpandedScriptEditorState>,
    pub macro_editor: Option<MacroEditorModalState>,
    pub last_macro_editor: Option<MacroEditorModalState>,
    pub action_catalog: Option<ActionCatalogModalState>,
    pub last_action_catalog: Option<ActionCatalogModalState>,
    pub language: crate::i18n::Language,
    pub theme: crate::config::AppTheme,
    pub last_sync: Instant,
    pub toast: Option<(String, Instant)>,
    pub is_daemon_connected: bool,
    pub icon_cache: crate::gui::components::icons::ProcessIconCache,
    pub available_update: Option<crate::updater::ReleaseInfo>,
    pub is_checking_update: bool,
    pub is_downloading_update: bool,
    pub gui_closed_during_update: bool,
    pub update_download_progress: f32,
    pub show_changelog_modal: bool,
    pub show_guide_modal: bool,
    pub guide_selected_topic: usize,
    pub analytics_log_filter: String,
    pub group_full_presses: bool,
    pub undo_stack: Vec<Rule>,
    pub redo_stack: Vec<Rule>,
    pub settings_tab: SettingsTab,
    update_event_tx: std::sync::mpsc::Sender<crate::updater::UpdateEvent>,
    update_event_rx: std::sync::mpsc::Receiver<crate::updater::UpdateEvent>,
}

impl GuiState {
    pub fn new() -> Self {
        let config = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
        let (update_event_tx, update_event_rx) = std::sync::mpsc::channel();
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
            builder_draft: None,
            picker_target: None,
            last_picker_target: None,
            process_search_query: String::new(),
            expanded_script_editor: None,
            last_script_editor: None,
            macro_editor: None,
            last_macro_editor: None,
            action_catalog: None,
            last_action_catalog: None,
            language: config.language,
            theme: config.theme,
            last_sync: Instant::now() - std::time::Duration::from_secs(10),
            toast: None,
            is_daemon_connected: false,
            icon_cache: crate::gui::components::icons::ProcessIconCache::new(),
            available_update: None,
            is_checking_update: false,
            is_downloading_update: false,
            gui_closed_during_update: false,
            update_download_progress: 0.0,
            show_changelog_modal: false,
            show_guide_modal: false,
            guide_selected_topic: 0,
            analytics_log_filter: String::new(),
            group_full_presses: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            settings_tab: SettingsTab::General,
            update_event_tx,
            update_event_rx,
        }
    }

    pub fn loc(&self) -> &'static crate::i18n::Strings {
        crate::i18n::get(self.language)
    }

    pub fn set_language(&mut self, lang: crate::i18n::Language) {
        if self.language != lang {
            self.language = lang;
            let mut cfg = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
            cfg.language = lang;
            let _ = cfg.save_to_file(crate::config::AppConfig::config_path());
            self.set_toast(self.loc().toast_lang_changed);
        }
    }

    pub fn set_theme(&mut self, theme: crate::config::AppTheme) {
        if self.theme != theme {
            self.theme = theme;
            let mut cfg = crate::config::AppConfig::load_from_file(crate::config::AppConfig::config_path());
            cfg.theme = theme;
            let _ = cfg.save_to_file(crate::config::AppConfig::config_path());
            self.set_toast(self.loc().toast_theme_changed);
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

    pub fn open_macro_editor(&mut self, step_idx: usize, is_exit_pipeline: bool) {
        let editor = MacroEditorModalState {
            step_idx,
            is_exit_pipeline,
        };
        self.macro_editor = Some(editor.clone());
        self.last_macro_editor = Some(editor);
        crate::gui::macro_recorder::set_studio_active(true);
    }

    pub fn close_macro_editor(&mut self) {
        let rec = crate::gui::macro_recorder::MacroRecorder::global();
        if rec.is_active() {
            rec.stop_recording();
        }
        crate::gui::macro_recorder::stop_macro_test();
        self.macro_editor = None;
        crate::gui::macro_recorder::set_studio_active(false);
    }

    pub fn open_action_catalog(&mut self, is_exit_pipeline: bool, insert_idx: Option<usize>) {
        let catalog = ActionCatalogModalState {
            is_exit_pipeline,
            insert_idx,
            selected_category: 0,
            search_query: String::new(),
            ifelse_target: None,
        };
        self.action_catalog = Some(catalog.clone());
        self.last_action_catalog = Some(catalog);
    }

    pub fn open_action_catalog_nested(&mut self, is_exit_pipeline: bool, target: IfElseCatalogTarget) {
        let catalog = ActionCatalogModalState {
            is_exit_pipeline,
            insert_idx: None,
            selected_category: 0,
            search_query: String::new(),
            ifelse_target: Some(target),
        };
        self.action_catalog = Some(catalog.clone());
        self.last_action_catalog = Some(catalog);
    }

    pub fn close_action_catalog(&mut self) {
        self.action_catalog = None;
    }

    pub fn add_action_from_catalog(
        &mut self,
        action: crate::models::Action,
        is_exit_pipeline: bool,
        insert_idx: Option<usize>,
        ifelse_target: Option<IfElseCatalogTarget>,
    ) {
        self.push_undo_snapshot();
        if let Some(rule) = &mut self.editing_rule {
            if let Some(target) = ifelse_target {
                let list = if is_exit_pipeline {
                    &mut rule.on_exit_actions
                } else {
                    &mut rule.actions
                };
                match target {
                    IfElseCatalogTarget::Then(parent_idx) => {
                        if let Some(crate::models::Action::IfElse { then_actions, .. }) = list.get_mut(parent_idx) {
                            then_actions.push(action);
                        }
                    }
                    IfElseCatalogTarget::ElseIf { parent_idx, branch_idx } => {
                        if let Some(crate::models::Action::IfElse { else_if_branches, .. }) = list.get_mut(parent_idx) {
                            if let Some(branch) = else_if_branches.get_mut(branch_idx) {
                                branch.actions.push(action);
                            }
                        }
                    }
                    IfElseCatalogTarget::Else(parent_idx) => {
                        if let Some(crate::models::Action::IfElse { else_actions, .. }) = list.get_mut(parent_idx) {
                            else_actions.push(action);
                        }
                    }
                }
            } else {
                let list = if is_exit_pipeline {
                    &mut rule.on_exit_actions
                } else {
                    &mut rule.actions
                };
                if let Some(idx) = insert_idx {
                    let clamped = idx.min(list.len());
                    list.insert(clamped, action);
                } else {
                    list.push(action);
                }
            }
        }
        self.close_action_catalog();
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

    pub fn is_any_rule_executing(&self) -> bool {
        self.daemon_state
            .as_ref()
            .map(|s| !s.active_rule_executions.is_empty())
            .unwrap_or(false)
    }

    pub fn sync_with_daemon(&mut self) {
        let interval_ms = if self.is_any_rule_executing() { 80 } else { 400 };
        if self.last_sync.elapsed().as_millis() < interval_ms {
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

        if self.picker_target.is_some() || self.editing_rule.is_some() || !self.search_query.is_empty() {
            if let Ok(IpcResponse::Processes(procs)) = IpcClient::call(&IpcRequest::GetProcesses) {
                self.cached_processes = procs;
            }
        }
    }

    pub fn copy_single_rule_to_clipboard(&mut self, ctx: &egui::Context, rule: &Rule) {
        match crate::sharing::compress_and_encode(&crate::sharing::SharedAutomation::Single(Box::new(rule.clone()))) {
            Ok(encoded) => {
                crate::sharing::copy_text_to_clipboard(ctx, &encoded);
                self.set_toast(self.loc().toast_copy_single_success.replace("{}", &rule.name));
            }
            Err(e) => {
                self.set_toast(format!("Error: {e}"));
            }
        }
    }

    pub fn copy_all_rules_to_clipboard(&mut self, ctx: &egui::Context) {
        let rules = self.daemon_state.as_ref().map(|s| s.config.rules.clone()).unwrap_or_default();
        if rules.is_empty() {
            self.set_toast(self.loc().toast_copy_all_empty);
            return;
        }
        match crate::sharing::compress_and_encode(&crate::sharing::SharedAutomation::Batch(rules.clone())) {
            Ok(encoded) => {
                crate::sharing::copy_text_to_clipboard(ctx, &encoded);
                self.set_toast(self.loc().toast_copy_all_success.replace("{}", &rules.len().to_string()));
            }
            Err(e) => {
                self.set_toast(format!("Error: {e}"));
            }
        }
    }

    pub fn import_automations_from_clipboard(&mut self, _ctx: &egui::Context) {
        let text = match crate::sharing::read_text_from_clipboard() {
            Ok(t) => t,
            Err(e) => {
                self.set_toast(format!("{}: {}", self.loc().toast_import_clipboard_err, e));
                return;
            }
        };

        match crate::sharing::decode_and_decompress(&text) {
            Ok(rules) => {
                if rules.is_empty() {
                    self.set_toast(self.loc().toast_import_empty);
                    return;
                }

                let existing_ids: std::collections::HashSet<String> = self.daemon_state
                    .as_ref()
                    .map(|s| s.config.rules.iter().map(|r| r.id.clone()).collect())
                    .unwrap_or_default();

                let mut prepared_rules = Vec::new();
                for mut r in rules {
                    if existing_ids.contains(&r.id) {
                        static IMPORT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                        let count = IMPORT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        r.id = format!("rule_{:x}_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), count);
                        r.name = format!("{} (Imported)", r.name);
                    }
                    prepared_rules.push(r);
                }

                let count = prepared_rules.len();
                let _ = IpcClient::call(&IpcRequest::ImportRules(prepared_rules));
                self.set_toast(self.loc().toast_import_success.replace("{}", &count.to_string()));
                self.sync_with_daemon();
            }
            Err(err) => {
                self.set_toast(format!("{}: {}", self.loc().toast_import_invalid, err));
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


    pub fn initial_new_rule(&self) -> Rule {
        Rule {
            id: format!("rule_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
            name: String::new(),
            description: String::new(),
            is_enabled: true,
            cooldown_seconds: 0.0,
            max_executions: None,
            execution_count: 0,
            stop_on_error: true,
            trigger: Some(crate::models::Trigger::Process {
                process_name: String::new(),
                match_mode: crate::models::ProcessMatchMode::Exact,
                single_instance: false,
            }),
            actions: Vec::new(),
            on_exit_actions: Vec::new(),
            custom_icon_path: None,
            auto_process_icon: false,
            conditions: Vec::new(),
        }
    }

    pub fn push_undo_snapshot(&mut self) {
        if let Some(r) = &self.editing_rule {
            if self.undo_stack.last() != Some(r) {
                self.undo_stack.push(r.clone());
                if self.undo_stack.len() > 50 {
                    self.undo_stack.remove(0);
                }
                self.redo_stack.clear();
            }
        }
    }

    pub fn push_undo_snapshot_of(&mut self, r: &Rule) {
        if self.undo_stack.last() != Some(r) {
            self.undo_stack.push(r.clone());
            if self.undo_stack.len() > 50 {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            if let Some(curr) = self.editing_rule.take() {
                self.redo_stack.push(curr);
            }
            self.editing_rule = Some(prev);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            if let Some(curr) = self.editing_rule.take() {
                self.undo_stack.push(curr);
            }
            self.editing_rule = Some(next);
        }
    }

    pub fn start_new_rule(&mut self) {
        self.selected_rule_id = None;
        self.selected_process = None;
        self.is_creating_new_rule = true;
        self.undo_stack.clear();
        self.redo_stack.clear();
        let rule = self.builder_draft.take().unwrap_or_else(|| self.initial_new_rule());
        self.editing_rule = Some(rule);
    }

    pub fn reset_builder_state(&mut self) {
        self.builder_draft = None;
        self.close_action_catalog();
        if self.is_creating_new_rule {
            self.editing_rule = Some(self.initial_new_rule());
        }
    }

    pub fn close_editor(&mut self) {
        self.close_action_catalog();
        if self.is_creating_new_rule {
            self.builder_draft = self.editing_rule.take();
        } else {
            self.editing_rule = None;
        }
        self.is_creating_new_rule = false;
        self.selected_rule_id = None;
        self.picker_target = None;
    }

    pub fn open_process_picker(&mut self, target: ProcessPickerTarget) {
        self.ensure_processes();
        self.process_search_query.clear();
        self.picker_target = Some(target);
        self.last_picker_target = Some(target);
    }

    pub fn close_process_picker(&mut self) {
        self.picker_target = None;
    }

    /// Whether any modal popup (script editor, action catalog, process picker, or changelog) is currently open.
    pub fn is_modal_open(&self) -> bool {
        self.expanded_script_editor.is_some()
            || self.action_catalog.is_some()
            || self.picker_target.is_some()
            || (self.show_changelog_modal && self.available_update.is_some())
            || self.show_guide_modal
            || self.macro_editor.is_some()
    }

    pub fn apply_selected_process(&mut self, proc_name: &str) {
        let loc = self.loc();
        let target = self.picker_target.take().or(self.last_picker_target);
        self.picker_target = None;

        self.push_undo_snapshot();
        if let Some(rule) = &mut self.editing_rule {
            match target {
                Some(ProcessPickerTarget::Action(idx)) => {
                    if let Some(action) = rule.actions.get_mut(idx) {
                        apply_process_to_action(action, proc_name);
                    }
                }
                Some(ProcessPickerTarget::ExitAction(idx)) => {
                    if let Some(action) = rule.on_exit_actions.get_mut(idx) {
                        apply_process_to_action(action, proc_name);
                    }
                }
                Some(ProcessPickerTarget::Condition(idx)) => {
                    if let Some(item) = rule.conditions.get_mut(idx) {
                        apply_process_to_condition(&mut item.condition, proc_name);
                    }
                }
                Some(ProcessPickerTarget::ActionCondition { action_idx, cond_idx, is_exit }) => {
                    let list = if is_exit { &mut rule.on_exit_actions } else { &mut rule.actions };
                    if let Some(crate::models::Action::IfElse { conditions, .. }) = list.get_mut(action_idx) {
                        if let Some(item) = conditions.get_mut(cond_idx) {
                            apply_process_to_condition(&mut item.condition, proc_name);
                        }
                    }
                }
                Some(ProcessPickerTarget::ActionElseIfCondition { action_idx, branch_idx, cond_idx, is_exit }) => {
                    let list = if is_exit { &mut rule.on_exit_actions } else { &mut rule.actions };
                    if let Some(crate::models::Action::IfElse { else_if_branches, .. }) = list.get_mut(action_idx) {
                        if let Some(branch) = else_if_branches.get_mut(branch_idx) {
                            if let Some(item) = branch.conditions.get_mut(cond_idx) {
                                apply_process_to_condition(&mut item.condition, proc_name);
                            }
                        }
                    }
                }
                Some(ProcessPickerTarget::IfElseThen { parent_idx, sub_idx, is_exit }) => {
                    let list = if is_exit { &mut rule.on_exit_actions } else { &mut rule.actions };
                    if let Some(crate::models::Action::IfElse { then_actions, .. }) = list.get_mut(parent_idx) {
                        if let Some(sub_action) = then_actions.get_mut(sub_idx) {
                            apply_process_to_action(sub_action, proc_name);
                        }
                    }
                }
                Some(ProcessPickerTarget::IfElseElseIf { parent_idx, branch_idx, sub_idx, is_exit }) => {
                    let list = if is_exit { &mut rule.on_exit_actions } else { &mut rule.actions };
                    if let Some(crate::models::Action::IfElse { else_if_branches, .. }) = list.get_mut(parent_idx) {
                        if let Some(branch) = else_if_branches.get_mut(branch_idx) {
                            if let Some(sub_action) = branch.actions.get_mut(sub_idx) {
                                apply_process_to_action(sub_action, proc_name);
                            }
                        }
                    }
                }
                Some(ProcessPickerTarget::IfElseElse { parent_idx, sub_idx, is_exit }) => {
                    let list = if is_exit { &mut rule.on_exit_actions } else { &mut rule.actions };
                    if let Some(crate::models::Action::IfElse { else_actions, .. }) = list.get_mut(parent_idx) {
                        if let Some(sub_action) = else_actions.get_mut(sub_idx) {
                            apply_process_to_action(sub_action, proc_name);
                        }
                    }
                }
                Some(ProcessPickerTarget::Trigger) | None => {
                    match &mut rule.trigger {
                        Some(crate::models::Trigger::Process { process_name, .. }) => {
                            *process_name = proc_name.to_string();
                        }
                        Some(crate::models::Trigger::WindowFocus { process_name, .. }) => {
                            *process_name = proc_name.to_string();
                        }
                        Some(crate::models::Trigger::ResourceThreshold { process_name, .. }) => {
                            *process_name = proc_name.to_string();
                        }
                        _ => {
                            rule.trigger = Some(crate::models::Trigger::Process {
                                process_name: proc_name.to_string(),
                                match_mode: crate::models::ProcessMatchMode::Exact,
                                single_instance: false,
                            });
                        }
                    }

                    if rule.name.is_empty()
                        || rule.name == loc.rules_default_new_name
                        || rule.name.starts_with(loc.inspector_rule_track_prefix)
                    {
                        rule.name = format!("{} {}", loc.inspector_rule_track_prefix, proc_name);
                    }
                }
            }
            rule.auto_process_icon = true;
        }
    }

    pub fn apply_process_to_draft_or_new_rule(&mut self, proc_name: &str) {
        let loc = self.loc();
        let mut rule = self.initial_new_rule();
        rule.trigger = Some(crate::models::Trigger::Process {
            process_name: proc_name.to_string(),
            match_mode: crate::models::ProcessMatchMode::Exact,
            single_instance: false,
        });
        rule.name = format!("{} {}", loc.inspector_rule_track_prefix, proc_name);
        rule.auto_process_icon = true;
        self.active_view = ActiveView::Rules;
        self.is_creating_new_rule = true;
        self.selected_rule_id = None;
        self.editing_rule = Some(rule);
        self.selected_process = None;
    }

    pub fn edit_rule(&mut self, rule: Rule) {
        if self.is_creating_new_rule {
            self.builder_draft = self.editing_rule.take();
        }
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.selected_rule_id = Some(rule.id.clone());
        self.editing_rule = Some(rule);
        self.is_creating_new_rule = false;
        self.selected_process = None;
    }

    pub fn save_rule(&mut self, mut rule: Rule) {
        if rule.name.trim().is_empty() {
            rule.name = self.loc().rules_default_new_name.to_string();
        }
        let _ = IpcClient::call(&IpcRequest::SaveRule(rule));
        self.editing_rule = None;
        self.is_creating_new_rule = false;
        self.builder_draft = None;
        self.set_toast(self.loc().toast_rule_saved);
        self.sync_with_daemon();
    }

    pub fn delete_rule(&mut self, id: &str) {
        let _ = IpcClient::call(&IpcRequest::DeleteRule(id.to_string()));
        if self.selected_rule_id.as_deref() == Some(id) {
            self.selected_rule_id = None;
            self.editing_rule = None;
        }
        self.set_toast(self.loc().toast_rule_deleted);
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
        self.set_toast(format!("{}: {}", self.loc().toast_rule_cloned, new_name));
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
        self.set_toast(self.loc().toast_test_execution);
        self.sync_with_daemon();
    }

    pub fn check_for_updates(&mut self, is_manual: bool) {
        if self.is_checking_update || self.is_downloading_update {
            return;
        }
        self.is_checking_update = true;
        let tx = self.update_event_tx.clone();
        let current_ver = env!("CARGO_PKG_VERSION").to_string();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(crate::updater::UpdateEvent::CheckComplete {
                        result: Err(format!("Runtime error: {e}")),
                        is_manual,
                    });
                    return;
                }
            };

            let result = rt.block_on(async {
                crate::updater::check_github_release(crate::updater::GITHUB_REPO, &current_ver).await
            });

            let _ = tx.send(crate::updater::UpdateEvent::CheckComplete {
                result,
                is_manual,
            });
        });
    }

    pub fn start_update(&mut self, info: crate::updater::ReleaseInfo) {
        if self.is_downloading_update {
            return;
        }
        self.is_downloading_update = true;
        self.gui_closed_during_update = false;
        crate::updater::set_gui_closed_during_update(false);
        self.update_download_progress = 0.0;
        self.set_toast(self.loc().toast_update_downloading.replace("{}", &info.version));

        let tx = self.update_event_tx.clone();
        let asset_url = info.asset_url.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(crate::updater::UpdateEvent::DownloadComplete(Err(format!("Runtime error: {e}"))));
                    return;
                }
            };

            let tx_progress = tx.clone();
            let result = rt.block_on(async move {
                crate::updater::download_and_install(&asset_url, move |pct| {
                    let _ = tx_progress.send(crate::updater::UpdateEvent::DownloadProgress(pct));
                }).await
            });

            let _ = tx.send(crate::updater::UpdateEvent::DownloadComplete(result));
        });
    }

    pub fn poll_update_events(&mut self) {
        while let Ok(event) = self.update_event_rx.try_recv() {
            match event {
                crate::updater::UpdateEvent::CheckStarted => {
                    self.is_checking_update = true;
                }
                crate::updater::UpdateEvent::CheckComplete { result, is_manual } => {
                    self.is_checking_update = false;
                    match result {
                        Ok(Some(info)) => {
                            let ver = info.version.clone();
                            self.available_update = Some(info);
                            if is_manual {
                                self.set_toast(self.loc().toast_update_available.replace("{}", &ver));
                            }
                        }
                        Ok(None) => {
                            if is_manual {
                                self.set_toast(self.loc().toast_latest_version.replace("{}", env!("CARGO_PKG_VERSION")));
                            }
                        }
                        Err(err) => {
                            if is_manual {
                                self.set_toast(self.loc().toast_update_check_failed.replace("{}", &err));
                            }
                        }
                    }
                }
                crate::updater::UpdateEvent::DownloadProgress(pct) => {
                    self.update_download_progress = pct;
                }
                crate::updater::UpdateEvent::DownloadComplete(result) => {
                    self.is_downloading_update = false;
                    match result {
                        Ok(()) => {
                            // The binary was swapped and PowerShell restart helper spawned.
                            // Exit GUI cleanly so PowerShell continues immediately.
                            std::process::exit(0);
                        }
                        Err(err) => {
                            self.set_toast(self.loc().toast_update_failed.replace("{}", &err));
                            if self.gui_closed_during_update {
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn apply_process_to_action(action: &mut crate::models::Action, proc_name: &str) {
    match action {
        crate::models::Action::KillProcess { process_name, .. } => {
            *process_name = proc_name.to_string();
        }
        crate::models::Action::AudioControl { target, process_name, .. } => {
            if *target == crate::models::AudioTarget::Process {
                *process_name = Some(proc_name.to_string());
            }
        }
        crate::models::Action::Firewall { process_name, .. } => {
            *process_name = proc_name.to_string();
        }
        crate::models::Action::Window { target_process, .. } => {
            *target_process = Some(proc_name.to_string());
        }
        crate::models::Action::ProcessTuning { target_process, .. } => {
            *target_process = Some(proc_name.to_string());
        }
        _ => {}
    }
}

fn apply_process_to_condition(cond: &mut crate::models::RuleCondition, proc_name: &str) {
    match cond {
        crate::models::RuleCondition::ProcessRunning { process_name, .. } => {
            *process_name = proc_name.to_string();
        }
        crate::models::RuleCondition::WindowFocused { process_name, .. } => {
            *process_name = proc_name.to_string();
        }
        crate::models::RuleCondition::ResourceThreshold { process_name, .. } => {
            *process_name = proc_name.to_string();
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_modal_open() {
        let mut state = GuiState::new();
        assert!(!state.is_modal_open());

        // Process picker
        state.open_process_picker(ProcessPickerTarget::Trigger);
        assert!(state.is_modal_open());
        state.close_process_picker();
        assert!(!state.is_modal_open());

        // Action catalog
        state.action_catalog = Some(ActionCatalogModalState {
            is_exit_pipeline: false,
            insert_idx: None,
            search_query: String::new(),
            selected_category: 0,
            ifelse_target: None,
        });
        assert!(state.is_modal_open());
        state.action_catalog = None;
        assert!(!state.is_modal_open());

        // Script editor
        state.expanded_script_editor = Some(ExpandedScriptEditorState {
            step_idx: 0,
            is_exit_pipeline: false,
            language: crate::gui::components::script_highlighter::ScriptLanguage::PowerShell,
        });
        assert!(state.is_modal_open());
        state.expanded_script_editor = None;
        assert!(!state.is_modal_open());

        // Changelog modal (requires show_changelog_modal && available_update.is_some())
        state.show_changelog_modal = true;
        assert!(!state.is_modal_open());
        state.available_update = Some(crate::updater::ReleaseInfo {
            version: "1.4.0".into(),
            tag_name: "v1.4.0".into(),
            name: "v1.4.0".into(),
            body: "Notes".into(),
            html_url: "http://example.com".into(),
            asset_url: "http://example.com/asset.zip".into(),
            asset_name: "asset.zip".into(),
            asset_size: 1024,
        });
        assert!(state.is_modal_open());
        state.show_changelog_modal = false;
        assert!(!state.is_modal_open());

        // Guide modal
        state.show_guide_modal = true;
        assert!(state.is_modal_open());
        state.show_guide_modal = false;
        assert!(!state.is_modal_open());

        // Macro editor
        state.open_macro_editor(0, false);
        assert!(state.is_modal_open());
        state.close_macro_editor();
        assert!(!state.is_modal_open());
    }

    #[test]
    fn test_undo_redo_multi_step() {
        let mut state = GuiState::new();
        let mut r0 = state.initial_new_rule();
        r0.name = "Step 0".to_string();
        state.editing_rule = Some(r0.clone());

        let mut r1 = r0.clone();
        r1.name = "Step 1".to_string();
        state.push_undo_snapshot_of(&r0);
        state.editing_rule = Some(r1.clone());

        let mut r2 = r1.clone();
        r2.name = "Step 2".to_string();
        state.push_undo_snapshot_of(&r1);
        state.editing_rule = Some(r2.clone());

        assert_eq!(state.editing_rule.as_ref().unwrap().name, "Step 2");
        assert_eq!(state.undo_stack.len(), 2);
        assert_eq!(state.redo_stack.len(), 0);

        // Undo 1 -> Step 1
        state.undo();
        assert_eq!(state.editing_rule.as_ref().unwrap().name, "Step 1");
        assert_eq!(state.undo_stack.len(), 1);
        assert_eq!(state.redo_stack.len(), 1);

        // Undo 2 -> Step 0
        state.undo();
        assert_eq!(state.editing_rule.as_ref().unwrap().name, "Step 0");
        assert_eq!(state.undo_stack.len(), 0);
        assert_eq!(state.redo_stack.len(), 2);

        // Redo 1 -> Step 1
        state.redo();
        assert_eq!(state.editing_rule.as_ref().unwrap().name, "Step 1");
        assert_eq!(state.undo_stack.len(), 1);
        assert_eq!(state.redo_stack.len(), 1);

        // Redo 2 -> Step 2
        state.redo();
        assert_eq!(state.editing_rule.as_ref().unwrap().name, "Step 2");
        assert_eq!(state.undo_stack.len(), 2);
        assert_eq!(state.redo_stack.len(), 0);
    }
}

