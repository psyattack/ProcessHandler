use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::models::{Rule, Trigger};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeyEvent {
    Trigger(String),
    HoldStart(String),
    HoldStop(String),
}

#[derive(Clone, Debug)]
pub struct MonitoredHotkey {
    pub rule_id: String,
    pub vkeys: Vec<i32>,
    pub hold_mode: bool,
    pub repeat_while_held: bool,
    pub was_pressed: bool,
}

static MONITORED_HOTKEYS: Mutex<Vec<MonitoredHotkey>> = Mutex::new(Vec::new());
static LISTENER_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn is_hotkey_held(rule_id: &str) -> bool {
    if let Ok(lock) = MONITORED_HOTKEYS.lock() {
        if let Some(hk) = lock.iter().find(|h| h.rule_id == rule_id) {
            return hk.was_pressed;
        }
    }
    false
}

pub fn is_hotkey_repeat_held(rule_id: &str) -> bool {
    if let Ok(lock) = MONITORED_HOTKEYS.lock() {
        if let Some(hk) = lock.iter().find(|h| h.rule_id == rule_id) {
            return hk.was_pressed && hk.repeat_while_held;
        }
    }
    false
}

#[derive(Clone)]
pub struct HotkeyHandle {
    _private: (),
}

impl HotkeyHandle {
    pub fn reload(&self, rules: &[Rule]) {
        let mut list = Vec::new();
        for r in rules {
            if r.is_enabled {
                if let Some(Trigger::GlobalHotkey { key_combination, hold_mode, repeat_while_held }) = &r.trigger {
                    if let Some(vkeys) = parse_hotkey_vkeys(key_combination) {
                        list.push(MonitoredHotkey {
                            rule_id: r.id.clone(),
                            vkeys,
                            hold_mode: *hold_mode,
                            repeat_while_held: *repeat_while_held,
                            was_pressed: false,
                        });
                    }
                }
            }
        }

        if let Ok(mut lock) = MONITORED_HOTKEYS.lock() {
            *lock = list;
        }
    }
}

pub fn start_hotkey_listener(event_tx: tokio::sync::mpsc::UnboundedSender<HotkeyEvent>) -> HotkeyHandle {
    LISTENER_RUNNING.store(true, Ordering::SeqCst);

    #[cfg(windows)]
    {
        std::thread::Builder::new()
            .name("global-hotkey-listener".into())
            .spawn(move || {
                run_hotkey_poll_loop(event_tx);
            })
            .expect("Failed to spawn global hotkey thread");
    }

    #[cfg(not(windows))]
    {
        let _ = event_tx;
    }

    HotkeyHandle { _private: () }
}

#[cfg(windows)]
fn run_hotkey_poll_loop(event_tx: tokio::sync::mpsc::UnboundedSender<HotkeyEvent>) {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    while LISTENER_RUNNING.load(Ordering::SeqCst) {
        if let Ok(mut lock) = MONITORED_HOTKEYS.lock() {
            for hotkey in lock.iter_mut() {
                let is_down = !hotkey.vkeys.is_empty() && hotkey.vkeys.iter().all(|&vk| {
                    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
                });

                if hotkey.hold_mode {
                    if !hotkey.was_pressed && is_down {
                        hotkey.was_pressed = true;
                        let _ = event_tx.send(HotkeyEvent::HoldStart(hotkey.rule_id.clone()));
                    } else if hotkey.was_pressed && !is_down {
                        hotkey.was_pressed = false;
                        let _ = event_tx.send(HotkeyEvent::HoldStop(hotkey.rule_id.clone()));
                    }
                } else {
                    if !hotkey.was_pressed && is_down {
                        hotkey.was_pressed = true;
                        let _ = event_tx.send(HotkeyEvent::Trigger(hotkey.rule_id.clone()));
                    } else if hotkey.was_pressed && !is_down {
                        hotkey.was_pressed = false;
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

pub fn parse_hotkey_vkeys(combo: &str) -> Option<Vec<i32>> {
    let clean = combo.replace(['{', '}'], "");
    let parts: Vec<&str> = clean.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut vkeys = Vec::new();
    for part in parts {
        let upper = part.to_uppercase();
        let vk = match upper.as_str() {
            "CTRL" | "CONTROL" | "LCTRL" | "RCTRL" => 0x11, // VK_CONTROL
            "ALT" | "MENU" | "LALT" | "RALT" => 0x12,       // VK_MENU
            "SHIFT" | "LSHIFT" | "RSHIFT" => 0x10,          // VK_SHIFT
            "WIN" | "WINDOWS" | "LWIN" | "RWIN" => 0x5B,    // VK_LWIN

            // Mouse buttons (full hardware mouse button support!)
            "LBUTTON" | "LMB" | "MOUSE1" | "LEFTCLICK" => 0x01,   // VK_LBUTTON
            "RBUTTON" | "RMB" | "MOUSE2" | "RIGHTCLICK" => 0x02,  // VK_RBUTTON
            "MBUTTON" | "MMB" | "MOUSE3" | "MIDDLECLICK" => 0x04, // VK_MBUTTON
            "M4" | "MOUSE4" | "XBUTTON1" | "MB4" | "SIDE1" => 0x05, // VK_XBUTTON1
            "M5" | "MOUSE5" | "XBUTTON2" | "MB5" | "SIDE2" => 0x06, // VK_XBUTTON2

            // Navigation & keys
            "ENTER" | "RETURN" => 0x0D,
            "ESC" | "ESCAPE" => 0x1B,
            "TAB" => 0x09,
            "SPACE" | "SPACEBAR" => 0x20,
            "BACK" | "BACKSPACE" => 0x08,
            "DEL" | "DELETE" => 0x2E,
            "INS" | "INSERT" => 0x2D,
            "HOME" => 0x24,
            "END" => 0x23,
            "PAGEUP" | "PGUP" | "PRIOR" => 0x21,
            "PAGEDOWN" | "PGDN" | "NEXT" => 0x22,
            "UP" | "ARROWUP" => 0x26,
            "DOWN" | "ARROWDOWN" => 0x28,
            "LEFT" | "ARROWLEFT" => 0x25,
            "RIGHT" | "ARROWRIGHT" => 0x27,
            "CAPS" | "CAPSLOCK" => 0x14,
            "PRTSC" | "PRINTSCREEN" => 0x2C,
            "PAUSE" => 0x13,
            "F1" => 0x70,
            "F2" => 0x71,
            "F3" => 0x72,
            "F4" => 0x73,
            "F5" => 0x74,
            "F6" => 0x75,
            "F7" => 0x76,
            "F8" => 0x77,
            "F9" => 0x78,
            "F10" => 0x79,
            "F11" => 0x7A,
            "F12" => 0x7B,
            s if s.len() == 1 => {
                let ch = s.chars().next().unwrap();
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_uppercase() as i32
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        if !vkeys.contains(&vk) {
            vkeys.push(vk);
        }
    }

    if vkeys.is_empty() {
        None
    } else {
        Some(vkeys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hotkey_vkeys_keyboard_and_mouse() {
        let vkeys_ctrl_m4 = parse_hotkey_vkeys("Ctrl+M4").expect("Valid combo");
        assert_eq!(vkeys_ctrl_m4, vec![0x11, 0x05]);

        let vkeys_m5 = parse_hotkey_vkeys("M5").expect("Valid combo");
        assert_eq!(vkeys_m5, vec![0x06]);

        let vkeys_shift_f12 = parse_hotkey_vkeys("Shift+F12").expect("Valid combo");
        assert_eq!(vkeys_shift_f12, vec![0x10, 0x7B]);

        let vkeys_alt_rmb = parse_hotkey_vkeys("Alt+RMB").expect("Valid combo");
        assert_eq!(vkeys_alt_rmb, vec![0x12, 0x02]);
    }

    #[test]
    fn test_monitored_hotkey_hold_loop_and_is_held() {
        let handle = HotkeyHandle { _private: () };
        let rule = Rule {
            id: "test_loop_rule".to_string(),
            is_enabled: true,
            trigger: Some(Trigger::GlobalHotkey {
                key_combination: "M4".to_string(),
                hold_mode: true,
                repeat_while_held: true,
            }),
            ..Default::default()
        };

        handle.reload(&[rule]);

        assert!(!is_hotkey_held("test_loop_rule"));

        // Simulate press in monitored list
        if let Ok(mut lock) = MONITORED_HOTKEYS.lock() {
            if let Some(hk) = lock.iter_mut().find(|h| h.rule_id == "test_loop_rule") {
                assert!(hk.hold_mode);
                assert!(hk.repeat_while_held);
                hk.was_pressed = true;
            }
        }

        assert!(is_hotkey_held("test_loop_rule"));
    }
}
