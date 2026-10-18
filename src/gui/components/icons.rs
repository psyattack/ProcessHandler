//! Win32 process icon extraction and GPU texture caching.

use std::collections::HashMap;
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::CloseHandle;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC,
    BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
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

    pub fn get_icon_from_path(&mut self, ctx: &Context, path: &str) -> Option<&TextureHandle> {
        let key = format!("path:{}", path.to_lowercase());
        if !self.cache.contains_key(&key) {
            let loaded = load_path_texture(ctx, path);
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

fn load_path_texture(ctx: &Context, path: &str) -> Option<TextureHandle> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return None;
    }

    #[cfg(windows)]
    {
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if ext == "exe" || ext == "ico" || ext == "dll" || ext == "lnk" {
            if let Some(color_image) = extract_icon_from_file_path(path) {
                return Some(ctx.load_texture(
                    format!("path_icon_{}", path),
                    color_image,
                    TextureOptions::LINEAR,
                ));
            }
        }
    }

    if let Ok(img) = image::open(p) {
        let rgba = img.to_rgba8();
        let final_img = if rgba.width() > 64 || rgba.height() > 64 {
            image::imageops::thumbnail(&rgba, 64, 64)
        } else {
            rgba
        };
        let size = [final_img.width() as usize, final_img.height() as usize];
        let pixels = final_img.into_raw();
        let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
        return Some(ctx.load_texture(
            format!("path_img_{}", path),
            color_image,
            TextureOptions::LINEAR,
        ));
    }

    #[cfg(windows)]
    {
        if let Some(color_image) = extract_icon_from_file_path(path) {
            return Some(ctx.load_texture(
                format!("path_icon_fb_{}", path),
                color_image,
                TextureOptions::LINEAR,
            ));
        }
    }

    None
}

#[cfg(windows)]
fn extract_icon_from_file_path(path: &str) -> Option<ColorImage> {
    unsafe {
        let path_w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut large_icon = HICON::default();
        let mut small_icon = HICON::default();
        let count = ExtractIconExW(PCWSTR(path_w.as_ptr()), 0, Some(&mut large_icon), Some(&mut small_icon), 1);
        if count > 0 {
            let hicon = if !large_icon.is_invalid() {
                if !small_icon.is_invalid() {
                    let _ = DestroyIcon(small_icon);
                }
                large_icon
            } else if !small_icon.is_invalid() {
                small_icon
            } else {
                return None;
            };

            let result = hicon_to_color_image(hicon);
            let _ = DestroyIcon(hicon);
            return result;
        }
    }
    None
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

    let hbm = if !icon_info.hbmColor.is_invalid() {
        icon_info.hbmColor
    } else {
        icon_info.hbmMask
    };

    let mut bm: BITMAP = std::mem::zeroed();
    if GetObjectW(
        hbm.into(),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut _),
    ) == 0
    {
        if !icon_info.hbmColor.is_invalid() {
            let _ = DeleteObject(icon_info.hbmColor.into());
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(icon_info.hbmMask.into());
        }
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
        return None;
    }

    let width = bm.bmWidth;
    let height = if icon_info.hbmColor.is_invalid() {
        bm.bmHeight / 2
    } else {
        bm.bmHeight
    };

    if width <= 0 || height <= 0 {
        if !icon_info.hbmColor.is_invalid() {
            let _ = DeleteObject(icon_info.hbmColor.into());
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(icon_info.hbmMask.into());
        }
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
        return None;
    }

    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -height; // Top-down DIB
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let mut buf = vec![0u8; (width * height * 4) as usize];

    let lines = GetDIBits(
        hdc_mem,
        hbm,
        0,
        height as u32,
        Some(buf.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );

    if lines == 0 {
        if !icon_info.hbmColor.is_invalid() {
            let _ = DeleteObject(icon_info.hbmColor.into());
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(icon_info.hbmMask.into());
        }
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
        return None;
    }

    // Convert BGRA to RGBA
    for pixel in buf.as_chunks_mut::<4>().0 {
        let b = pixel[0];
        let r = pixel[2];
        pixel[0] = r;
        pixel[2] = b;
    }

    // If icon has 0 alpha everywhere (common in 24-bit/16-bit Win32 icons), extract mask or set to 255
    let has_alpha = buf.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
    if !has_alpha {
        let mut mask_applied = false;
        if !icon_info.hbmMask.is_invalid() {
            let mut mask_bmi: BITMAPINFO = std::mem::zeroed();
            mask_bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            mask_bmi.bmiHeader.biWidth = width;
            mask_bmi.bmiHeader.biHeight = -height;
            mask_bmi.bmiHeader.biPlanes = 1;
            mask_bmi.bmiHeader.biBitCount = 32;
            mask_bmi.bmiHeader.biCompression = BI_RGB.0;
            let mut mask_buf = vec![0u8; (width * height * 4) as usize];
            let mask_lines = GetDIBits(
                hdc_mem,
                icon_info.hbmMask,
                0,
                height as u32,
                Some(mask_buf.as_mut_ptr() as *mut _),
                &mut mask_bmi,
                DIB_RGB_COLORS,
            );
            if mask_lines > 0 {
                for (pixel, mask_pixel) in buf.as_chunks_mut::<4>().0.iter_mut().zip(mask_buf.as_chunks::<4>().0.iter()) {
                    // AND mask bit is 0 for opaque content, non-zero for transparent
                    pixel[3] = if mask_pixel[0] == 0 { 255 } else { 0 };
                }
                mask_applied = true;
            }
        }
        if !mask_applied {
            for pixel in buf.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
        }
    }

    // Clean up GDI handles
    if !icon_info.hbmColor.is_invalid() {
        let _ = DeleteObject(icon_info.hbmColor.into());
    }
    if !icon_info.hbmMask.is_invalid() {
        let _ = DeleteObject(icon_info.hbmMask.into());
    }
    let _ = DeleteDC(hdc_mem);
    ReleaseDC(None, hdc_screen);

    let img = image::RgbaImage::from_raw(width as u32, height as u32, buf)?;
    let final_img = if width > 64 || height > 64 {
        image::imageops::thumbnail(&img, 64, 64)
    } else {
        img
    };

    let (w, h) = final_img.dimensions();
    Some(ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        final_img.as_raw(),
    ))
}
