use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::time::Instant;
use crate::models::{MacroEvent, MouseButton, MouseCoordMode};

#[cfg(windows)]
use windows::core::w;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, HINSTANCE, HMODULE, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorInfo,
    GetMessageW, KillTimer, PeekMessageW, PostThreadMessageW, RegisterClassExW,
    SetTimer, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, UnregisterClassW, CURSORINFO, HHOOK,
    KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WINDOW_EX_STYLE, WINDOW_STYLE,
    WNDCLASSEXW, WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_TIMER, WM_XBUTTONDOWN, WM_XBUTTONUP, PM_NOREMOVE, MESSAGEBOX_STYLE,
};
#[cfg(windows)]
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
    RID_INPUT, RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEMOUSE,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_ESCAPE, VK_F9, VK_F10,
};
#[cfg(windows)]
use windows::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows::Win32::System::Diagnostics::Debug::MessageBeep;

#[derive(Clone)]
pub struct MacroRecorder {
    pub is_recording: Arc<AtomicBool>,
    pub capture_keyboard: Arc<AtomicBool>,
    pub capture_mouse_clicks: Arc<AtomicBool>,
    pub capture_mouse_moves: Arc<AtomicBool>,
    pub capture_mouse_wheel: Arc<AtomicBool>,
    pub capture_delays: Arc<AtomicBool>,
    pub min_move_dist: Arc<AtomicU32>,
    pub last_was_mouse_move: Arc<AtomicBool>,
    pub events: Arc<Mutex<Vec<MacroEvent>>>,
    pub last_event_time: Arc<Mutex<Option<Instant>>>,
    pub record_start_time: Arc<Mutex<Option<Instant>>>,
    pub last_mouse_pos: Arc<Mutex<Option<(i32, i32, Instant)>>>,
    pub last_key_events: Arc<Mutex<std::collections::HashMap<String, (bool, Instant)>>>,
}

static RECORDER_INSTANCE: OnceLock<MacroRecorder> = OnceLock::new();

#[cfg(windows)]
static KBD_HOOK: AtomicIsize = AtomicIsize::new(0);
#[cfg(windows)]
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
#[cfg(windows)]
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

struct RawTurnAccumulator {
    accum_dx: i32,
    accum_dy: i32,
    start_time: Instant,
    last_update: Instant,
}

static RAW_TURN_ACCUMULATOR: Mutex<Option<RawTurnAccumulator>> = Mutex::new(None);

static LAST_RECORD_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_TEST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);

impl MacroRecorder {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            capture_keyboard: Arc::new(AtomicBool::new(true)),
            capture_mouse_clicks: Arc::new(AtomicBool::new(true)),
            capture_mouse_moves: Arc::new(AtomicBool::new(true)),
            capture_mouse_wheel: Arc::new(AtomicBool::new(true)),
            capture_delays: Arc::new(AtomicBool::new(true)),
            min_move_dist: Arc::new(AtomicU32::new(4)),
            last_was_mouse_move: Arc::new(AtomicBool::new(false)),
            events: Arc::new(Mutex::new(Vec::new())),
            last_event_time: Arc::new(Mutex::new(None)),
            record_start_time: Arc::new(Mutex::new(None)),
            last_mouse_pos: Arc::new(Mutex::new(None)),
            last_key_events: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn global() -> Self {
        RECORDER_INSTANCE.get_or_init(Self::new).clone()
    }

    pub fn is_active(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn start_recording(&self, capture_mouse_moves: bool) {
        if self.is_recording.swap(true, Ordering::SeqCst) {
            return; // Already recording
        }

        self.capture_mouse_moves.store(capture_mouse_moves, Ordering::SeqCst);
        self.last_was_mouse_move.store(false, Ordering::SeqCst);
        if let Ok(mut ev) = self.events.lock() {
            ev.clear();
        }
        let now = Instant::now();
        if let Ok(mut t) = self.last_event_time.lock() {
            *t = Some(now);
        }
        if let Ok(mut st) = self.record_start_time.lock() {
            *st = Some(now);
        }
        if let Ok(mut mp) = self.last_mouse_pos.lock() {
            *mp = None;
        }
        if let Ok(mut k) = self.last_key_events.lock() {
            k.clear();
        }
        if let Ok(mut lock) = LAST_RECORD_TOGGLE.lock() {
            *lock = Some(now);
        }

        if let Ok(mut lock) = RAW_TURN_ACCUMULATOR.lock() {
            *lock = None;
        }

        #[cfg(windows)]
        {
            let rec = self.clone();
            std::thread::Builder::new()
                .name("macro-recorder-hook".into())
                .spawn(move || {
                    run_recorder_hook_loop(rec);
                })
                .expect("Failed to spawn macro recorder hook thread");

            let rec_poller = self.clone();
            std::thread::Builder::new()
                .name("macro-recorder-poller".into())
                .spawn(move || {
                    run_recorder_polling_loop(rec_poller);
                })
                .expect("Failed to spawn macro recorder polling thread");
        }
    }

    pub fn start(&self) {
        self.start_recording(self.capture_mouse_moves.load(Ordering::SeqCst));
    }

    pub fn stop_recording(&self) {
        self.is_recording.store(false, Ordering::SeqCst);
        if let Ok(mut lock) = LAST_RECORD_TOGGLE.lock() {
            *lock = Some(Instant::now());
        }

        flush_raw_turn_accumulator(self);

        #[cfg(windows)]
        {
            let kh = KBD_HOOK.swap(0, Ordering::SeqCst);
            if kh != 0 {
                unsafe {
                    let _ = UnhookWindowsHookEx(HHOOK(kh as *mut _));
                }
            }
            let mh = MOUSE_HOOK.swap(0, Ordering::SeqCst);
            if mh != 0 {
                unsafe {
                    let _ = UnhookWindowsHookEx(HHOOK(mh as *mut _));
                }
            }

            let tid = HOOK_THREAD_ID.swap(0, Ordering::SeqCst);
            if tid != 0 {
                unsafe {
                    let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
                }
            }
        }
    }

    pub fn drain_events(&self) -> Vec<MacroEvent> {
        if let Ok(mut ev) = self.events.lock() {
            std::mem::take(&mut *ev)
        } else {
            Vec::new()
        }
    }

    /// Record key event safely with per-key deduplication window across hook, background poller, and GUI input
    pub fn record_key(&self, key_name: &str, is_down: bool) {
        let clean = key_name.trim();
        if clean.is_empty() {
            return;
        }
        let now = Instant::now();
        if let Ok(mut last_map) = self.last_key_events.lock() {
            if let Some((last_down, last_t)) = last_map.get(clean) {
                if *last_down == is_down && now.duration_since(*last_t).as_millis() < 70 {
                    return; // Deduplicate across hook, polling loop, and GUI event loop
                }
            }
            last_map.insert(clean.to_string(), (is_down, now));
        }

        flush_raw_turn_accumulator(self);

        record_delay_and_event(
            self,
            if is_down {
                MacroEvent::KeyDown { key: clean.to_string() }
            } else {
                MacroEvent::KeyUp { key: clean.to_string() }
            },
            true, // insert delay before key
        );
    }
}

pub fn flush_raw_turn_accumulator(rec: &MacroRecorder) {
    let acc = if let Ok(mut lock) = RAW_TURN_ACCUMULATOR.lock() {
        lock.take()
    } else {
        None
    };

    if let Some(acc) = acc {
        let abs_dx = acc.accum_dx.abs();
        let abs_dy = acc.accum_dy.abs();
        // Ignore resting hand micro-noise (< 3 counts)
        if abs_dx < 3 && abs_dy < 3 {
            return;
        }

        // Exact physical duration from first hardware packet to last hardware packet
        let stroke_ms = acc.last_update.duration_since(acc.start_time).as_millis() as u32;
        let move_duration = stroke_ms.max(16);

        // Calculate pause before this turn started relative to last recorded event
        let mut pause_delay = None;
        if rec.capture_delays.load(Ordering::SeqCst) {
            if let Ok(mut time_lock) = rec.last_event_time.lock() {
                if let Some(last_ev_t) = *time_lock {
                    if acc.start_time > last_ev_t {
                        let pause = acc.start_time.duration_since(last_ev_t).as_millis() as u32;
                        if pause >= 12 {
                            pause_delay = Some(pause);
                        }
                    }
                }
                *time_lock = Some(acc.last_update);
            }
        } else if let Ok(mut time_lock) = rec.last_event_time.lock() {
            *time_lock = Some(acc.last_update);
        }

        rec.last_was_mouse_move.store(true, Ordering::SeqCst);

        if let Ok(mut events) = rec.events.lock() {
            if let Some(ms) = pause_delay {
                events.push(MacroEvent::Delay { ms, organic: false, jitter_percent: 30 });
            }
            events.push(MacroEvent::MouseMove {
                x: acc.accum_dx,
                y: acc.accum_dy,
                coord_mode: MouseCoordMode::Relative,
                smooth: true,
                duration_ms: Some(move_duration),
                organic: false,
            });
        }
    }
}

#[cfg(windows)]
fn check_raw_turn_idle_flush(rec: &MacroRecorder) {
    let mut should_flush = false;
    if let Ok(lock) = RAW_TURN_ACCUMULATOR.lock() {
        if let Some(acc) = lock.as_ref() {
            if acc.last_update.elapsed().as_millis() >= 60 {
                should_flush = true;
            }
        }
    }
    if should_flush {
        flush_raw_turn_accumulator(rec);
    }
}

#[cfg(windows)]
unsafe fn process_raw_input_event(hraw: HRAWINPUT, rec: &MacroRecorder) {
    if !rec.is_recording.load(Ordering::SeqCst) || !rec.capture_mouse_moves.load(Ordering::SeqCst) {
        return;
    }

    let mut size = 0u32;
    let res = GetRawInputData(
        hraw,
        RID_INPUT,
        None,
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );
    if res == u32::MAX || size == 0 {
        return;
    }

    let mut buffer = vec![0u8; size as usize];
    let res2 = GetRawInputData(
        hraw,
        RID_INPUT,
        Some(buffer.as_mut_ptr() as *mut _),
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );
    if res2 == u32::MAX {
        return;
    }

    let raw = &*(buffer.as_ptr() as *const RAWINPUT);
    if raw.header.dwType != RIM_TYPEMOUSE.0 {
        return;
    }

    let mouse = &raw.data.mouse;
    // Relative displacement only (ignore absolute pen/tablet coordinates)
    if (mouse.usFlags.0 & 1) != 0 {
        return;
    }

    let dx = mouse.lLastX;
    let dy = mouse.lLastY;
    if dx == 0 && dy == 0 {
        return;
    }

    // Arbitration with desktop mouse moves:
    // If desktop cursor movements (Absolute) are also enabled, check if cursor is visibly moving on screen.
    if rec.capture_mouse_moves.load(Ordering::SeqCst) {
        let mut ci = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };
        let is_cursor_showing = GetCursorInfo(&mut ci).is_ok() && (ci.flags.0 & 1) != 0;
        if is_cursor_showing {
            if let Ok(pos_lock) = rec.last_mouse_pos.lock() {
                if let Some((_, _, last_t)) = *pos_lock {
                    if last_t.elapsed().as_millis() < 60 {
                        // Desktop mouse movement is actively being captured by WH_MOUSE_LL
                        return;
                    }
                }
            }
        }
    }

    let now = Instant::now();
    let mut should_flush = false;

    if let Ok(mut lock) = RAW_TURN_ACCUMULATOR.lock() {
        if let Some(acc) = lock.as_mut() {
            let x_flipped = (acc.accum_dx > 60 && dx < -30) || (acc.accum_dx < -60 && dx > 30);
            let y_flipped = (acc.accum_dy > 60 && dy < -30) || (acc.accum_dy < -60 && dy > 30);
            let duration = now.duration_since(acc.start_time).as_millis();

            // Only flush on true direction reversal or extreme duration (1.5s), NOT at 120ms!
            if duration >= 1500 || x_flipped || y_flipped {
                should_flush = true;
            } else {
                acc.accum_dx += dx;
                acc.accum_dy += dy;
                acc.last_update = now;
            }
        } else {
            *lock = Some(RawTurnAccumulator {
                accum_dx: dx,
                accum_dy: dy,
                start_time: now,
                last_update: now,
            });
        }
    }

    if should_flush {
        flush_raw_turn_accumulator(rec);
        if let Ok(mut lock) = RAW_TURN_ACCUMULATOR.lock() {
            *lock = Some(RawTurnAccumulator {
                accum_dx: dx,
                accum_dy: dy,
                start_time: now,
                last_update: now,
            });
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn raw_input_sink_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_INPUT => {
            let rec = MacroRecorder::global();
            process_raw_input_event(HRAWINPUT(lparam.0 as *mut _), &rec);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_TIMER => {
            let rec = MacroRecorder::global();
            check_raw_turn_idle_flush(&rec);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(windows)]
fn run_recorder_hook_loop(recorder: MacroRecorder) {
    unsafe {
        let tid = GetCurrentThreadId();
        HOOK_THREAD_ID.store(tid, Ordering::SeqCst);

        // Ensure thread has a Win32 message queue created
        let mut dummy_msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        let _ = PeekMessageW(&mut dummy_msg, None, 0, 0, PM_NOREMOVE);

        let hinstance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .unwrap_or(HMODULE(std::ptr::null_mut()));

        let class_name = w!("ProcessHandler_RawInput_Sink");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(raw_input_sink_wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: class_name,
            ..Default::default()
        };
        let _ = RegisterClassExW(&wc);

        let sink_hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("ProcessHandlerRawInputSink"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(HINSTANCE(hinstance.0)),
            None,
        ).unwrap_or(HWND(std::ptr::null_mut()));

        if !sink_hwnd.0.is_null() {
            let rid = RAWINPUTDEVICE {
                usUsagePage: 0x01, // Generic Desktop Controls
                usUsage: 0x02,     // Mouse
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: sink_hwnd,
            };
            let _ = RegisterRawInputDevices(&[rid], std::mem::size_of::<RAWINPUTDEVICE>() as u32);
            let _ = SetTimer(Some(sink_hwnd), 1, 20, None);
        }

        let kbd_hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            Some(HINSTANCE(hinstance.0)),
            0,
        );
        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(low_level_mouse_proc),
            Some(HINSTANCE(hinstance.0)),
            0,
        );
        if let Ok(h) = kbd_hook {
            KBD_HOOK.store(h.0 as isize, Ordering::SeqCst);
        }
        if let Ok(h) = mouse_hook {
            MOUSE_HOOK.store(h.0 as isize, Ordering::SeqCst);
        }

        let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        while recorder.is_recording.load(Ordering::SeqCst)
            && GetMessageW(&mut msg, None, 0, 0).as_bool()
        {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if !sink_hwnd.0.is_null() {
            let unreg_rid = RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x02,
                dwFlags: RIDEV_REMOVE,
                hwndTarget: HWND(std::ptr::null_mut()),
            };
            let _ = RegisterRawInputDevices(&[unreg_rid], std::mem::size_of::<RAWINPUTDEVICE>() as u32);
            let _ = KillTimer(Some(sink_hwnd), 1);
            let _ = DestroyWindow(sink_hwnd);
            let _ = UnregisterClassW(class_name, Some(HINSTANCE(hinstance.0)));
        }

        let kh = KBD_HOOK.swap(0, Ordering::SeqCst);
        if kh != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(kh as *mut _));
        }
        let mh = MOUSE_HOOK.swap(0, Ordering::SeqCst);
        if mh != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(mh as *mut _));
        }
        HOOK_THREAD_ID.store(0, Ordering::SeqCst);
    }
}

/// Dedicated high-precision physical polling loop for games, DirectInput, exclusive fullscreen, and elevated windows
/// where low-level hooks may be bypassed or suppressed.
#[cfg(windows)]
fn run_recorder_polling_loop(recorder: MacroRecorder) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE};
    use windows::Win32::System::Diagnostics::Debug::MessageBeep;
    use windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE;

    // Full set of physical virtual keyboard keys to poll
    const POLLABLE_KEYS: &[u32] = &[
        0x08, 0x09, 0x0C, 0x0D, // Backspace, Tab, Clear, Enter
        0x10, 0x11, 0x12, 0x13, 0x14, // Shift, Ctrl, Alt, Pause, CapsLock
        0x20, 0x21, 0x22, 0x23, 0x24, // Space, PageUp, PageDown, End, Home
        0x25, 0x26, 0x27, 0x28, // Left, Up, Right, Down
        0x2C, 0x2D, 0x2E, // PrintScreen, Insert, Delete
        // '0'..='9'
        0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
        // 'A'..='Z'
        0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A,
        0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54,
        0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,
        0x5B, 0x5C, // Win keys
        // NumPad keys
        0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
        0x6A, 0x6B, 0x6D, 0x6E, 0x6F,
        // Function keys (F1..F8, F11..F24)
        0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77,
        0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
        0x90, 0x91, // NumLock, ScrollLock
        // OEM punctuation keys
        0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xBF, 0xC0, 0xDB, 0xDC, 0xDD, 0xDE, 0xE2,
    ];

    let mut key_states = [false; 256];

    // Seed initial hardware key state to prevent phantom down events
    for &vk in POLLABLE_KEYS {
        let is_down = unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };
        key_states[vk as usize] = is_down;
    }

    while recorder.is_recording.load(Ordering::SeqCst) {
        let elapsed_start = recorder.record_start_time.lock().ok()
            .and_then(|t| *t)
            .map(|st| st.elapsed().as_millis())
            .unwrap_or(1000);

        // Escape cancellation check (fallback if hook was bypassed)
        if elapsed_start > 250 {
            let is_esc = unsafe { (GetAsyncKeyState(VK_ESCAPE.0 as i32) as u16 & 0x8000) != 0 };
            if is_esc {
                recorder.stop_recording();
                unsafe {
                    let _ = MessageBeep(MESSAGEBOX_STYLE(0));
                }
                break;
            }
        }

        if recorder.capture_keyboard.load(Ordering::SeqCst) {
            for &vk in POLLABLE_KEYS {
                let is_down = unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };
                let was_down = key_states[vk as usize];

                if is_down != was_down {
                    key_states[vk as usize] = is_down;
                    let key_name = vk_to_key_name(vk);
                    recorder.record_key(&key_name, is_down);
                }
            }
        }

        check_raw_turn_idle_flush(&recorder);
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

#[cfg(windows)]
unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = kbd.vkCode;
        let is_down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
        let is_up = wparam.0 as u32 == WM_KEYUP || wparam.0 as u32 == WM_SYSKEYUP;

        let rec = MacroRecorder::global();

        let elapsed_start = rec.record_start_time.lock().ok()
            .and_then(|t| *t)
            .map(|st| st.elapsed().as_millis())
            .unwrap_or(1000);

        // F9 controls start / stop recording - always swallow and never record into timeline!
        if vk == VK_F9.0 as u32 {
            if elapsed_start > 250 && is_down {
                rec.stop_recording();
                let _ = MessageBeep(windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE(0));
            }
            return LRESULT(1);
        }

        // F10 controls testing - always swallow and never record into timeline!
        if vk == VK_F10.0 as u32 {
            return LRESULT(1);
        }

        // Escape also stops recording if active (ignore during first 250ms)
        if vk == VK_ESCAPE.0 as u32 && rec.is_recording.load(Ordering::SeqCst) && elapsed_start > 250 {
            if is_down {
                rec.stop_recording();
                let _ = MessageBeep(windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE(0));
            }
            return LRESULT(1);
        }

        if rec.is_recording.load(Ordering::SeqCst)
            && rec.capture_keyboard.load(Ordering::SeqCst)
            && (is_down || is_up)
        {
            let key_name = vk_to_key_name(vk);
            rec.record_key(&key_name, is_down);
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let ms = *(lparam.0 as *const MSLLHOOKSTRUCT);
        let msg = wparam.0 as u32;
        let rec = MacroRecorder::global();

        if rec.is_recording.load(Ordering::SeqCst) {
            let elapsed_since_start = rec.record_start_time.lock().ok()
                .and_then(|t| *t)
                .map(|st| st.elapsed().as_millis())
                .unwrap_or(1000);

            let clicks_enabled = rec.capture_mouse_clicks.load(Ordering::SeqCst);
            let wheel_enabled = rec.capture_mouse_wheel.load(Ordering::SeqCst);

            // Filter out initial click from pressing the record button (first 200ms)
            if elapsed_since_start >= 200 {
                let is_mouse_btn = matches!(
                    msg,
                    WM_LBUTTONDOWN
                        | WM_LBUTTONUP
                        | WM_RBUTTONDOWN
                        | WM_RBUTTONUP
                        | WM_MBUTTONDOWN
                        | WM_MBUTTONUP
                        | WM_XBUTTONDOWN
                        | WM_XBUTTONUP
                );

                if clicks_enabled && is_mouse_btn {
                    flush_raw_turn_accumulator(&rec);
                    // Sync cursor position right before click event if moves are enabled
                    if rec.capture_mouse_moves.load(Ordering::SeqCst) {
                        let mut needs_pos_sync = false;
                        if let Ok(mut lock) = rec.last_mouse_pos.lock() {
                            if let Some((lx, ly, _)) = *lock {
                                if lx != ms.pt.x || ly != ms.pt.y {
                                    *lock = Some((ms.pt.x, ms.pt.y, Instant::now()));
                                    needs_pos_sync = true;
                                }
                            } else {
                                *lock = Some((ms.pt.x, ms.pt.y, Instant::now()));
                                needs_pos_sync = true;
                            }
                        }
                        if needs_pos_sync {
                            record_delay_and_event(
                                &rec,
                                MacroEvent::MouseMove {
                                    x: ms.pt.x,
                                    y: ms.pt.y,
                                    coord_mode: MouseCoordMode::Absolute,
                                    smooth: false,
                                    duration_ms: Some(16),
                                    organic: false,
                                },
                                true, // Preserve delay before click sync so pauses before clicking are recorded
                            );
                        }
                    }

                    match msg {
                        WM_LBUTTONDOWN => {
                            record_delay_and_event(&rec, MacroEvent::MouseDown { button: MouseButton::Left }, true);
                        }
                        WM_LBUTTONUP => {
                            record_delay_and_event(&rec, MacroEvent::MouseUp { button: MouseButton::Left }, true);
                        }
                        WM_RBUTTONDOWN => {
                            record_delay_and_event(&rec, MacroEvent::MouseDown { button: MouseButton::Right }, true);
                        }
                        WM_RBUTTONUP => {
                            record_delay_and_event(&rec, MacroEvent::MouseUp { button: MouseButton::Right }, true);
                        }
                        WM_MBUTTONDOWN => {
                            record_delay_and_event(&rec, MacroEvent::MouseDown { button: MouseButton::Middle }, true);
                        }
                        WM_MBUTTONUP => {
                            record_delay_and_event(&rec, MacroEvent::MouseUp { button: MouseButton::Middle }, true);
                        }
                        WM_XBUTTONDOWN => {
                            let hi = ((ms.mouseData >> 16) & 0xffff) as u16;
                            let btn = if hi == 1 { MouseButton::XButton1 } else { MouseButton::XButton2 };
                            record_delay_and_event(&rec, MacroEvent::MouseDown { button: btn }, true);
                        }
                        WM_XBUTTONUP => {
                            let hi = ((ms.mouseData >> 16) & 0xffff) as u16;
                            let btn = if hi == 1 { MouseButton::XButton1 } else { MouseButton::XButton2 };
                            record_delay_and_event(&rec, MacroEvent::MouseUp { button: btn }, true);
                        }
                        _ => {}
                    }
                }

                if wheel_enabled && msg == WM_MOUSEWHEEL {
                    flush_raw_turn_accumulator(&rec);
                    let delta = (((ms.mouseData >> 16) & 0xffff) as i16) as i32;
                    record_delay_and_event(&rec, MacroEvent::MouseWheel { delta }, true);
                }
            }

            // Direct, accurate mouse movement capture (smooth human sampling with physical timing)
            if msg == WM_MOUSEMOVE && rec.capture_mouse_moves.load(Ordering::SeqCst) {
                // In 3D games with hidden cursor, skip screen move capture in favor of Raw Input camera turns
                let mut ci = CURSORINFO {
                    cbSize: std::mem::size_of::<CURSORINFO>() as u32,
                    ..Default::default()
                };
                let is_cursor_hidden = GetCursorInfo(&mut ci).is_ok() && (ci.flags.0 & 1) == 0;
                if is_cursor_hidden {
                    return CallNextHookEx(None, code, wparam, lparam);
                }

                let min_dist = rec.min_move_dist.load(Ordering::SeqCst).clamp(2, 6) as f32;
                let mut should_record = false;
                let mut move_duration = None;
                let mut is_resumed_after_pause = false;

                if let Ok(mut lock) = rec.last_mouse_pos.lock() {
                    let now = Instant::now();
                    if let Some((lx, ly, lt)) = *lock {
                        let dx = (ms.pt.x - lx) as f32;
                        let dy = (ms.pt.y - ly) as f32;
                        let dist = dx.hypot(dy);
                        let elapsed = now.duration_since(lt).as_millis() as u32;

                        if elapsed >= 80 {
                            // After a pause (>= 80ms), record any actual movement (>= 1.0 px) as the start of a new stroke
                            if dist >= 1.0 {
                                *lock = Some((ms.pt.x, ms.pt.y, now));
                                should_record = true;
                                move_duration = Some(16);
                                is_resumed_after_pause = true;
                            }
                        } else if (dist >= min_dist && elapsed >= 10) || (elapsed >= 25 && dist >= 2.0) {
                            // Continuous movement (< 80ms): sample smoothly without cluttering
                            *lock = Some((ms.pt.x, ms.pt.y, now));
                            should_record = true;
                            move_duration = Some(elapsed.clamp(8, 60));
                        }
                    } else {
                        *lock = Some((ms.pt.x, ms.pt.y, now));
                        should_record = true;
                        move_duration = Some(16);
                        is_resumed_after_pause = true;
                    }
                }

                if should_record {
                    let is_prev_move = rec.last_was_mouse_move.load(Ordering::SeqCst);
                    // Allow delay if this is the start of mouse movement OR if movement resumed after an idle pause
                    let allow_delay = !is_prev_move || is_resumed_after_pause;

                    record_delay_and_event(
                        &rec,
                        MacroEvent::MouseMove {
                            x: ms.pt.x,
                            y: ms.pt.y,
                            coord_mode: MouseCoordMode::Absolute,
                            smooth: false,
                            duration_ms: move_duration,
                            organic: false,
                        },
                        allow_delay,
                    );
                }
            }
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

fn record_delay_and_event(rec: &MacroRecorder, event: MacroEvent, allow_delay: bool) {
    let now = Instant::now();
    let mut delay_to_insert = None;

    let is_move = matches!(event, MacroEvent::MouseMove { .. });
    rec.last_was_mouse_move.store(is_move, Ordering::SeqCst);

    if allow_delay && rec.capture_delays.load(Ordering::SeqCst) {
        if let Ok(mut time_lock) = rec.last_event_time.lock() {
            if let Some(last) = *time_lock {
                let elapsed = now.duration_since(last).as_millis() as u32;
                if elapsed >= 12 {
                    delay_to_insert = Some(elapsed);
                }
            }
            *time_lock = Some(now);
        }
    } else if let Ok(mut time_lock) = rec.last_event_time.lock() {
        *time_lock = Some(now);
    }

    if let Ok(mut events) = rec.events.lock() {
        if let Some(ms) = delay_to_insert {
            events.push(MacroEvent::Delay { ms, organic: false, jitter_percent: 30 });
        }
        events.push(event);
    }
}

/// Coalesces contiguous MouseMove sequences into clean, single smooth A -> B trajectory steps.
#[cfg(test)]
pub fn coalesce_mouse_moves(events: &mut Vec<MacroEvent>) {
    if events.len() < 2 {
        return;
    }

    let mut result: Vec<MacroEvent> = Vec::with_capacity(events.len());
    let mut i = 0;

    while i < events.len() {
        if let MacroEvent::MouseMove { x: start_x, y: start_y, coord_mode, organic, .. } = events[i] {
            let is_rel = coord_mode == MouseCoordMode::Relative;
            let mut final_x = start_x;
            let mut final_y = start_y;
            let final_cm = coord_mode;
            let final_org = organic;
            let mut moves_count = 1;
            let mut total_duration = match events[i] {
                MacroEvent::MouseMove { duration_ms, .. } => duration_ms.unwrap_or(25),
                _ => 25,
            };
            let mut j = i + 1;

            while j < events.len() {
                match &events[j] {
                    MacroEvent::MouseMove { x, y, coord_mode: cm, duration_ms, .. } if *cm == final_cm => {
                        if is_rel {
                            final_x += *x;
                            final_y += *y;
                        } else {
                            final_x = *x;
                            final_y = *y;
                        }
                        total_duration += duration_ms.unwrap_or(25);
                        moves_count += 1;
                        j += 1;
                    }
                    MacroEvent::Delay { ms, .. } if *ms <= 50 && j + 1 < events.len() && matches!(events[j + 1], MacroEvent::MouseMove { coord_mode: cm, .. } if cm == final_cm) => {
                        total_duration += *ms;
                        j += 1;
                    }
                    _ => break,
                }
            }

            if moves_count > 1 {
                let duration_estimate = total_duration.clamp(40, 3000);
                result.push(MacroEvent::MouseMove {
                    x: final_x,
                    y: final_y,
                    coord_mode: final_cm,
                    smooth: true,
                    duration_ms: Some(duration_estimate),
                    organic: final_org,
                });
                i = j;
            } else {
                result.push(events[i].clone());
                i += 1;
            }
        } else {
            result.push(events[i].clone());
            i += 1;
        }
    }

    *events = result;
}

pub fn vk_to_key_name(vk: u32) -> String {
    match vk {
        0x08 => "Backspace".to_string(),
        0x09 => "Tab".to_string(),
        0x0C => "Clear".to_string(),
        0x0D => "Enter".to_string(),
        0x10 | 0xA0 | 0xA1 => "Shift".to_string(),
        0x11 | 0xA2 | 0xA3 => "Ctrl".to_string(),
        0x12 | 0xA4 | 0xA5 => "Alt".to_string(),
        0x13 => "Pause".to_string(),
        0x14 => "CapsLock".to_string(),
        0x1B => "Esc".to_string(),
        0x20 => "Space".to_string(),
        0x21 => "PageUp".to_string(),
        0x22 => "PageDown".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x2C => "PrintScreen".to_string(),
        0x2D => "Insert".to_string(),
        0x2E => "Delete".to_string(),
        0x30..=0x39 => format!("{}", (vk - 0x30)),
        0x41..=0x5A => format!("{}", ((vk as u8) as char)),
        0x5B | 0x5C => "Win".to_string(),
        0x60..=0x69 => format!("NumPad{}", vk - 0x60),
        0x6A => "NumPadMultiply".to_string(),
        0x6B => "NumPadAdd".to_string(),
        0x6D => "NumPadSubtract".to_string(),
        0x6E => "NumPadDecimal".to_string(),
        0x6F => "NumPadDivide".to_string(),
        0x70..=0x87 => format!("F{}", vk - 0x70 + 1),
        0x90 => "NumLock".to_string(),
        0x91 => "ScrollLock".to_string(),
        0xBA => ";".to_string(),
        0xBB => "=".to_string(),
        0xBC => ",".to_string(),
        0xBD => "-".to_string(),
        0xBE => ".".to_string(),
        0xBF => "/".to_string(),
        0xC0 => "`".to_string(),
        0xDB => "[".to_string(),
        0xDC => "\\".to_string(),
        0xDD => "]".to_string(),
        0xDE => "'".to_string(),
        _ => format!("VK_0x{:02X}", vk),
    }
}

/// Convert egui Key to string representation for internal GUI key capture
pub fn egui_key_to_string(key: egui::Key) -> &'static str {
    match key {
        egui::Key::A => "A",
        egui::Key::B => "B",
        egui::Key::C => "C",
        egui::Key::D => "D",
        egui::Key::E => "E",
        egui::Key::F => "F",
        egui::Key::G => "G",
        egui::Key::H => "H",
        egui::Key::I => "I",
        egui::Key::J => "J",
        egui::Key::K => "K",
        egui::Key::L => "L",
        egui::Key::M => "M",
        egui::Key::N => "N",
        egui::Key::O => "O",
        egui::Key::P => "P",
        egui::Key::Q => "Q",
        egui::Key::R => "R",
        egui::Key::S => "S",
        egui::Key::T => "T",
        egui::Key::U => "U",
        egui::Key::V => "V",
        egui::Key::W => "W",
        egui::Key::X => "X",
        egui::Key::Y => "Y",
        egui::Key::Z => "Z",
        egui::Key::Num0 => "0",
        egui::Key::Num1 => "1",
        egui::Key::Num2 => "2",
        egui::Key::Num3 => "3",
        egui::Key::Num4 => "4",
        egui::Key::Num5 => "5",
        egui::Key::Num6 => "6",
        egui::Key::Num7 => "7",
        egui::Key::Num8 => "8",
        egui::Key::Num9 => "9",
        egui::Key::F1 => "F1",
        egui::Key::F2 => "F2",
        egui::Key::F3 => "F3",
        egui::Key::F4 => "F4",
        egui::Key::F5 => "F5",
        egui::Key::F6 => "F6",
        egui::Key::F7 => "F7",
        egui::Key::F8 => "F8",
        egui::Key::F9 => "F9",
        egui::Key::F10 => "F10",
        egui::Key::F11 => "F11",
        egui::Key::F12 => "F12",
        egui::Key::Enter => "Enter",
        egui::Key::Tab => "Tab",
        egui::Key::Space => "Space",
        egui::Key::Backspace => "Backspace",
        egui::Key::Delete => "Delete",
        egui::Key::Insert => "Insert",
        egui::Key::Home => "Home",
        egui::Key::End => "End",
        egui::Key::PageUp => "PageUp",
        egui::Key::PageDown => "PageDown",
        egui::Key::ArrowUp => "Up",
        egui::Key::ArrowDown => "Down",
        egui::Key::ArrowLeft => "Left",
        egui::Key::ArrowRight => "Right",
        egui::Key::Escape => "Esc",
        egui::Key::ShiftLeft | egui::Key::ShiftRight => "Shift",
        egui::Key::ControlLeft | egui::Key::ControlRight => "Ctrl",
        egui::Key::AltLeft | egui::Key::AltRight => "Alt",
        egui::Key::SuperLeft | egui::Key::SuperRight => "Win",
        egui::Key::Colon => ":",
        egui::Key::Comma => ",",
        egui::Key::Backslash => "\\",
        egui::Key::Slash => "/",
        egui::Key::Pipe => "|",
        egui::Key::Questionmark => "?",
        egui::Key::Exclamationmark => "!",
        egui::Key::OpenBracket => "[",
        egui::Key::CloseBracket => "]",
        egui::Key::OpenCurlyBracket => "{",
        egui::Key::CloseCurlyBracket => "}",
        egui::Key::Backtick => "`",
        egui::Key::Minus => "-",
        egui::Key::Period => ".",
        egui::Key::Plus => "+",
        egui::Key::Equals => "=",
        egui::Key::Semicolon => ";",
        egui::Key::Quote => "'",
        _ => "",
    }
}

static TEST_RUNNING: AtomicBool = AtomicBool::new(false);
static TEST_CANCEL: AtomicBool = AtomicBool::new(false);

pub fn is_macro_test_running() -> bool {
    TEST_RUNNING.load(Ordering::SeqCst)
}

pub fn stop_macro_test() {
    TEST_CANCEL.store(true, Ordering::SeqCst);
    if let Ok(mut lock) = LAST_TEST_TOGGLE.lock() {
        *lock = Some(Instant::now());
    }
    crate::daemon::actions::release_all_simulated_inputs();
}

/// Safely execute macro events on a dedicated worker thread with current-thread tokio runtime.
/// Never panics on GUI thread and never freezes the interface.
pub fn run_macro_test_safe(events: Vec<MacroEvent>, speed: f32) {
    if events.is_empty() {
        return;
    }
    if TEST_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Ok(mut lock) = LAST_TEST_TOGGLE.lock() {
        *lock = Some(Instant::now());
    }
    TEST_CANCEL.store(false, Ordering::SeqCst);

    std::thread::Builder::new()
        .name("macro-test-runner".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_time().build() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create tokio runtime for macro test runner: {e}");
                    TEST_RUNNING.store(false, Ordering::SeqCst);
                    return;
                }
            };
            rt.block_on(async move {
                let start_time = std::time::Instant::now();
                let dummy_ctx = std::collections::HashMap::new();
                for ev in &events {
                    if TEST_CANCEL.load(Ordering::SeqCst) {
                        break;
                    }

                    #[cfg(windows)]
                    unsafe {
                        if start_time.elapsed().as_millis() > 300
                            && windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                                windows::Win32::UI::Input::KeyboardAndMouse::VK_F10.0 as i32,
                            ) < 0
                        {
                            TEST_CANCEL.store(true, Ordering::SeqCst);
                            break;
                        }
                    }

                    match ev {
                        MacroEvent::Delay { ms, organic, jitter_percent } => {
                            let eff_ms = (*ms as f32 / speed.max(0.01)).round() as u32;
                            let sleep_ms = crate::daemon::actions::calculate_sleep_ms(eff_ms, *organic, *jitter_percent, None);
                            let sleep_target = std::time::Instant::now() + std::time::Duration::from_millis(sleep_ms);
                            while std::time::Instant::now() < sleep_target {
                                if TEST_CANCEL.load(Ordering::SeqCst) {
                                    break;
                                }
                                #[cfg(windows)]
                                unsafe {
                                    if start_time.elapsed().as_millis() > 300
                                        && windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                                            windows::Win32::UI::Input::KeyboardAndMouse::VK_F10.0 as i32,
                                        ) < 0
                                    {
                                        TEST_CANCEL.store(true, Ordering::SeqCst);
                                        break;
                                    }
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                            }
                        }
                        _ => {
                            let _ = crate::daemon::actions::execute_macro_event(ev, speed, &dummy_ctx).await;
                        }
                    }

                    if TEST_CANCEL.load(Ordering::SeqCst) {
                        break;
                    }
                }
            });
            crate::daemon::actions::release_all_simulated_inputs();
            TEST_RUNNING.store(false, Ordering::SeqCst);
            if let Ok(mut lock) = LAST_TEST_TOGGLE.lock() {
                *lock = Some(std::time::Instant::now());
            }
        })
        .expect("Failed to spawn macro test runner thread");
}

static STUDIO_ACTIVE: AtomicBool = AtomicBool::new(false);
static STUDIO_MACRO_PAYLOAD: Mutex<Option<(Vec<MacroEvent>, f32)>> = Mutex::new(None);
static STUDIO_DRAINED_BUFFER: Mutex<Vec<MacroEvent>> = Mutex::new(Vec::new());
static STUDIO_HOTKEY_THREAD_STARTED: AtomicBool = AtomicBool::new(false);

pub fn set_studio_active(active: bool) {
    STUDIO_ACTIVE.store(active, Ordering::SeqCst);
    if active {
        ensure_studio_hotkey_thread_started();
    }
}

pub fn update_studio_payload(events: Vec<MacroEvent>, speed: f32) {
    if let Ok(mut lock) = STUDIO_MACRO_PAYLOAD.lock() {
        *lock = Some((events, speed));
    }
}

pub fn take_studio_drained_events() -> Option<Vec<MacroEvent>> {
    if let Ok(mut lock) = STUDIO_DRAINED_BUFFER.lock() {
        if lock.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut *lock))
        }
    } else {
        None
    }
}

fn ensure_studio_hotkey_thread_started() {
    #[cfg(windows)]
    if !STUDIO_HOTKEY_THREAD_STARTED.swap(true, Ordering::SeqCst) {
        std::thread::Builder::new()
            .name("macro-studio-global-hotkeys".into())
            .spawn(|| {
                run_studio_hotkey_loop();
            })
            .expect("Failed to spawn studio hotkey listener thread");
    }
}

#[cfg(windows)]
fn run_studio_hotkey_loop() {
    let mut was_f9 = false;
    let mut was_f10 = false;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(20));

        if !STUDIO_ACTIVE.load(Ordering::SeqCst) {
            was_f9 = false;
            was_f10 = false;
            continue;
        }

        let is_f9 = unsafe { GetAsyncKeyState(VK_F9.0 as i32) } < 0;
        let is_f10 = unsafe { GetAsyncKeyState(VK_F10.0 as i32) } < 0;

        let now = Instant::now();

        if is_f9 && !was_f9 {
            let cooldown_ok = if let Ok(last) = LAST_RECORD_TOGGLE.lock() {
                last.map(|t| now.duration_since(t).as_millis() > 350).unwrap_or(true)
            } else {
                true
            };

            if cooldown_ok {
                let rec = MacroRecorder::global();
                if rec.is_active() {
                    rec.stop_recording();
                    let drained = rec.drain_events();
                    if !drained.is_empty() {
                        if let Ok(mut buf) = STUDIO_DRAINED_BUFFER.lock() {
                            buf.extend(drained);
                        }
                    }
                    unsafe {
                        let _ = MessageBeep(MESSAGEBOX_STYLE(0));
                    }
                } else if !is_macro_test_running() {
                    rec.start();
                    unsafe {
                        let _ = MessageBeep(MESSAGEBOX_STYLE(0));
                    }
                }
            }
        }
        was_f9 = is_f9;

        if is_f10 && !was_f10 {
            let cooldown_ok = if let Ok(last) = LAST_TEST_TOGGLE.lock() {
                last.map(|t| now.duration_since(t).as_millis() > 350).unwrap_or(true)
            } else {
                true
            };

            if cooldown_ok {
                let rec = MacroRecorder::global();
                if !rec.is_active() {
                    if is_macro_test_running() {
                        stop_macro_test();
                        unsafe {
                            let _ = MessageBeep(MESSAGEBOX_STYLE(0));
                        }
                    } else {
                        let pending = rec.drain_events();
                        if !pending.is_empty() {
                            if let Ok(mut buf) = STUDIO_DRAINED_BUFFER.lock() {
                                buf.extend(pending.clone());
                            }
                        }

                        if let Ok(mut lock) = STUDIO_MACRO_PAYLOAD.lock() {
                            if let Some((events, speed)) = lock.as_mut() {
                                if !pending.is_empty() {
                                    events.extend(pending);
                                }
                                if !events.is_empty() {
                                    unsafe {
                                        let _ = MessageBeep(MESSAGEBOX_STYLE(0));
                                    }
                                    run_macro_test_safe(events.clone(), *speed);
                                }
                            }
                        }
                    }
                }
            }
        }
        was_f10 = is_f10;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coalesce_mouse_moves() {
        let mut events = vec![
            MacroEvent::MouseMove { x: 10, y: 10, coord_mode: MouseCoordMode::Absolute, smooth: false, duration_ms: None, organic: false },
            MacroEvent::MouseMove { x: 50, y: 50, coord_mode: MouseCoordMode::Absolute, smooth: false, duration_ms: None, organic: false },
            MacroEvent::MouseMove { x: 100, y: 200, coord_mode: MouseCoordMode::Absolute, smooth: false, duration_ms: None, organic: false },
            MacroEvent::Delay { ms: 500, organic: false, jitter_percent: 30 },
            MacroEvent::MouseDown { button: MouseButton::Left },
        ];

        coalesce_mouse_moves(&mut events);
        assert_eq!(events.len(), 3);
        match &events[0] {
            MacroEvent::MouseMove { x, y, smooth, duration_ms, .. } => {
                assert_eq!(*x, 100);
                assert_eq!(*y, 200);
                assert!(*smooth);
                assert!(duration_ms.is_some());
            }
            _ => panic!("Expected coalesced MouseMove"),
        }
        assert!(matches!(events[1], MacroEvent::Delay { ms: 500, .. }));
        assert!(matches!(events[2], MacroEvent::MouseDown { .. }));
    }

    #[test]
    fn test_coalesce_relative_mouse_moves() {
        let mut events = vec![
            MacroEvent::MouseMove { x: 50, y: 10, coord_mode: MouseCoordMode::Relative, smooth: true, duration_ms: Some(50), organic: false },
            MacroEvent::MouseMove { x: 30, y: -5, coord_mode: MouseCoordMode::Relative, smooth: true, duration_ms: Some(40), organic: false },
            MacroEvent::Delay { ms: 300, organic: false, jitter_percent: 30 },
            MacroEvent::KeyDown { key: "Space".to_string() },
        ];

        coalesce_mouse_moves(&mut events);
        assert_eq!(events.len(), 3);
        match &events[0] {
            MacroEvent::MouseMove { x, y, coord_mode, smooth, duration_ms, .. } => {
                assert_eq!(*x, 80);
                assert_eq!(*y, 5);
                assert_eq!(*coord_mode, MouseCoordMode::Relative);
                assert!(*smooth);
                assert_eq!(duration_ms.unwrap(), 90);
            }
            _ => panic!("Expected coalesced relative MouseMove"),
        }
        assert!(matches!(events[1], MacroEvent::Delay { ms: 300, .. }));
        assert!(matches!(events[2], MacroEvent::KeyDown { .. }));
    }

    #[test]
    fn test_record_key_deduplication() {
        let rec = MacroRecorder::new();
        rec.is_recording.store(true, Ordering::SeqCst);

        // Immediate duplicate "A" down should be deduplicated
        rec.record_key("A", true);
        rec.record_key("A", true);
        // Different key "B" down should be recorded
        rec.record_key("B", true);
        // Releasing "A" should be recorded
        rec.record_key("A", false);
        // Immediate duplicate "A" up should be deduplicated
        rec.record_key("A", false);

        let events = rec.drain_events();
        let keys: Vec<_> = events.into_iter().filter_map(|e| match e {
            MacroEvent::KeyDown { key } => Some((key, true)),
            MacroEvent::KeyUp { key } => Some((key, false)),
            _ => None,
        }).collect();

        assert_eq!(keys, vec![
            ("A".to_string(), true),
            ("B".to_string(), true),
            ("A".to_string(), false),
        ]);
    }



    #[test]
    #[cfg(windows)]
    fn test_hooks_creation() {
        unsafe {
            let hmod = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .ok()
                .map(|h| windows::Win32::Foundation::HINSTANCE(h.0));
            println!("GetModuleHandleW result: {:?}", hmod);

            let kbd_hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                hmod,
                0,
            );
            println!("kbd_hook result: {:?}", kbd_hook);
            assert!(kbd_hook.is_ok(), "SetWindowsHookExW for WH_KEYBOARD_LL failed: {:?}", kbd_hook.err());

            let mouse_hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(low_level_mouse_proc),
                hmod,
                0,
            );
            println!("mouse_hook result: {:?}", mouse_hook);
            assert!(mouse_hook.is_ok(), "SetWindowsHookExW for WH_MOUSE_LL failed: {:?}", mouse_hook.err());

            if let Ok(h) = kbd_hook {
                let _ = UnhookWindowsHookEx(h);
            }
            if let Ok(h) = mouse_hook {
                let _ = UnhookWindowsHookEx(h);
            }
        }
    }

    #[test]
    fn test_record_key_retains_events_after_stop() {
        let rec = MacroRecorder::new();
        rec.is_recording.store(true, Ordering::SeqCst);

        rec.record_key("A", true);
        rec.record_key("A", false);

        rec.stop_recording();

        let events = rec.drain_events();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], MacroEvent::KeyDown { key } if key == "A"));
        assert!(matches!(&events[1], MacroEvent::KeyUp { key } if key == "A"));

        // After drain, it should be empty
        assert_eq!(rec.drain_events().len(), 0);
    }

    #[test]
    fn test_dnd_reordering_math() {
        // Test move down: from index 0 to target 2
        let mut items = vec!["A", "B", "C", "D"];
        let src = 0;
        let dst = 2;
        if dst > src + 1 {
            let item = items.remove(src);
            items.insert(dst - 1, item);
        }
        assert_eq!(items, vec!["B", "A", "C", "D"]);

        // Test move up: from index 3 to target 1
        let mut items2 = vec!["A", "B", "C", "D"];
        let src2 = 3;
        let dst2 = 1;
        if dst2 < src2 {
            let item = items2.remove(src2);
            items2.insert(dst2, item);
        }
        assert_eq!(items2, vec!["A", "D", "B", "C"]);
    }

    #[test]
    fn test_egui_key_to_string_modifiers() {
        assert_eq!(egui_key_to_string(egui::Key::ShiftLeft), "Shift");
        assert_eq!(egui_key_to_string(egui::Key::ShiftRight), "Shift");
        assert_eq!(egui_key_to_string(egui::Key::ControlLeft), "Ctrl");
        assert_eq!(egui_key_to_string(egui::Key::ControlRight), "Ctrl");
        assert_eq!(egui_key_to_string(egui::Key::AltLeft), "Alt");
        assert_eq!(egui_key_to_string(egui::Key::AltRight), "Alt");
        assert_eq!(egui_key_to_string(egui::Key::SuperLeft), "Win");
        assert_eq!(egui_key_to_string(egui::Key::SuperRight), "Win");
        assert_eq!(egui_key_to_string(egui::Key::Enter), "Enter");
    }

    #[test]
    fn test_studio_payload_and_drained_events() {
        set_studio_active(true);
        assert!(STUDIO_ACTIVE.load(Ordering::SeqCst));

        let test_ev = vec![MacroEvent::KeyPress { key: "Space".into() }];
        update_studio_payload(test_ev.clone(), 1.5);
        if let Ok(lock) = STUDIO_MACRO_PAYLOAD.lock() {
            let payload = lock.as_ref().unwrap();
            assert_eq!(payload.0.len(), 1);
            assert_eq!(payload.1, 1.5);
        }

        if let Ok(mut buf) = STUDIO_DRAINED_BUFFER.lock() {
            buf.push(MacroEvent::KeyPress { key: "Enter".into() });
        }
        let drained = take_studio_drained_events();
        assert_eq!(drained.map(|v| v.len()), Some(1));
        assert!(take_studio_drained_events().is_none());

        set_studio_active(false);
        assert!(!STUDIO_ACTIVE.load(Ordering::SeqCst));
    }

    #[test]
    fn test_mouse_pause_delay_recording() {
        use std::time::Duration;

        let rec = MacroRecorder::new();
        rec.is_recording.store(true, Ordering::SeqCst);
        rec.capture_delays.store(true, Ordering::SeqCst);
        rec.capture_mouse_moves.store(true, Ordering::SeqCst);

        // 1. First move
        record_delay_and_event(
            &rec,
            MacroEvent::MouseMove {
                x: 10,
                y: 10,
                coord_mode: MouseCoordMode::Absolute,
                smooth: false,
                duration_ms: Some(16),
                organic: false,
            },
            true,
        );

        // 2. Continuous move (< 80ms, allow_delay = false)
        record_delay_and_event(
            &rec,
            MacroEvent::MouseMove {
                x: 20,
                y: 20,
                coord_mode: MouseCoordMode::Absolute,
                smooth: false,
                duration_ms: Some(15),
                organic: false,
            },
            false,
        );

        let events_step1 = rec.events.lock().unwrap().clone();
        // There should be no delays between continuous moves
        assert_eq!(events_step1.len(), 2);
        assert!(matches!(events_step1[0], MacroEvent::MouseMove { x: 10, y: 10, .. }));
        assert!(matches!(events_step1[1], MacroEvent::MouseMove { x: 20, y: 20, .. }));

        // 3. User stops mouse for 5 seconds, then starts moving again
        if let Ok(mut t) = rec.last_event_time.lock() {
            *t = Some(Instant::now() - Duration::from_millis(5000));
        }

        // Resumed move after pause (allow_delay = true)
        record_delay_and_event(
            &rec,
            MacroEvent::MouseMove {
                x: 50,
                y: 50,
                coord_mode: MouseCoordMode::Absolute,
                smooth: false,
                duration_ms: Some(16),
                organic: false,
            },
            true,
        );

        let events_step2 = rec.events.lock().unwrap().clone();
        assert_eq!(events_step2.len(), 4);
        assert!(matches!(events_step2[2], MacroEvent::Delay { ms, .. } if ms >= 4900));
        assert!(matches!(events_step2[3], MacroEvent::MouseMove { x: 50, y: 50, .. }));

        // 4. User stops for 3 seconds, then clicks (with 1px micro-shift sync)
        if let Ok(mut t) = rec.last_event_time.lock() {
            *t = Some(Instant::now() - Duration::from_millis(3000));
        }

        // Pos sync before click (allow_delay = true)
        record_delay_and_event(
            &rec,
            MacroEvent::MouseMove {
                x: 51,
                y: 50,
                coord_mode: MouseCoordMode::Absolute,
                smooth: false,
                duration_ms: Some(16),
                organic: false,
            },
            true,
        );
        // MouseDown immediately follows
        record_delay_and_event(
            &rec,
            MacroEvent::MouseDown { button: MouseButton::Left },
            true,
        );

        let events_step3 = rec.events.lock().unwrap().clone();
        assert_eq!(events_step3.len(), 7);
        // Delay of ~3000ms before pos sync
        assert!(matches!(events_step3[4], MacroEvent::Delay { ms, .. } if ms >= 2900));
        assert!(matches!(events_step3[5], MacroEvent::MouseMove { x: 51, y: 50, .. }));
        // No redundant delay before MouseDown (0ms elapsed)
        assert!(matches!(events_step3[6], MacroEvent::MouseDown { button: MouseButton::Left }));
    }

    #[test]
    fn test_raw_turn_accumulator_flushes_relative_smooth_mouse_move() {
        let rec = MacroRecorder::new();
        rec.is_recording.store(true, Ordering::SeqCst);
        rec.capture_mouse_moves.store(true, Ordering::SeqCst);

        // Put a simulated raw turn in the accumulator
        let now = Instant::now();
        if let Ok(mut lock) = RAW_TURN_ACCUMULATOR.lock() {
            *lock = Some(RawTurnAccumulator {
                accum_dx: 150,
                accum_dy: -45,
                start_time: now - std::time::Duration::from_millis(100),
                last_update: now,
            });
        }

        // Flush accumulator
        flush_raw_turn_accumulator(&rec);

        let events = rec.events.lock().unwrap().clone();
        assert_eq!(events.len(), 1);
        match &events[0] {
            MacroEvent::MouseMove { x, y, coord_mode, smooth, duration_ms, .. } => {
                assert_eq!(*x, 150);
                assert_eq!(*y, -45);
                assert_eq!(*coord_mode, MouseCoordMode::Relative);
                assert!(*smooth);
                assert!(duration_ms.unwrap() >= 16);
            }
            _ => panic!("Expected MacroEvent::MouseMove with Relative coord mode"),
        }
    }
}
