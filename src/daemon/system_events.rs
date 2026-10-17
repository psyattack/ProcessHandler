use crate::models::SystemEventType;

#[cfg(windows)]
use windows::Win32::System::SystemInformation::GetTickCount;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
#[cfg(windows)]
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

pub struct SystemEventsWatcher {
    last_idle_state: bool,
    idle_threshold_ms: u32,
    last_power_is_ac: Option<bool>,
}

impl SystemEventsWatcher {
    pub fn new() -> Self {
        Self {
            last_idle_state: false,
            idle_threshold_ms: 300_000, // default 5 minutes
            last_power_is_ac: None,
        }
    }
}

impl Default for SystemEventsWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemEventsWatcher {

    pub fn check_events(&mut self) -> Vec<SystemEventType> {
        let mut events = Vec::new();

        // 1. Check Idle / Active
        #[cfg(windows)]
        unsafe {
            let mut lii = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            if GetLastInputInfo(&mut lii).as_bool() {
                let current_tick = GetTickCount();
                let idle_ms = current_tick.saturating_sub(lii.dwTime);

                let is_idle_now = idle_ms >= self.idle_threshold_ms;
                if is_idle_now != self.last_idle_state {
                    self.last_idle_state = is_idle_now;
                    if is_idle_now {
                        events.push(SystemEventType::UserIdle);
                    } else {
                        events.push(SystemEventType::UserActive);
                    }
                }
            }

            // 2. Check AC / Battery power source
            let mut sps: SYSTEM_POWER_STATUS = std::mem::zeroed();
            if GetSystemPowerStatus(&mut sps).is_ok() {
                let is_ac = sps.ACLineStatus == 1;
                match self.last_power_is_ac {
                    Some(prev) if prev != is_ac => {
                        self.last_power_is_ac = Some(is_ac);
                        if is_ac {
                            events.push(SystemEventType::PowerSourceAc);
                        } else {
                            events.push(SystemEventType::PowerSourceBattery);
                        }
                    }
                    None => {
                        self.last_power_is_ac = Some(is_ac);
                    }
                    _ => {}
                }
            }
        }

        events
    }
}
