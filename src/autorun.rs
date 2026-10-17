pub const APP_NAME: &str = "ProcessHandler";

#[cfg(windows)]
pub fn is_autorun_enabled(app_name: &str) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{
        RegOpenKeyExW, RegQueryValueExW, RegCloseKey,
        HKEY_CURRENT_USER, KEY_READ,
    };

    unsafe {
        let subkey = HSTRING::from("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let ret = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &subkey,
            None,
            KEY_READ,
            &mut hkey,
        );
        if !ret.is_ok() {
            return false;
        }

        let name_w = HSTRING::from(app_name);
        let mut val_type = windows::Win32::System::Registry::REG_VALUE_TYPE::default();
        let mut data_len = 0u32;
        let res = RegQueryValueExW(
            hkey,
            &name_w,
            None,
            Some(&mut val_type),
            None,
            Some(&mut data_len),
        );
        let _ = RegCloseKey(hkey);
        res.is_ok() && data_len > 0
    }
}

#[cfg(windows)]
pub fn set_autorun(app_name: &str, enable: bool) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{
        RegOpenKeyExW, RegSetValueExW, RegDeleteValueW, RegCloseKey,
        HKEY_CURRENT_USER, KEY_WRITE, REG_SZ,
    };

    unsafe {
        let subkey = HSTRING::from("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let ret = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &subkey,
            None,
            KEY_WRITE,
            &mut hkey,
        );
        if !ret.is_ok() {
            return Err(format!("Failed to open registry key: {ret:?}"));
        }

        let name_w = HSTRING::from(app_name);
        let res = if enable {
            let exe_path = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
            let cmd = format!("\"{}\" --daemon", exe_path.to_string_lossy());
            let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
            let slice_bytes = std::slice::from_raw_parts(
                cmd_wide.as_ptr() as *const u8,
                cmd_wide.len() * 2,
            );
            RegSetValueExW(
                hkey,
                &name_w,
                None,
                REG_SZ,
                Some(slice_bytes),
            )
        } else {
            RegDeleteValueW(hkey, &name_w)
        };
        let _ = RegCloseKey(hkey);
        if res.is_ok() {
            Ok(())
        } else {
            Err(format!("Registry write failed: {res:?}"))
        }
    }
}

#[cfg(not(windows))]
pub fn is_autorun_enabled(_app_name: &str) -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_autorun(_app_name: &str, _enable: bool) -> Result<(), String> {
    Ok(())
}
