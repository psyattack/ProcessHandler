#[cfg(windows)]
use windows::core::w;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
#[cfg(windows)]
use windows::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

pub struct SingleInstanceGuard {
    #[cfg(windows)]
    handle: HANDLE,
}

impl SingleInstanceGuard {
    #[cfg(windows)]
    pub fn acquire_named(name: &str) -> Option<Self> {
        use windows::core::HSTRING;
        unsafe {
            let hstring = HSTRING::from(name);
            let handle = CreateMutexW(None, true, windows::core::PCWSTR(hstring.as_ptr())).ok()?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return None;
            }
            Some(Self { handle })
        }
    }

    pub fn acquire_gui() -> Option<Self> {
        #[cfg(windows)]
        return Self::acquire_named("Local\\ProcessHandler_GUI_SingleInstance");
        #[cfg(not(windows))]
        Some(Self {})
    }

    pub fn acquire_daemon() -> Option<Self> {
        #[cfg(windows)]
        return Self::acquire_named("Local\\ProcessHandler_Daemon_SingleInstance");
        #[cfg(not(windows))]
        Some(Self {})
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

/// Finds the existing ProcessHandler GUI window and restores / brings it to the foreground.
/// Returns `true` if the window was found and focused, `false` otherwise.
pub fn focus_existing_gui_window() -> bool {
    #[cfg(windows)]
    {
        for _ in 0..5 {
            unsafe {
                if let Ok(hwnd) = FindWindowW(None, w!("ProcessHandler")) {
                    if !hwnd.0.is_null() {
                        let _ = ShowWindow(hwnd, SW_RESTORE);
                        let _ = SetForegroundWindow(hwnd);
                        return true;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        false
    }
    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_single_instance_named_mutex() {
        let test_name = format!("Local\\ProcessHandler_Test_{}", std::process::id());
        let guard1 = SingleInstanceGuard::acquire_named(&test_name);
        assert!(guard1.is_some(), "First acquire should succeed");

        let guard2 = SingleInstanceGuard::acquire_named(&test_name);
        assert!(guard2.is_none(), "Second acquire should fail while first guard is held");

        drop(guard1);

        let guard3 = SingleInstanceGuard::acquire_named(&test_name);
        assert!(guard3.is_some(), "Acquire should succeed again after previous guard was dropped");
    }
}
