//! Win32 process icon extraction and GPU texture caching.

use std::collections::HashMap;
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::CloseHandle;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(windows)]
use windows::Win32::UI::Shell::ExtractIconExW;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

pub struct ProcessIconCache {
    cache: HashMap<String, Option<TextureHandle>>,
}

impl ProcessIconCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get_icon(&mut self, ctx: &Context, pid: u32, name: &str) -> Option<&TextureHandle> {
        let key = name.to_lowercase();
        if !self.cache.contains_key(&key) {
            let loaded = load_process_texture(ctx, pid, name);
            self.cache.insert(key.clone(), loaded);
        }
        self.cache.get(&key).and_then(|opt| opt.as_ref())
    }
}

fn load_process_texture(ctx: &Context, pid: u32, name: &str) -> Option<TextureHandle> {
    #[cfg(windows)]
    {
        let color_image = extract_process_icon(pid, name)?;
        Some(ctx.load_texture(
            format!("proc_icon_{}", name),
            color_image,
            TextureOptions::LINEAR,
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (ctx, pid, name);
        None
    }
}

#[cfg(windows)]
fn extract_process_icon(pid: u32, name: &str) -> Option<ColorImage> {
    unsafe {
        let hicon = get_process_hicon(pid, name)?;
        let result = hicon_to_color_image(hicon);
        let _ = DestroyIcon(hicon);
        result
    }
}

#[cfg(windows)]
unsafe fn get_process_hicon(pid: u32, name: &str) -> Option<HICON> {
    // 1. Try to query full executable path from PID
    let mut exe_path: Option<String> = None;
    if let Ok(h_proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        if QueryFullProcessImageNameW(
            h_proc,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok()
        {
            exe_path = Some(String::from_utf16_lossy(&buf[..size as usize]));
        }
        let _ = CloseHandle(h_proc);
    }

    // 2. Extract 16x16 icon from path
    if let Some(path) = exe_path {
        let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut small_icon = HICON::default();
        let count = ExtractIconExW(PCWSTR(path_w.as_ptr()), 0, None, Some(&mut small_icon), 1);
        if count > 0 && !small_icon.is_invalid() {
            return Some(small_icon);
        }
    }

    // 3. Fallback: Check System32
    let sys_root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows"));
    let sys32_candidate = sys_root.join("System32").join(name);
    if sys32_candidate.exists() {
        let path_w: Vec<u16> = sys32_candidate.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
        let mut small_icon = HICON::default();
        let count = ExtractIconExW(PCWSTR(path_w.as_ptr()), 0, None, Some(&mut small_icon), 1);
        if count > 0 && !small_icon.is_invalid() {
            return Some(small_icon);
        }
    }

    None
}

#[cfg(windows)]
unsafe fn hicon_to_color_image(hicon: HICON) -> Option<ColorImage> {
    let mut icon_info: ICONINFO = std::mem::zeroed();
    if GetIconInfo(hicon, &mut icon_info).is_err() {
        return None;
    }

    let hdc_screen = GetDC(None);
    let hdc_mem = CreateCompatibleDC(Some(hdc_screen));

    let width = 16i32;
    let height = 16i32;

    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -height; // Top-down DIB
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let mut buf = vec![0u8; (width * height * 4) as usize];
    let hbm = if !icon_info.hbmColor.is_invalid() {
        icon_info.hbmColor
    } else {
        icon_info.hbmMask
    };

    let lines = GetDIBits(
        hdc_mem,
        hbm,
        0,
        height as u32,
        Some(buf.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );

    // Clean up GDI handles
    if !icon_info.hbmColor.is_invalid() {
        let _ = DeleteObject(icon_info.hbmColor.into());
    }
    if !icon_info.hbmMask.is_invalid() {
        let _ = DeleteObject(icon_info.hbmMask.into());
    }
    let _ = DeleteDC(hdc_mem);
    ReleaseDC(None, hdc_screen);

    if lines == 0 {
        return None;
    }

    // Convert BGRA to RGBA
    for pixel in buf.as_chunks_mut::<4>().0 {
        let b = pixel[0];
        let r = pixel[2];
        pixel[0] = r;
        pixel[2] = b;
    }

    // If icon has 0 alpha everywhere (common in 24-bit/16-bit Win32 icons), set to 255
    let has_alpha = buf.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
    if !has_alpha {
        for pixel in buf.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
    }

    Some(ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        &buf,
    ))
}
