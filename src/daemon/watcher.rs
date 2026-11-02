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
#[cfg(windows)]
use windows::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
    CONTROLTRACE_HANDLE, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW,
    EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, PROCESSTRACE_HANDLE,
    PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME, WNODE_FLAG_TRACED_GUID,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEvent {
    pub process_name: String,
    pub pid: u32,
    pub is_started: bool,
}

#[cfg(windows)]
#[repr(C)]
struct EtwPropertiesBuffer {
    props: EVENT_TRACE_PROPERTIES,
    session_name: [u16; 128],
    log_file_name: [u16; 128],
}

#[cfg(windows)]
impl EtwPropertiesBuffer {
    fn new(name: &[u16]) -> Self {
        let mut buf: Self = unsafe { std::mem::zeroed() };
        buf.props.Wnode.BufferSize = std::mem::size_of::<Self>() as u32;
        buf.props.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        buf.props.LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        buf.props.LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
        buf.props.LogFileNameOffset = 0;

        let copy_len = name.len().min(buf.session_name.len() - 1);
        buf.session_name[..copy_len].copy_from_slice(&name[..copy_len]);
        buf
    }
}

#[cfg(windows)]
const KERNEL_PROCESS_GUID: windows::core::GUID = windows::core::GUID::from_values(
    0x22fb2cd6,
    0x0e7b,
    0x422b,
    [0xa0, 0xc7, 0x2f, 0xad, 0x1f, 0xd0, 0xe7, 0x16],
);

#[cfg(windows)]
static ETW_EVENT_QUEUE: std::sync::Mutex<Vec<ProcessEvent>> = std::sync::Mutex::new(Vec::new());

#[cfg(windows)]
unsafe extern "system" fn etw_event_record_callback(record: *mut EVENT_RECORD) {
    if record.is_null() {
        return;
    }
    let rec = &*record;
    if rec.EventHeader.ProviderId == KERNEL_PROCESS_GUID {
        let id = rec.EventHeader.EventDescriptor.Id;
        if (id == 1 || id == 2) && rec.UserDataLength >= 4 && !rec.UserData.is_null() {
            let pid = *(rec.UserData as *const u32);
            if pid != 0 {
                let is_started = id == 1;
                let proc_name = if is_started {
                    ProcessWatcher::get_process_image_path(pid)
                        .and_then(|p| {
                            std::path::Path::new(&p)
                                .file_name()
                                .and_then(|f| f.to_str())
                                .map(|s| s.to_lowercase())
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                if let Ok(mut q) = ETW_EVENT_QUEUE.lock() {
                    if q.len() < 2000 {
                        q.push(ProcessEvent {
                            process_name: proc_name,
                            pid,
                            is_started,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
struct EtwSession {
    session_handle: CONTROLTRACE_HANDLE,
    trace_handle: PROCESSTRACE_HANDLE,
    session_name_wide: Vec<u16>,
}

#[cfg(windows)]
impl EtwSession {
    fn try_start() -> Option<Self> {
        use windows::core::PCWSTR;

        let session_name_str = "SynectorKernelProcessTrace\0";
        let session_name_wide: Vec<u16> = session_name_str.encode_utf16().collect();

        unsafe {
            let mut session_handle = CONTROLTRACE_HANDLE::default();
            let mut buf = EtwPropertiesBuffer::new(&session_name_wide);

            let mut err = StartTraceW(&mut session_handle, PCWSTR(session_name_wide.as_ptr()), &mut buf.props);
            if err.0 == 183 {
                // ERROR_ALREADY_EXISTS: close stale session from earlier run and retry
                let mut stop_buf = EtwPropertiesBuffer::new(&session_name_wide);
                let _ = ControlTraceW(
                    CONTROLTRACE_HANDLE::default(),
                    PCWSTR(session_name_wide.as_ptr()),
                    &mut stop_buf.props,
                    EVENT_TRACE_CONTROL_STOP,
                );
                err = StartTraceW(&mut session_handle, PCWSTR(session_name_wide.as_ptr()), &mut buf.props);
            }

            if err.0 != 0 {
                return None;
            }

            let enable_err = EnableTraceEx2(
                session_handle,
                &KERNEL_PROCESS_GUID,
                1,                  // EVENT_CONTROL_CODE_ENABLE_PROVIDER
                4,                  // TRACE_LEVEL_INFORMATION
                0x8000000000000010, // Process keyword
                0,
                0,
                None,
            );
            if enable_err.0 != 0 {
                let mut stop_buf = EtwPropertiesBuffer::new(&session_name_wide);
                let _ = ControlTraceW(session_handle, PCWSTR(session_name_wide.as_ptr()), &mut stop_buf.props, EVENT_TRACE_CONTROL_STOP);
                return None;
            }

            let mut logfile = EVENT_TRACE_LOGFILEW::default();
            let mut logger_name_wide = session_name_wide.clone();
            logfile.LoggerName = windows::core::PWSTR(logger_name_wide.as_mut_ptr());
            logfile.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
            logfile.Anonymous2.EventRecordCallback = Some(etw_event_record_callback);

            let trace_handle = OpenTraceW(&mut logfile);
            if trace_handle.Value == 0 || trace_handle.Value == u64::MAX {
                let mut stop_buf = EtwPropertiesBuffer::new(&session_name_wide);
                let _ = ControlTraceW(session_handle, PCWSTR(session_name_wide.as_ptr()), &mut stop_buf.props, EVENT_TRACE_CONTROL_STOP);
                return None;
            }

            let th = trace_handle;
            if std::thread::Builder::new()
                .name("etw-kernel-process-trace".into())
                .spawn(move || {
                    let _ = ProcessTrace(&[th], None, None);
                })
                .is_err()
            {
                let _ = CloseTrace(trace_handle);
                let mut stop_buf = EtwPropertiesBuffer::new(&session_name_wide);
                let _ = ControlTraceW(session_handle, PCWSTR(session_name_wide.as_ptr()), &mut stop_buf.props, EVENT_TRACE_CONTROL_STOP);
                return None;
            }

            Some(Self {
                session_handle,
                trace_handle,
                session_name_wide,
            })
        }
    }
}

#[cfg(windows)]
impl Drop for EtwSession {
    fn drop(&mut self) {
        use windows::core::PCWSTR;
        unsafe {
            if self.trace_handle.Value != 0 && self.trace_handle.Value != u64::MAX {
                let _ = CloseTrace(self.trace_handle);
            }
            if self.session_handle.Value != 0 {
                let mut buf = EtwPropertiesBuffer::new(&self.session_name_wide);
                let _ = ControlTraceW(
                    self.session_handle,
                    PCWSTR(self.session_name_wide.as_ptr()),
                    &mut buf.props,
                    EVENT_TRACE_CONTROL_STOP,
                );
            }
        }
    }
}

pub struct ProcessWatcher {
    known_processes: HashMap<u32, String>,
    #[cfg(windows)]
    etw_session: Option<EtwSession>,
    last_reconcile: std::time::Instant,
}

impl ProcessWatcher {
    pub fn new() -> Self {
        let initial = Self::snapshot_process_map();
        #[cfg(windows)]
        let etw = if is_elevated() {
            EtwSession::try_start()
        } else {
            None
        };

        Self {
            known_processes: initial,
            #[cfg(windows)]
            etw_session: etw,
            last_reconcile: std::time::Instant::now(),
        }
    }

    pub fn is_etw_active(&self) -> bool {
        #[cfg(windows)]
        {
            self.etw_session.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    pub fn get_mode_name(&self) -> &'static str {
        if self.is_etw_active() {
            "Kernel ETW (Real-Time Push)"
        } else {
            "Adaptive Snapshot Polling"
        }
    }

    pub fn poll_changes(&mut self) -> Vec<ProcessEvent> {
        #[cfg(windows)]
        if self.etw_session.is_some() {
            let mut events = Vec::new();
            if let Ok(mut q) = ETW_EVENT_QUEUE.lock() {
                if !q.is_empty() {
                    events = std::mem::take(&mut *q);
                }
            }

            let mut out = Vec::with_capacity(events.len());
            for mut ev in events {
                if ev.is_started {
                    if ev.process_name.is_empty() {
                        if let Some(path) = Self::get_process_image_path(ev.pid) {
                            if let Some(name) = std::path::Path::new(&path).file_name().and_then(|f| f.to_str()) {
                                ev.process_name = name.to_lowercase();
                            }
                        }
                    }
                    if !ev.process_name.is_empty() {
                        self.known_processes.insert(ev.pid, ev.process_name.clone());
                        out.push(ev);
                    }
                } else {
                    if let Some(known_name) = self.known_processes.remove(&ev.pid) {
                        ev.process_name = known_name;
                        out.push(ev);
                    }
                }
            }

            // Periodic background reconciliation every 10 seconds to guarantee no drift
            if self.last_reconcile.elapsed().as_secs() >= 10 {
                self.last_reconcile = std::time::Instant::now();
                let current = Self::snapshot_process_map();
                for (&pid, name) in &current {
                    self.known_processes.entry(pid).or_insert_with(|| name.clone());
                }
                self.known_processes.retain(|pid, _| current.contains_key(pid));
            }

            return out;
        }

        // Fallback: Adaptive Toolhelp32 snapshot differential
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

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut ret_len = 0u32;
            let success = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut ret_len,
            ).is_ok();
            let _ = CloseHandle(token);
            if success {
                return elevation.TokenIsElevated != 0;
            }
        }
    }
    false
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etw_imports() {
        #[cfg(windows)]
        {
            use windows::Win32::System::Diagnostics::Etw::*;
            let _ = EVENT_TRACE_CONTROL_STOP;
            let _ = EVENT_TRACE_REAL_TIME_MODE;
            let _ = PROCESS_TRACE_MODE_REAL_TIME;
            let _ = PROCESS_TRACE_MODE_EVENT_RECORD;
            let _ = WNODE_FLAG_TRACED_GUID;

            let logfile = EVENT_TRACE_LOGFILEW::default();
            let _ = logfile.LoggerName;

            assert!(!crate::daemon::watcher::is_elevated());
        }
    }

    #[test]
    fn test_process_watcher_initialization_and_mode() {
        let mut watcher = ProcessWatcher::new();
        assert!(!watcher.known_processes.is_empty());
        let mode = watcher.get_mode_name();
        assert!(mode.contains("ETW") || mode.contains("Snapshot"));
        let _ = watcher.poll_changes();
    }
}
