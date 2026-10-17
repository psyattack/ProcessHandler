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
    pub language: crate::i18n::Language,
    pub last_sync: Instant,
    pub toast: Option<(String, Instant)>,
    pub is_daemon_connected: bool,
    pub icon_cache: crate::gui::components::icons::ProcessIconCache,
    pub available_update: Option<crate::updater::ReleaseInfo>,
    pub is_checking_update: bool,
    pub is_downloading_update: bool,
    pub update_download_progress: f32,
    pub show_changelog_modal: bool,
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
            expanded_script_editor: None,
            last_script_editor: None,
            language: config.language,
            last_sync: Instant::now() - std::time::Duration::from_secs(10),
            toast: None,
            is_daemon_connected: false,
            icon_cache: crate::gui::components::icons::ProcessIconCache::new(),
            available_update: None,
            is_checking_update: false,
            is_downloading_update: false,
            update_download_progress: 0.0,
            show_changelog_modal: false,
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
                    if let Err(err) = result {
                        self.set_toast(self.loc().toast_update_failed.replace("{}", &err));
                    }
                }
            }
        }
    }
}
