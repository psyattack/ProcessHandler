use std::collections::HashMap;
use crate::models::ProcessSnapshotItem;

#[cfg(windows)]
use windows::core::BOOL;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(windows)]
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

#[derive(Debug, Clone)]
pub struct ProcessEvent {
    pub process_name: String,
    pub pid: u32,
    pub is_started: bool,
}

pub struct ProcessWatcher {
    known_processes: HashMap<u32, String>,
}

impl ProcessWatcher {
    pub fn new() -> Self {
        let initial = Self::snapshot_process_map();
        Self {
            known_processes: initial,
        }
    }

    pub fn poll_changes(&mut self) -> Vec<ProcessEvent> {
        let current = Self::snapshot_process_map();
        let mut events = Vec::new();

        // Check for started processes
        for (&pid, name) in &current {
            if !self.known_processes.contains_key(&pid) {
                events.push(ProcessEvent {
                    process_name: name.clone(),
                    pid,
                    is_started: true,
                });
            }
        }

        // Check for stopped processes
        for (&pid, name) in &self.known_processes {
            if !current.contains_key(&pid) {
                events.push(ProcessEvent {
                    process_name: name.clone(),
                    pid,
                    is_started: false,
                });
            }
        }

        self.known_processes = current;
        events
    }

    #[cfg(windows)]
    pub fn snapshot_process_map() -> HashMap<u32, String> {
        let mut map = HashMap::new();
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot.is_err() {
                return map;
            }
            let handle = snapshot.unwrap();

            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(handle, &mut entry).is_ok() {
                loop {
                    let name_len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]).to_lowercase();
                    map.insert(entry.th32ProcessID, name);

                    if Process32NextW(handle, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(handle);
        }
        map
    }

    #[cfg(not(windows))]
    pub fn snapshot_process_map() -> HashMap<u32, String> {
        HashMap::new()
    }

    #[cfg(windows)]
    pub fn get_process_image_path(pid: u32) -> Option<String> {
        unsafe {
            if let Ok(h_proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                let mut buf = [0u16; 1024];
                let mut size = buf.len() as u32;
                let res = QueryFullProcessImageNameW(
                    h_proc,
                    PROCESS_NAME_FORMAT(0),
                    windows::core::PWSTR(buf.as_mut_ptr()),
                    &mut size,
                );
                let _ = CloseHandle(h_proc);
                if res.is_ok() {
                    return Some(String::from_utf16_lossy(&buf[..size as usize]));
                }
            }
            None
        }
    }

    #[cfg(not(windows))]
    pub fn get_process_image_path(_pid: u32) -> Option<String> {
        None
    }

    pub fn find_exe_path_by_name(process_name: &str) -> Option<String> {
        let lower = process_name.to_lowercase();
        let map = Self::snapshot_process_map();
        for (&pid, name) in &map {
            if name.to_lowercase() == lower {
                if let Some(path) = Self::get_process_image_path(pid) {
                    return Some(path);
                }
            }
        }
        let sys_root = std::env::var_os("SystemRoot")
            .or_else(|| std::env::var_os("WINDIR"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows"));
        let sys32_candidate = sys_root.join("System32").join(process_name);
        if sys32_candidate.exists() {
            return Some(sys32_candidate.to_string_lossy().to_string());
        }
        None
    }

    #[cfg(windows)]
    pub fn get_detailed_snapshot() -> Vec<ProcessSnapshotItem> {
        let mut items = Vec::new();
        let window_titles = Self::get_window_titles();

        unsafe {
            let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                Ok(h) => h,
                Err(_) => return items,
            };

            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let pid = entry.th32ProcessID;
                    if pid != 0 {
                        let name_len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                        let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);

                        let mut mem_bytes = 0u64;
                        if let Ok(h_proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                            let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                            pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                            if K32GetProcessMemoryInfo(h_proc, &mut pmc, pmc.cb).as_bool() {
                                mem_bytes = pmc.WorkingSetSize as u64;
                            }
                            let _ = CloseHandle(h_proc);
                        }

                        let window_title = window_titles.get(&pid).cloned().unwrap_or_default();

                        items.push(ProcessSnapshotItem {
                            pid,
                            name,
                            window_title,
                            memory_bytes: mem_bytes,
                            cpu_percent: 0.0,
                        });
                    }

                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }

        // Sort by memory descending
        items.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));
        items
    }

    #[cfg(not(windows))]
    pub fn get_detailed_snapshot() -> Vec<ProcessSnapshotItem> {
        Vec::new()
    }

    #[cfg(windows)]
    fn get_window_titles() -> HashMap<u32, String> {
        let mut map: HashMap<u32, String> = HashMap::new();
        unsafe {
            unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let map = &mut *(lparam.0 as *mut HashMap<u32, String>);
                if IsWindowVisible(hwnd).as_bool() {
                    let mut pid = 0u32;
                    GetWindowThreadProcessId(hwnd, Some(&mut pid));
                    if pid != 0 && !map.contains_key(&pid) {
                        let mut buf = [0u16; 256];
                        let len = GetWindowTextW(hwnd, &mut buf);
                        if len > 0 {
                            let title = String::from_utf16_lossy(&buf[..len as usize]);
                            if !title.trim().is_empty() {
                                map.insert(pid, title);
                            }
                        }
                    }
                }
                BOOL(1)
            }

            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut map as *mut _ as isize));
        }
        map
    }
}
