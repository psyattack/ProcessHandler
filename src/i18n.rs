pub mod en;
pub mod ru;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    English,
    Russian,
}

impl Language {
    pub fn label(&self) -> &'static str {
        match self {
            Self::English => "ENGLISH",
            Self::Russian => "РУССКИЙ",
        }
    }
}

pub struct Strings {
    // Navigation Tabs (header_bar.rs)
    pub tab_automations: &'static str,
    pub tab_automations_tip: &'static str,
    pub tab_hunter: &'static str,
    pub tab_hunter_tip: &'static str,
    pub tab_logs: &'static str,
    pub tab_logs_tip: &'static str,
    pub tab_settings: &'static str,
    pub tab_settings_tip: &'static str,

    // Window Chrome & Title Bar (widgets.rs, header_bar.rs)
    pub win_btn_minimize: &'static str,
    pub win_btn_maximize: &'static str,
    pub win_btn_restore: &'static str,
    pub win_btn_close: &'static str,

    // Tray Menu (tray.rs)
    pub tray_menu_open: &'static str,
    pub tray_menu_exit: &'static str,

    // Footer (gui.rs)
    pub footer_rules_active: &'static str,
    pub footer_autorun_on: &'static str,
    pub footer_autorun_off: &'static str,

    // Rules View (rules_view.rs)
    pub rules_cell_all: &'static str,
    pub rules_cell_all_tip: &'static str,
    pub rules_cell_active: &'static str,
    pub rules_cell_active_tip: &'static str,
    pub rules_cell_paused: &'static str,
    pub rules_cell_paused_tip: &'static str,
    pub rules_btn_new: &'static str,
    pub rules_search_placeholder: &'static str,
    pub rules_empty_title: &'static str,
    pub rules_btn_create_first: &'static str,
    pub rules_badge_automation: &'static str,
    pub rules_badge_active: &'static str,
    pub rules_badge_paused: &'static str,
    pub rules_badge_cooldown_suffix: &'static str,
    pub rules_btn_delete: &'static str,
    pub rules_btn_delete_tip: &'static str,
    pub rules_btn_edit: &'static str,
    pub rules_btn_edit_tip: &'static str,
    pub rules_btn_clone: &'static str,
    pub rules_btn_clone_tip: &'static str,
    pub rules_btn_pause: &'static str,
    pub rules_btn_resume: &'static str,
    pub rules_btn_test_run: &'static str,
    pub rules_btn_test_run_tip: &'static str,
    pub rules_btn_copy_all: &'static str,
    pub rules_btn_copy_all_tip: &'static str,
    pub rules_btn_import: &'static str,
    pub rules_btn_import_tip: &'static str,
    pub rules_btn_copy_single: &'static str,
    pub rules_btn_copy_single_tip: &'static str,
    pub rules_btn_more: &'static str,
    pub rules_btn_more_tip: &'static str,
    pub rules_btn_collapse: &'static str,
    pub rules_badge_running: &'static str,
    pub rules_badge_step: &'static str,
    pub rules_badge_concurrent_suffix: &'static str,
    pub rules_summary_step_active: &'static str,
    pub rules_summary_trigger_process: &'static str,
    pub rules_summary_single_instance: &'static str,
    pub rules_summary_trigger_system: &'static str,
    pub rules_summary_every_year: &'static str,
    pub rules_summary_every_month: &'static str,
    pub rules_summary_every_week: &'static str,
    pub rules_summary_every_day: &'static str,
    pub rules_summary_every_hour: &'static str,
    pub rules_summary_every_minute: &'static str,
    pub rules_summary_every_second: &'static str,
    pub rules_summary_trigger_daily: &'static str,
    pub rules_summary_trigger_once: &'static str,
    pub rules_summary_trigger_startup: &'static str,
    pub rules_summary_trigger_shutdown: &'static str,
    pub rules_summary_trigger_none: &'static str,
    pub rules_summary_no_actions: &'static str,
    pub rules_summary_actions: &'static str,
    pub rules_summary_bound_satellite: &'static str,
    pub rules_summary_on_exit: &'static str,
    pub rules_default_new_name: &'static str,
    pub summary_action_launch: &'static str,
    pub summary_action_kill: &'static str,
    pub summary_action_mute: &'static str,
    pub summary_action_unmute: &'static str,
    pub summary_action_toggle: &'static str,
    pub summary_action_vol: &'static str,
    pub summary_action_wait: &'static str,
    pub summary_action_power: &'static str,

    // Process Hunter View (processes_view.rs)
    pub hunter_cell_all: &'static str,
    pub hunter_cell_all_tip: &'static str,
    pub hunter_cell_windowed: &'static str,
    pub hunter_cell_windowed_tip: &'static str,
    pub hunter_cell_services: &'static str,
    pub hunter_cell_services_tip: &'static str,
    pub hunter_btn_rescan: &'static str,
    pub hunter_search_placeholder: &'static str,
    pub hunter_empty_title: &'static str,
    pub hunter_col_process_name: &'static str,
    pub hunter_col_pid: &'static str,
    pub hunter_col_working_set: &'static str,
    pub hunter_col_window_title: &'static str,
    pub hunter_col_quick_actions: &'static str,
    pub hunter_btn_rule: &'static str,
    pub hunter_btn_rule_tip: &'static str,
    pub hunter_btn_select: &'static str,
    pub hunter_btn_select_tip: &'static str,
    pub hunter_modal_title: &'static str,
    pub hunter_modal_subtitle: &'static str,

    // Activity Log Stream View (logs_view.rs)
    pub logs_cell_all: &'static str,
    pub logs_cell_all_tip: &'static str,
    pub logs_cell_rules: &'static str,
    pub logs_cell_rules_tip: &'static str,
    pub logs_cell_processes: &'static str,
    pub logs_cell_processes_tip: &'static str,
    pub logs_cell_system: &'static str,
    pub logs_cell_system_tip: &'static str,
    pub logs_cell_errors: &'static str,
    pub logs_cell_errors_tip: &'static str,
    pub logs_btn_clear: &'static str,
    pub logs_search_placeholder: &'static str,
    pub logs_empty_title: &'static str,
    pub logs_empty_subtitle: &'static str,

    // Settings View (settings_view.rs)
    pub settings_daemon_offline: &'static str,
    pub settings_eyebrow_system: &'static str,
    pub settings_btn_enable_autorun: &'static str,
    pub settings_btn_disable_autorun: &'static str,
    pub settings_tray_icon_override: &'static str,
    pub settings_btn_browse: &'static str,
    pub settings_btn_reset: &'static str,
    pub settings_tray_icon_placeholder: &'static str,
    pub settings_eyebrow_kernel: &'static str,
    pub settings_monitoring_mode: &'static str,
    pub settings_mode_polling: &'static str,
    pub settings_mode_polling_tip: &'static str,
    pub settings_mode_wmi: &'static str,
    pub settings_mode_wmi_tip: &'static str,
    pub settings_scan_interval: &'static str,
    pub settings_eyebrow_language: &'static str,
    pub settings_language_label: &'static str,
    pub settings_eyebrow_appearance: &'static str,
    pub settings_theme_label: &'static str,
    pub settings_theme_dark: &'static str,
    pub settings_theme_dark_tip: &'static str,
    pub settings_theme_light: &'static str,
    pub settings_theme_light_tip: &'static str,
    pub settings_theme_chocolate: &'static str,
    pub settings_theme_chocolate_tip: &'static str,

    // Inspector Drawer (inspector.rs)
    pub inspector_eyebrow_builder: &'static str,
    pub inspector_eyebrow_task_details: &'static str,
    pub inspector_btn_close_tip: &'static str,
    pub inspector_field_rule_name: &'static str,
    pub inspector_field_description: &'static str,
    pub inspector_field_description_hint: &'static str,
    pub inspector_field_cooldown: &'static str,
    pub inspector_checkbox_rule_active: &'static str,
    pub inspector_eyebrow_trigger: &'static str,
    pub inspector_trigger_process: &'static str,
    pub inspector_trigger_system: &'static str,
    pub inspector_trigger_timer: &'static str,
    pub inspector_trigger_tab_process: &'static str,
    pub inspector_trigger_tab_schedule: &'static str,
    pub inspector_trigger_tab_events: &'static str,
    pub inspector_match_tab_exact: &'static str,
    pub inspector_match_tab_wildcard: &'static str,
    pub inspector_match_tab_regex: &'static str,
    pub inspector_sched_tab_interval: &'static str,
    pub inspector_sched_tab_daily: &'static str,
    pub inspector_sched_tab_specific: &'static str,
    pub inspector_sched_tab_app_start: &'static str,
    pub inspector_sched_tab_app_close: &'static str,
    pub inspector_field_target_process: &'static str,
    pub inspector_field_target_process_hint: &'static str,
    pub inspector_field_match_mode: &'static str,
    pub inspector_match_mode_exact: &'static str,
    pub inspector_match_mode_wildcard: &'static str,
    pub inspector_match_mode_contains: &'static str,
    pub inspector_checkbox_single_instance: &'static str,
    pub inspector_field_sys_event_type: &'static str,
    pub inspector_sys_event_lock: &'static str,
    pub inspector_sys_event_unlock: &'static str,
    pub inspector_sys_event_sleep: &'static str,
    pub inspector_sys_event_resume: &'static str,
    pub inspector_sys_event_ac: &'static str,
    pub inspector_sys_event_battery: &'static str,
    pub inspector_sys_event_idle: &'static str,
    pub inspector_sys_event_active: &'static str,
    pub inspector_field_idle_duration: &'static str,
    pub inspector_schedule_interval: &'static str,
    pub inspector_schedule_daily: &'static str,
    pub inspector_schedule_oneshot: &'static str,
    pub inspector_schedule_app_start: &'static str,
    pub inspector_schedule_app_close: &'static str,
    pub inspector_preset_intervals: &'static str,
    pub inspector_custom_interval: &'static str,
    pub inspector_interval_hours: &'static str,
    pub inspector_interval_minutes: &'static str,
    pub inspector_interval_seconds: &'static str,
    pub inspector_checkbox_first_execution: &'static str,
    pub inspector_daily_execution_time: &'static str,
    pub inspector_active_days_of_week: &'static str,
    pub inspector_dow_mon: &'static str,
    pub inspector_dow_tue: &'static str,
    pub inspector_dow_wed: &'static str,
    pub inspector_dow_thu: &'static str,
    pub inspector_dow_fri: &'static str,
    pub inspector_dow_sat: &'static str,
    pub inspector_dow_sun: &'static str,
    pub inspector_preset_tip_1s: &'static str,
    pub inspector_preset_tip_1m: &'static str,
    pub inspector_preset_tip_1h: &'static str,
    pub inspector_preset_tip_1d: &'static str,
    pub inspector_preset_tip_1w: &'static str,
    pub inspector_preset_tip_1mo: &'static str,
    pub inspector_preset_tip_1y: &'static str,
    pub inspector_btn_all_days: &'static str,
    pub inspector_mini_calendar: &'static str,
    pub inspector_cal_month_names: [&'static str; 12],
    pub inspector_cal_day_names: [&'static str; 7],
    pub inspector_execution_time: &'static str,
    pub inspector_badge_app_startup: &'static str,
    pub inspector_badge_app_shutdown: &'static str,
    pub inspector_eyebrow_actions_start: &'static str,
    pub inspector_eyebrow_actions_pipeline: &'static str,
    pub inspector_eyebrow_actions_exit: &'static str,
    pub inspector_btn_add_action: &'static str,
    pub inspector_btn_save_rule: &'static str,
    pub inspector_btn_clone: &'static str,
    pub inspector_btn_clone_tip: &'static str,
    pub inspector_btn_cancel: &'static str,
    pub inspector_btn_reset: &'static str,
    pub inspector_btn_reset_tip: &'static str,

    // Inspector Action Dropdown Menu Options
    pub inspector_menu_launch_process: &'static str,
    pub inspector_menu_kill_process: &'static str,
    pub inspector_menu_powershell: &'static str,
    pub inspector_menu_cmd: &'static str,
    pub inspector_menu_notification: &'static str,
    pub inspector_menu_sound: &'static str,
    pub inspector_menu_microphone: &'static str,
    pub inspector_menu_process_audio: &'static str,
    pub inspector_menu_system_power: &'static str,
    pub inspector_menu_delay: &'static str,
    pub inspector_menu_webhook: &'static str,
    pub inspector_search_action_hint: &'static str,
    pub inspector_no_matching_actions: &'static str,
    pub inspector_btn_add_action_tip: &'static str,
    pub inspector_btn_open_catalog: &'static str,

    // Action Catalog Modal
    pub catalog_modal_eyebrow: &'static str,
    pub catalog_modal_title: &'static str,
    pub catalog_target_start: &'static str,
    pub catalog_target_exit: &'static str,
    pub catalog_search_hint: &'static str,
    pub catalog_cat_all: &'static str,
    pub catalog_cat_process: &'static str,
    pub catalog_cat_audio: &'static str,
    pub catalog_cat_automation: &'static str,
    pub catalog_cat_system: &'static str,
    pub catalog_no_matches: &'static str,
    pub catalog_esc_hint: &'static str,
    pub catalog_actions_count: &'static str,

    // Action Descriptions for Catalog
    pub catalog_desc_launch_process: &'static str,
    pub catalog_desc_kill_process: &'static str,
    pub catalog_desc_sound: &'static str,
    pub catalog_desc_microphone: &'static str,
    pub catalog_desc_process_audio: &'static str,
    pub catalog_desc_powershell: &'static str,
    pub catalog_desc_cmd: &'static str,
    pub catalog_desc_notification: &'static str,
    pub catalog_desc_webhook: &'static str,
    pub catalog_desc_system_power: &'static str,
    pub catalog_desc_delay: &'static str,

    // Inspector Action Cards & Badges
    pub inspector_tip_drag_step: &'static str,
    pub inspector_badge_moving_step: &'static str,
    pub inspector_tip_remove_step: &'static str,
    pub inspector_tip_clone_step: &'static str,
    pub inspector_step_prefix: &'static str,
    pub inspector_exit_prefix: &'static str,
    pub inspector_badge_launch_companion: &'static str,
    pub inspector_badge_launch_process: &'static str,
    pub inspector_badge_kill_process: &'static str,
    pub inspector_badge_powershell: &'static str,
    pub inspector_badge_cmd: &'static str,
    pub inspector_badge_notification: &'static str,
    pub inspector_badge_audio_volume: &'static str,
    pub inspector_badge_microphone: &'static str,
    pub inspector_badge_process_audio: &'static str,
    pub inspector_badge_system_power: &'static str,
    pub inspector_badge_delay: &'static str,
    pub inspector_badge_webhook: &'static str,

    // Inspector Action Fields
    pub inspector_field_executable_path: &'static str,
    pub inspector_field_executable_hint: &'static str,
    pub inspector_field_arguments: &'static str,
    pub inspector_field_arguments_hint: &'static str,
    pub inspector_field_working_directory: &'static str,
    pub inspector_field_working_directory_hint: &'static str,
    pub inspector_btn_folder: &'static str,
    pub inspector_checkbox_run_hidden: &'static str,
    pub inspector_checkbox_only_if_not_running: &'static str,
    pub inspector_checkbox_run_as_admin: &'static str,
    pub inspector_checkbox_close_on_exit: &'static str,
    pub inspector_field_terminate_process_name: &'static str,
    pub inspector_field_terminate_hint: &'static str,
    pub inspector_checkbox_kill_tree: &'static str,
    pub inspector_checkbox_force_terminate: &'static str,
    pub inspector_field_powershell_script: &'static str,
    pub inspector_tip_expand_pwsh: &'static str,
    pub inspector_field_cmd_prompt: &'static str,
    pub inspector_tip_expand_cmd: &'static str,
    pub inspector_checkbox_visible_console: &'static str,
    pub inspector_field_notif_title: &'static str,
    pub inspector_field_notif_message: &'static str,
    pub inspector_field_audio_action_type: &'static str,
    pub inspector_audio_beep: &'static str,
    pub inspector_audio_mute: &'static str,
    pub inspector_audio_unmute: &'static str,
    pub inspector_audio_toggle_mute: &'static str,
    pub inspector_audio_set_volume: &'static str,
    pub inspector_field_volume_level: &'static str,
    pub inspector_field_mic_action_type: &'static str,
    pub inspector_field_process_audio_action: &'static str,
    pub inspector_field_power_action: &'static str,
    pub inspector_power_lock: &'static str,
    pub inspector_power_sleep: &'static str,
    pub inspector_power_screen_off: &'static str,
    pub inspector_power_restart: &'static str,
    pub inspector_power_shutdown: &'static str,
    pub inspector_field_pause_delay: &'static str,
    pub inspector_field_webhook_url: &'static str,
    pub inspector_field_webhook_url_hint: &'static str,
    pub inspector_field_json_body: &'static str,

    // Inspector Process Details
    pub inspector_field_window_title: &'static str,
    pub inspector_no_active_window: &'static str,
    pub inspector_field_working_set_ram: &'static str,
    pub inspector_bytes_suffix: &'static str,
    pub inspector_eyebrow_quick_actions: &'static str,
    pub inspector_btn_create_automation: &'static str,
    pub inspector_btn_terminate_process: &'static str,
    pub inspector_rule_track_prefix: &'static str,
    pub inspector_pick_running: &'static str,
    pub inspector_search_process_hint: &'static str,
    pub inspector_no_matching_processes: &'static str,

    // Inspector Rule Icon (inspector.rs)
    pub inspector_sec_icon: &'static str,
    pub inspector_icon_browse: &'static str,
    pub inspector_icon_browse_tip: &'static str,
    pub inspector_icon_reset: &'static str,
    pub inspector_icon_reset_tip: &'static str,
    pub inspector_icon_auto_process: &'static str,
    pub inspector_icon_auto_process_tip: &'static str,
    pub inspector_icon_detect_now: &'static str,
    pub inspector_icon_detect_now_tip: &'static str,
    pub inspector_icon_default_label: &'static str,
    pub inspector_icon_custom_label: &'static str,
    pub inspector_badge_executing_now: &'static str,

    // Script Modal (script_modal.rs)
    pub modal_tip_close: &'static str,
    pub modal_lines: &'static str,
    pub modal_bytes: &'static str,
    pub modal_esc_hint: &'static str,
    pub modal_btn_done: &'static str,
    pub modal_btn_copy_script: &'static str,
    pub modal_pwsh_subtitle: &'static str,
    pub modal_cmd_subtitle: &'static str,

    // Toasts
    pub toast_script_copied: &'static str,
    pub toast_process_refreshed: &'static str,
    pub toast_stream_cleared: &'static str,
    pub toast_config_saved: &'static str,
    pub toast_rule_saved: &'static str,
    pub toast_rule_deleted: &'static str,
    pub toast_rule_cloned: &'static str,
    pub toast_test_execution: &'static str,
    pub toast_process_terminated: &'static str,
    pub toast_autorun_enabled: &'static str,
    pub toast_autorun_disabled: &'static str,
    pub toast_lang_changed: &'static str,
    pub toast_theme_changed: &'static str,
    pub toast_copy_single_success: &'static str,
    pub toast_copy_all_success: &'static str,
    pub toast_copy_all_empty: &'static str,
    pub toast_import_success: &'static str,
    pub toast_import_invalid: &'static str,
    pub toast_import_empty: &'static str,
    pub toast_import_clipboard_err: &'static str,
    pub toast_icon_detected: &'static str,
    pub toast_icon_not_found: &'static str,

    // Settings Updates (settings_view.rs)
    pub settings_eyebrow_updates: &'static str,
    pub settings_check_startup_checkbox: &'static str,
    pub settings_btn_check_updates: &'static str,
    pub settings_btn_checking_updates: &'static str,
    pub settings_last_checked: &'static str,

    // Footer Updates (gui.rs)
    pub footer_update_available: &'static str,
    pub footer_btn_update: &'static str,
    pub footer_btn_info: &'static str,
    pub footer_btn_info_tip: &'static str,
    pub footer_btn_update_tip: &'static str,
    pub footer_downloading_progress: &'static str,

    // Changelog Modal (changelog_modal.rs)
    pub changelog_eyebrow: &'static str,
    pub changelog_modal_title: &'static str,
    pub changelog_btn_close: &'static str,
    pub changelog_btn_update: &'static str,
    pub changelog_open_github: &'static str,
    pub changelog_no_notes: &'static str,

    // Update Toasts
    pub toast_update_available: &'static str,
    pub toast_latest_version: &'static str,
    pub toast_update_check_failed: &'static str,
    pub toast_update_downloading: &'static str,
    pub toast_update_failed: &'static str,
}

pub fn get(lang: Language) -> &'static Strings {
    match lang {
        Language::English => &en::STRINGS,
        Language::Russian => &ru::STRINGS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_defaults_and_labels() {
        assert_eq!(Language::default(), Language::English);
        assert_eq!(Language::English.label(), "ENGLISH");
        assert_eq!(Language::Russian.label(), "РУССКИЙ");
    }

    #[test]
    fn test_language_serde_roundtrip() {
        let json_en = serde_json::to_string(&Language::English).unwrap();
        assert_eq!(json_en, "\"english\"");
        let parsed_en: Language = serde_json::from_str(&json_en).unwrap();
        assert_eq!(parsed_en, Language::English);

        let json_ru = serde_json::to_string(&Language::Russian).unwrap();
        assert_eq!(json_ru, "\"russian\"");
        let parsed_ru: Language = serde_json::from_str(&json_ru).unwrap();
        assert_eq!(parsed_ru, Language::Russian);
    }

    #[test]
    fn test_get_language_catalogs() {
        let en = get(Language::English);
        let ru = get(Language::Russian);

        assert_eq!(en.tab_automations, "AUTOMATIONS");
        assert_eq!(ru.tab_automations, "АВТОМАТИЗАЦИЯ");

        assert_eq!(en.rules_cell_all, "ALL RULES");
        assert_eq!(ru.rules_cell_all, "ВСЕ ПРАВИЛА");

        assert_eq!(en.hunter_btn_rule, "+ RULE");
        assert_eq!(ru.hunter_btn_rule, "+ ПРАВИЛО");

        assert_eq!(en.settings_language_label, "ACTIVE LANGUAGE:");
        assert_eq!(ru.settings_language_label, "ТЕКУЩИЙ ЯЗЫК:");
        assert_eq!(en.settings_eyebrow_language, "INTERFACE LANGUAGE");
        assert_eq!(ru.settings_eyebrow_language, "ЯЗЫК ИНТЕРФЕЙСА");
        assert_eq!(en.settings_eyebrow_appearance, "APPEARANCE & COLOR THEME");
        assert_eq!(ru.settings_eyebrow_appearance, "ОФОРМЛЕНИЕ И ЦВЕТОВАЯ ТЕМА");
        assert_eq!(en.settings_theme_chocolate, "CHOCOLATE");
        assert_eq!(ru.settings_theme_chocolate, "ШОКОЛАДНАЯ");

        assert_eq!(en.settings_eyebrow_updates, "APPLICATION UPDATES");
        assert_eq!(ru.settings_eyebrow_updates, "ОБНОВЛЕНИЕ ПРИЛОЖЕНИЯ");
        assert_eq!(en.footer_btn_update, "UPDATE");
        assert_eq!(ru.footer_btn_update, "ОБНОВИТЬ");
        assert_eq!(en.footer_btn_info, "INFO");
        assert_eq!(ru.footer_btn_info, "ИНФО");
    }
}
