use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::models::Rule;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub is_maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            width: 1180.0,
            height: 760.0,
            is_maximized: false,
        }
    }
}

impl WindowState {
    pub fn sanitize(&mut self) {
        self.width = self.width.clamp(880.0, 3840.0);
        self.height = self.height.clamp(580.0, 2160.0);
        if self.x < -4000.0 || self.x > 8000.0 || self.y < -1000.0 || self.y > 5000.0 {
            self.x = 100.0;
            self.y = 100.0;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    Chocolate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default = "default_monitoring_mode")]
    pub monitoring_mode: String, // "polling" or "wmi"

    #[serde(default = "default_poll_interval")]
    pub poll_interval_sec: f64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tray_icon_path: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowState>,

    #[serde(default)]
    pub rules: Vec<Rule>,

    #[serde(default)]
    pub autorun_configured: bool,

    #[serde(default)]
    pub hide_tray_icon: bool,

    #[serde(default)]
    pub mask_daemon_process: bool,

    #[serde(default)]
    pub language: crate::i18n::Language,

    #[serde(default = "default_check_updates_on_startup")]
    pub check_updates_on_startup: bool,

    #[serde(default)]
    pub theme: AppTheme,

    #[serde(default = "default_config_overlays")]
    pub overlays: Vec<crate::models::OverlayDefinition>,
}

pub fn default_config_overlays() -> Vec<crate::models::OverlayDefinition> {
    vec![
        crate::models::OverlayDefinition::default_crosshair(),
        crate::models::OverlayDefinition::default_macro_badge(),
        crate::models::OverlayDefinition::default_telemetry(),
    ]
}

fn default_check_updates_on_startup() -> bool {
    true
}

fn default_monitoring_mode() -> String {
    "wmi".to_string()
}

fn default_poll_interval() -> f64 {
    2.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            monitoring_mode: "wmi".into(),
            poll_interval_sec: 2.0,
            tray_icon_path: None,
            hide_tray_icon: false,
            mask_daemon_process: false,
            window: None,
            rules: Vec::new(),
            autorun_configured: false,
            language: crate::i18n::Language::English,
            check_updates_on_startup: true,
            theme: AppTheme::Dark,
            overlays: default_config_overlays(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        Self::load_from_file(Self::config_path())
    }
    pub fn config_dir() -> PathBuf {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let dir = PathBuf::from(local).join("ProcessHandler");
            let _ = fs::create_dir_all(&dir);
            return dir;
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            let dir = PathBuf::from(profile).join("AppData").join("Local").join("ProcessHandler");
            let _ = fs::create_dir_all(&dir);
            return dir;
        }
        let dir = PathBuf::from("ProcessHandler");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        let is_app_config = !cfg!(test) && (path == Self::config_path() || path.ends_with("config.json"));

        if !path.exists() {
            let mut default_cfg = Self::default();
            if is_app_config {
                default_cfg.autorun_configured = true;
                let mask = default_cfg.mask_daemon_process;
                let _ = default_cfg.save_to_file(path);
                let _ = crate::autorun::set_autorun_with_mask(crate::autorun::APP_NAME, true, mask);
            } else {
                let _ = default_cfg.save_to_file(path);
            }
            return default_cfg;
        }

        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                Ok(mut cfg) => {
                    if is_app_config && !cfg.autorun_configured {
                        cfg.autorun_configured = true;
                        let mask = cfg.mask_daemon_process;
                        let _ = cfg.save_to_file(path);
                        if !crate::autorun::is_autorun_enabled(crate::autorun::APP_NAME) {
                            let _ = crate::autorun::set_autorun_with_mask(crate::autorun::APP_NAME, true, mask);
                        }
                    }
                    if let Some(ws) = &mut cfg.window {
                        ws.sanitize();
                    }
                    cfg
                }
                Err(err) => {
                    eprintln!("Failed to parse config.json: {err}. Using default.");
                    Self::default()
                }
            },
            Err(err) => {
                eprintln!("Failed to read config.json: {err}. Using default.");
                Self::default()
            }
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn save_window_state(ws: &WindowState) {
        let path = Self::config_path();
        let mut cfg = Self::load_from_file(&path);
        cfg.window = Some(ws.clone());
        let _ = cfg.save_to_file(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_autorun_configured_default() {
        let json = r#"{"monitoring_mode": "wmi", "poll_interval_sec": 2.0}"#;
        let cfg: AppConfig = serde_json::from_str(json).expect("deserialize");
        assert!(!cfg.autorun_configured);
        assert!(cfg.check_updates_on_startup);
        assert_eq!(cfg.overlays.len(), 3);
    }

    #[test]
    fn test_app_config_autorun_configured_serde() {
        let cfg = AppConfig {
            autorun_configured: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let parsed: AppConfig = serde_json::from_str(&json).expect("deserialize");
        assert!(parsed.autorun_configured);
    }

    #[test]
    fn test_app_theme_serde_and_default() {
        assert_eq!(AppTheme::default(), AppTheme::Dark);

        let dark_json = serde_json::to_string(&AppTheme::Dark).unwrap();
        assert_eq!(dark_json, "\"dark\"");
        assert_eq!(serde_json::from_str::<AppTheme>(&dark_json).unwrap(), AppTheme::Dark);

        let light_json = serde_json::to_string(&AppTheme::Light).unwrap();
        assert_eq!(light_json, "\"light\"");
        assert_eq!(serde_json::from_str::<AppTheme>(&light_json).unwrap(), AppTheme::Light);

        let choco_json = serde_json::to_string(&AppTheme::Chocolate).unwrap();
        assert_eq!(choco_json, "\"chocolate\"");
        assert_eq!(serde_json::from_str::<AppTheme>(&choco_json).unwrap(), AppTheme::Chocolate);
    }
}
