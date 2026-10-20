use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use crate::daemon::engine::DaemonEngine;
use crate::models::Trigger;

pub struct FileWatcherService {
    engine: Arc<Mutex<DaemonEngine>>,
    known_dir_states: HashMap<PathBuf, HashMap<PathBuf, (SystemTime, u64)>>,
}

impl FileWatcherService {
    pub fn new(engine: Arc<Mutex<DaemonEngine>>) -> Self {
        Self {
            engine,
            known_dir_states: HashMap::new(),
        }
    }

    pub async fn poll_once(&mut self) {
        let (rules, _is_empty) = {
            let eng = self.engine.lock().await;
            (eng.config.rules.clone(), eng.config.rules.is_empty())
        };

        for rule in &rules {
            if !rule.is_enabled {
                continue;
            }

            if let Some(Trigger::FileWatcher {
                folder_path,
                filter_extension,
                on_created,
                on_modified,
                on_deleted,
            }) = &rule.trigger
            {
                let target_path = PathBuf::from(folder_path);
                let prev_state = self.known_dir_states.get(&target_path).cloned();

                let current_state = if target_path.is_file() {
                    Self::scan_file(&target_path)
                } else if target_path.is_dir() {
                    Self::scan_dir(&target_path, filter_extension.as_deref())
                } else {
                    HashMap::new()
                };

                if let Some(prev) = prev_state {
                    // 1. Created files
                    if *on_created {
                        for (file, _) in &current_state {
                            if !prev.contains_key(file) {
                                self.trigger_file_event(rule, "File Created", file).await;
                            }
                        }
                    }

                    // 2. Modified files
                    if *on_modified {
                        for (file, &(mtime, size)) in &current_state {
                            if let Some(&(prev_mtime, prev_size)) = prev.get(file) {
                                if mtime != prev_mtime || size != prev_size {
                                    self.trigger_file_event(rule, "File Modified", file).await;
                                }
                            }
                        }
                    }

                    // 3. Deleted files
                    if *on_deleted {
                        for (file, _) in &prev {
                            if !current_state.contains_key(file) {
                                self.trigger_file_event(rule, "File Deleted", file).await;
                            }
                        }
                    }
                }

                self.known_dir_states.insert(target_path, current_state);
            }
        }
    }

    async fn trigger_file_event(&self, rule: &crate::models::Rule, event_type: &str, file_path: &Path) {
        let file_str = file_path.to_string_lossy().to_string();
        let file_name = file_path.file_name().and_then(|f| f.to_str()).unwrap_or_default().to_string();
        let file_dir = file_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

        let mut eng = self.engine.lock().await;
        let mut custom_ctx = HashMap::new();
        custom_ctx.insert("FilePath".to_string(), file_str);
        custom_ctx.insert("FileName".to_string(), file_name);
        custom_ctx.insert("FileDir".to_string(), file_dir);

        let _ = eng.try_execute_rule_with_context(rule, event_type, 0, custom_ctx).await;
    }

    fn scan_file(file: &Path) -> HashMap<PathBuf, (SystemTime, u64)> {
        let mut map = HashMap::new();
        if let Ok(meta) = std::fs::metadata(file) {
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let size = meta.len();
            map.insert(file.to_path_buf(), (mtime, size));
        }
        map
    }

    fn scan_dir(dir: &Path, filter_ext: Option<&str>) -> HashMap<PathBuf, (SystemTime, u64)> {
        let mut map = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = filter_ext {
                        let matches_ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.eq_ignore_ascii_case(ext.trim_start_matches('.')))
                            .unwrap_or(false);
                        if !matches_ext {
                            continue;
                        }
                    }

                    if let Ok(meta) = entry.metadata() {
                        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                        let size = meta.len();
                        map.insert(path, (mtime, size));
                    }
                }
            }
        }
        map
    }
}
