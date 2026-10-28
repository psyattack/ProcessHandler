#[cfg(windows)]
#[allow(clippy::manual_dangling_ptr)]
pub fn strip_pe_icon(exe_path: &std::path::Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{BOOL, PCWSTR};
    use windows::Win32::Foundation::{FreeLibrary, HMODULE};
    use windows::Win32::System::LibraryLoader::{
        BeginUpdateResourceW, EndUpdateResourceW, EnumResourceLanguagesW, EnumResourceNamesW,
        LoadLibraryExW, UpdateResourceW, LOAD_LIBRARY_AS_DATAFILE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON, RT_VERSION};

    let wide_path: Vec<u16> = exe_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    // 1. Enumerate all existing icon and group icon resources with their exact language IDs
    let mut targets: Vec<(PCWSTR, usize, u16)> = Vec::new();

    unsafe {
        if let Ok(hmod) = LoadLibraryExW(PCWSTR(wide_path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) {
            unsafe extern "system" fn collect_langs(
                _hmod: HMODULE,
                lpsztype: PCWSTR,
                lpszname: PCWSTR,
                w_id_language: u16,
                lparam: isize,
            ) -> BOOL {
                let list = &mut *(lparam as *mut Vec<(PCWSTR, usize, u16)>);
                list.push((lpsztype, lpszname.0 as usize, w_id_language));
                BOOL(1)
            }

            unsafe extern "system" fn collect_names(
                hmod: HMODULE,
                lpsztype: PCWSTR,
                lpszname: PCWSTR,
                lparam: isize,
            ) -> BOOL {
                let _ = EnumResourceLanguagesW(Some(hmod), lpsztype, lpszname, Some(collect_langs), lparam);
                BOOL(1)
            }

            let _ = EnumResourceNamesW(
                Some(hmod),
                RT_GROUP_ICON,
                Some(collect_names),
                &mut targets as *mut _ as isize,
            );
            let _ = EnumResourceNamesW(
                Some(hmod),
                RT_ICON,
                Some(collect_names),
                &mut targets as *mut _ as isize,
            );
            let _ = EnumResourceNamesW(
                Some(hmod),
                RT_VERSION,
                Some(collect_names),
                &mut targets as *mut _ as isize,
            );
            let _ = FreeLibrary(hmod);
        }

        if targets.is_empty() {
            return false;
        }

        // 2. Open for resource updating and remove each discovered icon resource
        let h_update = match BeginUpdateResourceW(PCWSTR(wide_path.as_ptr()), false) {
            Ok(h) => h,
            Err(_) => return false,
        };
        if h_update.is_invalid() {
            return false;
        }

        for (res_type, name_id, lang_id) in targets {
            let _ = UpdateResourceW(
                h_update,
                res_type,
                PCWSTR(name_id as *const u16),
                lang_id,
                None,
                0,
            );
        }

        EndUpdateResourceW(h_update, false).is_ok()
    }
}

#[cfg(not(windows))]
pub fn strip_pe_icon(_exe_path: &std::path::Path) -> bool {
    false
}

pub fn get_masked_daemon_exe() -> std::path::PathBuf {
    let bin_dir = crate::config::AppConfig::config_dir().join("bin");
    let _ = std::fs::create_dir_all(&bin_dir);
    bin_dir.join("WindowsServiceHost.exe")
}

pub fn cleanup_masked_daemon() {
    let bin_dir = crate::config::AppConfig::config_dir().join("bin");
    let masked_exe = bin_dir.join("WindowsServiceHost.exe");
    if masked_exe.exists() {
        let _ = std::fs::remove_file(&masked_exe);
    }
    let _ = std::fs::remove_dir(&bin_dir);
}

pub fn prepare_masked_daemon() -> Result<std::path::PathBuf, String> {
    let current_exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
    let masked_exe = get_masked_daemon_exe();

    // Check if target needs to be copied or refreshed
    let needs_copy = match (std::fs::metadata(&current_exe), std::fs::metadata(&masked_exe)) {
        (Ok(cur_meta), Ok(mask_meta)) => {
            let cur_time = cur_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let mask_time = mask_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            cur_meta.len() != mask_meta.len() || cur_time > mask_time
        }
        _ => true,
    };

    if needs_copy {
        let mut copied = false;
        let mut last_err = String::new();
        for _ in 0..10 {
            if masked_exe.exists() {
                let _ = std::fs::remove_file(&masked_exe);
            }
            match std::fs::copy(&current_exe, &masked_exe) {
                Ok(_) => {
                    copied = true;
                    break;
                }
                Err(e) => {
                    last_err = e.to_string();
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
        if !copied {
            return Err(format!("Failed to copy masked executable: {last_err}"));
        }
        #[cfg(windows)]
        {
            strip_pe_icon(&masked_exe);
        }
    }

    Ok(masked_exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_pe_icon_roundtrip() {
        if let Ok(cur) = std::env::current_exe() {
            let tmp = std::env::temp_dir().join(format!("ph_mask_test_{}.exe", std::process::id()));
            if std::fs::copy(&cur, &tmp).is_ok() {
                let res = strip_pe_icon(&tmp);
                println!("strip_pe_icon on temp exe: {}", res);
                assert!(res);
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }
}
