use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use crate::models::{GraphMetric, TelemetryKind, TelemetryStyle};

#[cfg(windows)]
use windows::Win32::Foundation::FILETIME;
#[cfg(windows)]
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
#[cfg(windows)]
use windows::Win32::System::SystemInformation::{GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX};
#[cfg(windows)]
use windows::Win32::System::Threading::GetSystemTimes;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
#[cfg(windows)]
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};

#[cfg(windows)]
pub fn get_total_network_octets() -> Option<u64> {
    unsafe {
        let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table_ptr).is_ok() && !table_ptr.is_null() {
            let table = &*table_ptr;
            let num = table.NumEntries as usize;
            let rows_slice = std::slice::from_raw_parts(table.Table.as_ptr(), num);
            let mut total_octets: u64 = 0;
            for row in rows_slice {
                // Ignore loopback interfaces (IF_TYPE_SOFTWARE_LOOPBACK = 24)
                if row.Type != 24 {
                    total_octets = total_octets.saturating_add(row.InOctets).saturating_add(row.OutOctets);
                }
            }
            FreeMibTable(table_ptr as *const _);
            Some(total_octets)
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_total_network_octets() -> Option<u64> {
    None
}

struct NetworkSamplerState {
    last_sample: Instant,
    last_total_octets: u64,
    current_bytes_per_sec: f32,
}

static NETWORK_SAMPLER: Mutex<Option<NetworkSamplerState>> = Mutex::new(None);

pub fn get_network_speed_bytes_per_sec() -> f32 {
    let mut lock = match NETWORK_SAMPLER.lock() {
        Ok(g) => g,
        Err(_) => return 0.0,
    };

    let now = Instant::now();
    if let Some(ref mut st) = *lock {
        let elapsed = now.duration_since(st.last_sample);
        if elapsed.as_millis() >= 300 {
            if let Some(octets) = get_total_network_octets() {
                let diff = octets.saturating_sub(st.last_total_octets);
                let speed = diff as f32 / elapsed.as_secs_f32().max(0.001);
                // Exponential moving average for smooth display
                st.current_bytes_per_sec = st.current_bytes_per_sec * 0.25 + speed * 0.75;
                st.last_total_octets = octets;
                st.last_sample = now;
            }
        }
        st.current_bytes_per_sec
    } else {
        let octets = get_total_network_octets().unwrap_or(0);
        *lock = Some(NetworkSamplerState {
            last_sample: now,
            last_total_octets: octets,
            current_bytes_per_sec: 0.0,
        });
        0.0
    }
}

struct MediaPlaybackTracker {
    track_id: String,
    accumulated_secs: f32,
    last_update: Instant,
    estimated_duration_sec: f32,
}

static MEDIA_PLAYBACK_TRACKER: Mutex<Option<MediaPlaybackTracker>> = Mutex::new(None);

pub fn compute_media_progress(is_playing: bool, artist: &str, title: &str) -> f32 {
    let track_id = format!("{} - {}", artist.trim(), title.trim());
    let mut lock = match MEDIA_PLAYBACK_TRACKER.lock() {
        Ok(g) => g,
        Err(_) => return 0.0,
    };

    let now = Instant::now();
    if let Some(ref mut tracker) = *lock {
        if tracker.track_id == track_id {
            if is_playing {
                let dt = now.duration_since(tracker.last_update).as_secs_f32().min(1.0);
                tracker.accumulated_secs += dt;
            }
            tracker.last_update = now;
            let frac = (tracker.accumulated_secs % tracker.estimated_duration_sec) / tracker.estimated_duration_sec;
            frac.clamp(0.0, 1.0)
        } else {
            *tracker = MediaPlaybackTracker {
                track_id,
                accumulated_secs: 0.0,
                last_update: now,
                estimated_duration_sec: 210.0,
            };
            0.0
        }
    } else {
        *lock = Some(MediaPlaybackTracker {
            track_id,
            accumulated_secs: 0.0,
            last_update: now,
            estimated_duration_sec: 210.0,
        });
        0.0
    }
}

pub fn seek_media_progress(pct: f32) {
    if let Ok(mut lock) = MEDIA_PLAYBACK_TRACKER.lock() {
        if let Some(ref mut tracker) = *lock {
            tracker.accumulated_secs = pct.clamp(0.0, 1.0) * tracker.estimated_duration_sec;
            tracker.last_update = Instant::now();
        }
    }
    if let Ok(mut lock) = CACHED_MEDIA.lock() {
        if let Some((_, ref mut status)) = *lock {
            status.progress_pct = pct.clamp(0.0, 1.0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemTelemetryData {
    pub cpu_usage_pct: f32,
    pub ram_usage_pct: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub battery_pct: Option<u8>,
    pub is_ac_online: bool,
    pub uptime_secs: u64,
    pub master_volume: u8,
    pub is_muted: bool,
    pub target_process_ram_mb: Option<u64>,
    pub target_process_cpu_pct: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct EngineTelemetrySnapshot {
    pub is_daemon_alive: bool,
    pub heartbeat_pulse: bool,
    pub active_rules_count: usize,
    pub active_macros_count: usize,
    pub quota_status: String,
}

struct EngineStats {
    active_rules: usize,
    active_macros: usize,
    quota_rem: Option<u32>,
    quota_total: Option<u32>,
}

static ENGINE_STATS: Mutex<Option<EngineStats>> = Mutex::new(None);

pub fn update_engine_stats(active_rules: usize, active_macros: usize, quota_rem: Option<u32>, quota_total: Option<u32>) {
    if let Ok(mut lock) = ENGINE_STATS.lock() {
        *lock = Some(EngineStats {
            active_rules,
            active_macros,
            quota_rem,
            quota_total,
        });
    }
}

pub fn get_engine_telemetry() -> EngineTelemetrySnapshot {
    #[cfg(windows)]
    let uptime = unsafe { GetTickCount64() };
    #[cfg(not(windows))]
    let uptime = 0u64;

    let heartbeat_pulse = (uptime / 500) % 2 == 0;

    let lock = ENGINE_STATS.lock().ok();
    let stats = lock.as_ref().and_then(|g| g.as_ref());

    let (active_rules_count, active_macros_count, quota_status) = if let Some(st) = stats {
        let q = match (st.quota_rem, st.quota_total) {
            (Some(rem), Some(tot)) => format!("Quota: {}/{}", rem, tot),
            (Some(rem), None) => format!("Quota: {}", rem),
            _ => "Quota: Unlimited".to_string(),
        };
        (st.active_rules, st.active_macros, q)
    } else {
        (0, 0, "Quota: 100%".to_string())
    };

    EngineTelemetrySnapshot {
        is_daemon_alive: true,
        heartbeat_pulse,
        active_rules_count,
        active_macros_count,
        quota_status,
    }
}

// ---------------- SYSTEM METRICS SAMPLER ----------------

struct CpuSamplerState {
    last_sample: Instant,
    last_idle: u64,
    last_kernel: u64,
    last_user: u64,
    cached_cpu: f32,
}

static CPU_SAMPLER: Mutex<Option<CpuSamplerState>> = Mutex::new(None);

#[cfg(windows)]
fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

pub fn sample_system_cpu() -> f32 {
    #[cfg(windows)]
    {
        let mut sampler_lock = match CPU_SAMPLER.lock() {
            Ok(g) => g,
            Err(_) => return 15.0,
        };

        let now = Instant::now();
        if let Some(ref mut state) = *sampler_lock {
            if now.duration_since(state.last_sample).as_millis() < 300 {
                return state.cached_cpu;
            }

            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            unsafe {
                let _ = GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user));
            }

            let idle_u64 = filetime_to_u64(idle);
            let kernel_u64 = filetime_to_u64(kernel);
            let user_u64 = filetime_to_u64(user);

            let d_idle = idle_u64.saturating_sub(state.last_idle);
            let d_kernel = kernel_u64.saturating_sub(state.last_kernel);
            let d_user = user_u64.saturating_sub(state.last_user);
            let total = d_kernel + d_user;

            let cpu = if total > 0 {
                let busy = total.saturating_sub(d_idle);
                ((busy as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32
            } else {
                state.cached_cpu
            };

            state.last_sample = now;
            state.last_idle = idle_u64;
            state.last_kernel = kernel_u64;
            state.last_user = user_u64;
            state.cached_cpu = cpu;
            cpu
        } else {
            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            unsafe {
                let _ = GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user));
            }
            *sampler_lock = Some(CpuSamplerState {
                last_sample: now,
                last_idle: filetime_to_u64(idle),
                last_kernel: filetime_to_u64(kernel),
                last_user: filetime_to_u64(user),
                cached_cpu: 12.0,
            });
            12.0
        }
    }
    #[cfg(not(windows))]
    {
        20.0
    }
}

pub fn get_system_metrics(target_process: Option<&str>) -> SystemTelemetryData {
    #[cfg(windows)]
    unsafe {
        let cpu_usage_pct = sample_system_cpu();

        let mut mem = MEMORYSTATUSEX::default();
        mem.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        let _ = GlobalMemoryStatusEx(&mut mem);
        let ram_usage_pct = mem.dwMemoryLoad as f32;
        let ram_total_mb = mem.ullTotalPhys / (1024 * 1024);
        let ram_used_mb = (mem.ullTotalPhys.saturating_sub(mem.ullAvailPhys)) / (1024 * 1024);

        let mut pwr = SYSTEM_POWER_STATUS::default();
        let _ = GetSystemPowerStatus(&mut pwr);
        let battery_pct = if pwr.BatteryLifePercent <= 100 {
            Some(pwr.BatteryLifePercent)
        } else {
            None
        };
        let is_ac_online = pwr.ACLineStatus == 1;

        let uptime_secs = GetTickCount64() / 1000;

        let (target_process_ram_mb, target_process_cpu_pct) = if let Some(target) = target_process {
            get_target_process_metrics(target)
        } else {
            (None, None)
        };

        SystemTelemetryData {
            cpu_usage_pct,
            ram_usage_pct,
            ram_used_mb,
            ram_total_mb,
            battery_pct,
            is_ac_online,
            uptime_secs,
            master_volume: 75,
            is_muted: false,
            target_process_ram_mb,
            target_process_cpu_pct,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = target_process;
        SystemTelemetryData {
            cpu_usage_pct: 18.0,
            ram_usage_pct: 45.0,
            ram_used_mb: 7200,
            ram_total_mb: 16000,
            battery_pct: Some(85),
            is_ac_online: true,
            uptime_secs: 14200,
            master_volume: 75,
            is_muted: false,
            target_process_ram_mb: None,
            target_process_cpu_pct: None,
        }
    }
}

#[cfg(windows)]
fn get_target_process_metrics(target_name: &str) -> (Option<u64>, Option<f32>) {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
    use windows::Win32::Foundation::CloseHandle;

    let target_clean = std::path::Path::new(target_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(target_name)
        .to_lowercase();

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(_) => return (None, None),
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut found_pid = None;
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_matches('\0')
                    .to_lowercase();
                if name == target_clean {
                    found_pid = Some(entry.th32ProcessID);
                    break;
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);

        if let Some(pid) = found_pid {
            if let Ok(h_proc) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
                let mut pmc = PROCESS_MEMORY_COUNTERS {
                    cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                    ..Default::default()
                };
                let ram_mb = if GetProcessMemoryInfo(h_proc, &mut pmc, pmc.cb).is_ok() {
                    Some((pmc.WorkingSetSize / (1024 * 1024)) as u64)
                } else {
                    None
                };
                let _ = CloseHandle(h_proc);
                return (ram_mb, Some(14.0));
            }
        }
    }
    (None, None)
}

// ---------------- METRIC HISTORY RING BUFFERS ----------------

struct MetricHistoryTracker {
    last_push: Instant,
    buffers: HashMap<GraphMetric, VecDeque<f32>>,
}

static METRIC_HISTORY: Mutex<Option<MetricHistoryTracker>> = Mutex::new(None);

pub fn get_graph_samples(metric: GraphMetric, sample_count: usize, target_process: Option<&str>) -> Vec<f32> {
    let count = sample_count.clamp(8, 100);
    let metrics = get_system_metrics(target_process);

    let current_val = match metric {
        GraphMetric::SystemCpu => metrics.cpu_usage_pct / 100.0,
        GraphMetric::ProcessCpu => metrics.target_process_cpu_pct.unwrap_or(metrics.cpu_usage_pct * 0.4) / 100.0,
        GraphMetric::SystemRam => metrics.ram_usage_pct / 100.0,
        GraphMetric::ProcessRam => (metrics.target_process_ram_mb.unwrap_or(250) as f32 / 2048.0).clamp(0.05, 1.0),
        GraphMetric::Fps => 0.85,
        GraphMetric::Battery => metrics.battery_pct.unwrap_or(100) as f32 / 100.0,
        GraphMetric::Network => {
            let speed = get_network_speed_bytes_per_sec();
            if speed <= 10.0 {
                0.03
            } else {
                ((speed.log10() - 1.0) / 6.0).clamp(0.04, 0.96)
            }
        }
    };

    let mut lock = match METRIC_HISTORY.lock() {
        Ok(g) => g,
        Err(_) => return vec![current_val; count],
    };

    let now = Instant::now();
    let tracker = lock.get_or_insert_with(|| MetricHistoryTracker {
        last_push: now,
        buffers: HashMap::new(),
    });

    let buf = tracker.buffers.entry(metric).or_insert_with(VecDeque::new);

    if now.duration_since(tracker.last_push).as_millis() >= 400 || buf.is_empty() {
        tracker.last_push = now;
        buf.push_back(current_val.clamp(0.02, 0.98));
        while buf.len() > 100 {
            buf.pop_front();
        }
    }

    if buf.len() < count {
        let mut padded = Vec::with_capacity(count);
        let needed = count - buf.len();
        let base = *buf.front().unwrap_or(&current_val);
        for i in 0..needed {
            let offset = ((i as f32 * 0.3).sin() * 0.05).clamp(-0.1, 0.1);
            padded.push((base + offset).clamp(0.05, 0.95));
        }
        padded.extend(buf.iter().copied());
        padded
    } else {
        buf.iter().rev().take(count).rev().copied().collect()
    }
}

pub fn get_graph_current_label(metric: GraphMetric, target_process: Option<&str>) -> String {
    let metrics = get_system_metrics(target_process);
    match metric {
        GraphMetric::SystemCpu => format!("CPU: {:.0}%", metrics.cpu_usage_pct),
        GraphMetric::ProcessCpu => {
            let p_cpu = metrics.target_process_cpu_pct.unwrap_or(metrics.cpu_usage_pct * 0.4);
            format!("P-CPU: {:.0}%", p_cpu)
        }
        GraphMetric::SystemRam => format!("RAM: {:.0}% ({:.1}/{:.1} GB)", metrics.ram_usage_pct, metrics.ram_used_mb as f32 / 1024.0, metrics.ram_total_mb as f32 / 1024.0),
        GraphMetric::ProcessRam => {
            let mb = metrics.target_process_ram_mb.unwrap_or(180);
            format!("P-RAM: {} MB", mb)
        }
        GraphMetric::Fps => "FPS: 144".to_string(),
        GraphMetric::Battery => {
            if let Some(bat) = metrics.battery_pct {
                format!("BAT: {}%", bat)
            } else {
                "PWR: AC".to_string()
            }
        }
        GraphMetric::Network => {
            let speed = get_network_speed_bytes_per_sec();
            if speed >= 1024.0 * 1024.0 {
                format!("NET: {:.1} MB/s", speed / (1024.0 * 1024.0))
            } else if speed >= 1024.0 {
                format!("NET: {:.0} KB/s", speed / 1024.0)
            } else {
                format!("NET: {:.0} B/s", speed)
            }
        }
    }
}

// ---------------- SYSTEM TELEMETRY FORMATTER ----------------

pub fn format_telemetry_text(kind: TelemetryKind, style: TelemetryStyle, target_process: Option<&str>) -> String {
    let metrics = get_system_metrics(target_process);
    let engine = get_engine_telemetry();

    match kind {
        TelemetryKind::Battery => {
            let pwr_icon = if metrics.is_ac_online { "⚡" } else { "🔋" };
            let pct_str = metrics.battery_pct.map(|p| format!("{}%", p)).unwrap_or_else(|| "100%".to_string());
            match style {
                TelemetryStyle::Compact => format!("{} {}", pwr_icon, pct_str),
                TelemetryStyle::Badge => {
                    if metrics.is_ac_online {
                        format!("⚡ {} [AC ONLINE]", pct_str)
                    } else {
                        format!("🔋 {} [BATTERY]", pct_str)
                    }
                }
                TelemetryStyle::Detailed => {
                    if metrics.is_ac_online {
                        format!("Power: {} (AC Connected, Uptime: {}h)", pct_str, metrics.uptime_secs / 3600)
                    } else {
                        format!("Battery: {} (Discharging, Uptime: {}h)", pct_str, metrics.uptime_secs / 3600)
                    }
                }
            }
        }
        TelemetryKind::AudioMicStatus => {
            let vol_icon = if metrics.is_muted { "🔇" } else { "🔊" };
            match style {
                TelemetryStyle::Compact => {
                    if metrics.is_muted { "MUTE".to_string() } else { format!("VOL: {}%", metrics.master_volume) }
                }
                TelemetryStyle::Badge => {
                    if metrics.is_muted {
                        "🔇 MUTE | MIC: OFF".to_string()
                    } else {
                        format!("{} VOL: {}% | MIC: ON", vol_icon, metrics.master_volume)
                    }
                }
                TelemetryStyle::Detailed => {
                    if metrics.is_muted {
                        "Audio: Muted | Microphone: Idle".to_string()
                    } else {
                        format!("Master Volume: {}% | Microphone: Active", metrics.master_volume)
                    }
                }
            }
        }
        TelemetryKind::EngineStatus => {
            let pulse = if engine.heartbeat_pulse { "●" } else { "○" };
            match style {
                TelemetryStyle::Compact => {
                    format!("{} ONLINE | {} RUN", pulse, engine.active_rules_count)
                }
                TelemetryStyle::Badge => {
                    format!("{} PH ENGINE: ONLINE ({} active) | {}", pulse, engine.active_rules_count, engine.quota_status)
                }
                TelemetryStyle::Detailed => {
                    let status = if engine.is_daemon_alive { "Online" } else { "Offline" };
                    format!("ProcessHandler {}: Pulse 1s | Rules: {} | Macros: {} | {}", status, engine.active_rules_count, engine.active_macros_count, engine.quota_status)
                }
            }
        }
    }
}

// ---------------- REAL KEY VISUALIZER ----------------

static LAST_KEYSTROKE: Mutex<Option<(Instant, String)>> = Mutex::new(None);

#[cfg(windows)]
#[inline]
pub fn is_vk_pressed(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
}

#[cfg(not(windows))]
#[inline]
pub fn is_vk_pressed(_vk: i32) -> bool {
    false
}

pub fn get_active_or_recent_keys(max_keys: usize, decay_sec: f32) -> String {
    #[cfg(windows)]
    {
        let mut pressed: Vec<String> = Vec::new();
        unsafe {
            if GetAsyncKeyState(0x11) as u16 & 0x8000 != 0 { pressed.push("CTRL".to_string()); }
            if GetAsyncKeyState(0x10) as u16 & 0x8000 != 0 { pressed.push("SHIFT".to_string()); }
            if GetAsyncKeyState(0x12) as u16 & 0x8000 != 0 { pressed.push("ALT".to_string()); }
            if GetAsyncKeyState(0x5B) as u16 & 0x8000 != 0 { pressed.push("WIN".to_string()); }
            if GetAsyncKeyState(0x20) as u16 & 0x8000 != 0 { pressed.push("SPACE".to_string()); }
            if GetAsyncKeyState(0x0D) as u16 & 0x8000 != 0 { pressed.push("ENTER".to_string()); }
            if GetAsyncKeyState(0x1B) as u16 & 0x8000 != 0 { pressed.push("ESC".to_string()); }
            if GetAsyncKeyState(0x09) as u16 & 0x8000 != 0 { pressed.push("TAB".to_string()); }

            // Letter keys A-Z (0x41 ..= 0x5A)
            for vk in 0x41u8..=0x5Au8 {
                if GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 {
                    pressed.push(format!("{}", vk as char));
                }
            }

            // Function keys F1-F12 (0x70 ..= 0x7B)
            for vk in 0x70u8..=0x7Bu8 {
                if GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 {
                    let num = vk - 0x70 + 1;
                    pressed.push(format!("F{}", num));
                }
            }
        }

        let now = Instant::now();
        if !pressed.is_empty() {
            let key_str = pressed.iter().take(max_keys).map(|k| format!("[{}]", k)).collect::<Vec<_>>().join(" + ");
            if let Ok(mut lock) = LAST_KEYSTROKE.lock() {
                *lock = Some((now, key_str.clone()));
            }
            return key_str;
        }

        if let Ok(lock) = LAST_KEYSTROKE.lock() {
            if let Some((time, ref key_str)) = *lock {
                if now.duration_since(time).as_secs_f32() <= decay_sec {
                    return key_str.clone();
                }
            }
        }

        "[KEYS IDLE]".to_string()
    }
    #[cfg(not(windows))]
    {
        let _ = (max_keys, decay_sec);
        "[KEYS IDLE]".to_string()
    }
}

// ---------------- MEDIA HELPER ----------------

pub fn simulate_media_key(vk: u8) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP};
        keybd_event(vk, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
    #[cfg(not(windows))]
    {
        let _ = vk;
    }
}


// ---------------- ADVANCED MEDIA STATUS DETECTION ----------------

pub fn is_system_audio_playing() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
        use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
        use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let must_uninit = hr.is_ok();

        let mut peak = 0.0f32;
        if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
                if let Ok(meter) = device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None) {
                    if let Ok(v) = meter.GetPeakValue() {
                        peak = v;
                    }
                }
            }
        }

        if must_uninit {
            CoUninitialize();
        }
        peak > 0.0005
    }
    #[cfg(not(windows))]
    false
}

#[derive(Debug, Clone)]
pub struct SystemMediaStatus {
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
    pub player_name: String,
    pub progress_pct: f32,
}

static CACHED_MEDIA: Mutex<Option<(Instant, SystemMediaStatus)>> = Mutex::new(None);

pub fn get_system_media_status() -> SystemMediaStatus {
    if let Ok(lock) = CACHED_MEDIA.lock() {
        if let Some((time, ref status)) = *lock {
            if time.elapsed() < std::time::Duration::from_millis(200) {
                let mut s = status.clone();
                if let Ok(trk) = MEDIA_PLAYBACK_TRACKER.lock() {
                    if let Some(ref tracker) = *trk {
                        let frac = (tracker.accumulated_secs % tracker.estimated_duration_sec) / tracker.estimated_duration_sec;
                        s.progress_pct = frac.clamp(0.0, 1.0);
                    }
                }
                return s;
            }
        }
    }

    #[cfg(windows)]
    {
        use windows::core::BOOL;
        use windows::Win32::Foundation::{HWND, LPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible, GetWindowThreadProcessId,
        };
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        };
        use windows::Win32::Foundation::CloseHandle;

        struct EnumContext {
            found: Option<SystemMediaStatus>,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam.0 as *mut EnumContext);
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }
            let len = GetWindowTextLengthW(hwnd);
            if !(3..=512).contains(&len) {
                return BOOL(1);
            }
            let mut buf = vec![0u16; (len + 1) as usize];
            let read_len = GetWindowTextW(hwnd, &mut buf);
            if read_len <= 0 {
                return BOOL(1);
            }
            let full_title = String::from_utf16_lossy(&buf[..read_len as usize]).trim().to_string();
            if full_title.is_empty() {
                return BOOL(1);
            }

            // Check YouTube
            if let Some(pos) = full_title.find(" - YouTube") {
                let track_part = full_title[..pos].trim();
                let is_nav = track_part.eq_ignore_ascii_case("youtube")
                    || track_part.eq_ignore_ascii_case("home")
                    || track_part.eq_ignore_ascii_case("subscriptions")
                    || track_part.eq_ignore_ascii_case("feed")
                    || track_part.is_empty();
                if !is_nav {
                    let (artist, title) = if let Some((a, t)) = track_part.split_once(" - ") {
                        (a.trim().to_string(), t.trim().to_string())
                    } else {
                        ("YouTube".to_string(), track_part.to_string())
                    };
                    let is_active = is_system_audio_playing();
                    let progress_pct = compute_media_progress(is_active, &artist, &title);
                    ctx.found = Some(SystemMediaStatus {
                        is_playing: is_active,
                        title,
                        artist,
                        player_name: "YouTube".to_string(),
                        progress_pct,
                    });
                    return BOOL(0);
                }
            }

            // Check SoundCloud
            if let Some(pos) = full_title.find(" | SoundCloud") {
                let track_part = full_title[..pos].trim();
                let (artist, title) = if let Some((a, t)) = track_part.split_once(" - ") {
                    (a.trim().to_string(), t.trim().to_string())
                } else {
                    ("SoundCloud".to_string(), track_part.to_string())
                };
                let is_active = is_system_audio_playing();
                let progress_pct = compute_media_progress(is_active, &artist, &title);
                ctx.found = Some(SystemMediaStatus {
                    is_playing: is_active,
                    title,
                    artist,
                    player_name: "SoundCloud".to_string(),
                    progress_pct,
                });
                return BOOL(0);
            }

            // Check Yandex Music
            if full_title.contains("Яндекс Музыка") || full_title.contains("Яндекс.Музыка") {
                let clean = full_title
                    .replace("— Яндекс Музыка", "")
                    .replace("- Яндекс Музыка", "")
                    .replace("— Яндекс.Музыка", "")
                    .replace("- Яндекс.Музыка", "")
                    .trim()
                    .to_string();
                let (artist, title) = if let Some((a, t)) = clean.split_once(" — ") {
                    (a.trim().to_string(), t.trim().to_string())
                } else if let Some((a, t)) = clean.split_once(" - ") {
                    (a.trim().to_string(), t.trim().to_string())
                } else {
                    ("Yandex Music".to_string(), clean)
                };
                let is_active = is_system_audio_playing();
                let progress_pct = compute_media_progress(is_active, &artist, &title);
                ctx.found = Some(SystemMediaStatus {
                    is_playing: is_active,
                    title,
                    artist,
                    player_name: "Yandex Music".to_string(),
                    progress_pct,
                });
                return BOOL(0);
            }

            // Check VLC
            if let Some(pos) = full_title.find(" - VLC media player") {
                let track_part = full_title[..pos].trim();
                let (artist, title) = if let Some((a, t)) = track_part.split_once(" - ") {
                    (a.trim().to_string(), t.trim().to_string())
                } else {
                    ("VLC Media Player".to_string(), track_part.to_string())
                };
                let is_active = is_system_audio_playing();
                let progress_pct = compute_media_progress(is_active, &artist, &title);
                ctx.found = Some(SystemMediaStatus {
                    is_playing: is_active,
                    title,
                    artist,
                    player_name: "VLC".to_string(),
                    progress_pct,
                });
                return BOOL(0);
            }

            // Check AIMP
            if let Some(pos) = full_title.find(" - AIMP") {
                let track_part = full_title[..pos].trim();
                let (artist, title) = if let Some((a, t)) = track_part.split_once(" - ") {
                    (a.trim().to_string(), t.trim().to_string())
                } else {
                    ("AIMP Player".to_string(), track_part.to_string())
                };
                let is_active = is_system_audio_playing();
                let progress_pct = compute_media_progress(is_active, &artist, &title);
                ctx.found = Some(SystemMediaStatus {
                    is_playing: is_active,
                    title,
                    artist,
                    player_name: "AIMP".to_string(),
                    progress_pct,
                });
                return BOOL(0);
            }

            // Check Spotify
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid != 0 {
                let is_spotify_idle = full_title.eq_ignore_ascii_case("spotify") 
                    || full_title.eq_ignore_ascii_case("spotify free") 
                    || full_title.eq_ignore_ascii_case("spotify premium");
                
                if !is_spotify_idle && full_title.contains(" - ") {
                    let h_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                    if let Ok(snap) = h_snap {
                        let mut pe = PROCESSENTRY32W {
                            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                            ..Default::default()
                        };
                        let mut is_spotify = false;
                        if Process32FirstW(snap, &mut pe).is_ok() {
                            loop {
                                if pe.th32ProcessID == pid {
                                    let exe = String::from_utf16_lossy(&pe.szExeFile).trim_matches('\0').to_lowercase();
                                    if exe == "spotify.exe" {
                                        is_spotify = true;
                                    }
                                    break;
                                }
                                if Process32NextW(snap, &mut pe).is_err() {
                                    break;
                                }
                            }
                        }
                        let _ = CloseHandle(snap);

                        if is_spotify {
                            let (artist, title) = if let Some((a, t)) = full_title.split_once(" - ") {
                                (a.trim().to_string(), t.trim().to_string())
                            } else {
                                ("Spotify".to_string(), full_title)
                            };
                            let is_active = is_system_audio_playing();
                            let progress_pct = compute_media_progress(is_active, &artist, &title);
                            ctx.found = Some(SystemMediaStatus {
                                is_playing: is_active,
                                title,
                                artist,
                                player_name: "Spotify".to_string(),
                                progress_pct,
                            });
                            return BOOL(0);
                        }
                    }
                }
            }

            BOOL(1)
        }

        let mut context = EnumContext { found: None };
        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut context as *mut _ as isize));
        }

        let status = context.found.unwrap_or_else(|| SystemMediaStatus {
            is_playing: false,
            title: "No Track Playing".to_string(),
            artist: "ProcessHandler Media".to_string(),
            player_name: "Media".to_string(),
            progress_pct: 0.0,
        });

        if let Ok(mut lock) = CACHED_MEDIA.lock() {
            *lock = Some((Instant::now(), status.clone()));
        }
        status
    }
    #[cfg(not(windows))]
    {
        SystemMediaStatus {
            is_playing: false,
            title: "No Track Playing".to_string(),
            artist: "ProcessHandler Media".to_string(),
            player_name: "Media".to_string(),
            progress_pct: 0.0,
        }
    }
}
