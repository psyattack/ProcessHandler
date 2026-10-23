use std::path::Path;
use serde::Deserialize;

pub const GITHUB_REPO: &str = "psyattack/ProcessHandler";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub html_url: String,
    pub asset_url: String,
    pub asset_name: String,
    pub asset_size: u64,
}

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    CheckStarted,
    CheckComplete {
        result: Result<Option<ReleaseInfo>, String>,
        is_manual: bool,
    },
    DownloadProgress(f32),
    DownloadComplete(Result<(), String>),
}

/// Parses a semver-like version string into (major, minor, patch).
/// Strips leading 'v' / 'V' and ignores trailing metadata/pre-release tags.
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let clean = v.trim().trim_start_matches(['v', 'V']);
    // Take only the numeric part before any '-' or '+' (e.g. 1.0.1-beta -> 1.0.1)
    let num_part = clean.split(&['-', '+'][..]).next().unwrap_or(clean);
    let mut parts = num_part.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((major, minor, patch))
}

/// Returns true if `remote` is strictly newer than `current`.
pub fn is_newer_version(current: &str, remote: &str) -> bool {
    match (parse_version(current), parse_version(remote)) {
        (Some((c_maj, c_min, c_pat)), Some((r_maj, r_min, r_pat))) => {
            (r_maj, r_min, r_pat) > (c_maj, c_min, c_pat)
        }
        _ => {
            // Fallback string comparison if unable to parse standard digits
            let c_clean = current.trim().trim_start_matches(['v', 'V']);
            let r_clean = remote.trim().trim_start_matches(['v', 'V']);
            r_clean != c_clean
        }
    }
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

/// Queries GitHub releases API for the latest release and checks if a newer version is available.
pub async fn check_github_release(repo: &str, current_version: &str) -> Result<Option<ReleaseInfo>, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent(concat!("ProcessHandler-Updater/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!("GitHub API returned HTTP {}", response.status()));
    }

    let release: GhRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release metadata: {e}"))?;

    let remote_ver = release.tag_name.trim().trim_start_matches(['v', 'V']).to_string();

    if !is_newer_version(current_version, &release.tag_name) {
        return Ok(None);
    }

    // Find the release asset (prioritizing portable zip archive ProcessHandler-*-Windows-x64.zip)
    let asset = release
        .assets
        .iter()
        .find(|a| {
            let name = a.name.to_lowercase();
            name.ends_with(".zip") && name.contains("processhandler")
        })
        .or_else(|| release.assets.iter().find(|a| a.name.to_lowercase().ends_with(".zip")))
        .or_else(|| release.assets.iter().find(|a| a.name.eq_ignore_ascii_case("ProcessHandler.exe")))
        .or_else(|| release.assets.iter().find(|a| a.name.to_lowercase().ends_with(".exe")))
        .or_else(|| release.assets.first());

    let (asset_url, asset_name, asset_size) = match asset {
        Some(a) => (a.browser_download_url.clone(), a.name.clone(), a.size),
        None => return Err(format!("Release v{} does not contain a suitable asset", remote_ver)),
    };

    let tag_name = release.tag_name.clone();
    let name = release.name.unwrap_or_else(|| format!("v{}", tag_name));

    Ok(Some(ReleaseInfo {
        version: remote_ver,
        tag_name: release.tag_name,
        name,
        body: release.body.unwrap_or_default(),
        html_url: release.html_url,
        asset_url,
        asset_name,
        asset_size,
    }))
}

/// Unpacks a zip archive into a temporary folder and extracts the main executable into `target_exe_path`.
pub fn extract_exe_from_zip(zip_path: &Path, target_exe_path: &Path) -> Result<(), String> {
    let extract_dir = zip_path.with_extension("extracted");
    if extract_dir.exists() {
        let _ = std::fs::remove_dir_all(&extract_dir);
    }
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create temporary extraction directory {:?}: {e}", extract_dir))?;

    let mut extracted_successfully = false;

    // 1. Try Windows built-in bsdtar (`tar.exe`)
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = std::process::Command::new("tar");
        cmd.args(["-xf", &zip_path.to_string_lossy(), "-C", &extract_dir.to_string_lossy()]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(status) = cmd.status() {
            if status.success() {
                extracted_successfully = true;
            }
        }
    }

    #[cfg(not(windows))]
    {
        let mut cmd = std::process::Command::new("tar");
        cmd.args(["-xf", &zip_path.to_string_lossy(), "-C", &extract_dir.to_string_lossy()]);
        if let Ok(status) = cmd.status() {
            if status.success() {
                extracted_successfully = true;
            }
        }
    }

    // 2. Fallback to PowerShell Expand-Archive if tar is unavailable
    #[cfg(windows)]
    if !extracted_successfully {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let zip_esc = zip_path.to_string_lossy().replace('\'', "''");
        let dir_esc = extract_dir.to_string_lossy().replace('\'', "''");
        let ps_script = format!(
            "Expand-Archive -LiteralPath '{zip_esc}' -DestinationPath '{dir_esc}' -Force"
        );
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(status) = cmd.status() {
            if status.success() {
                extracted_successfully = true;
            }
        }
    }

    if !extracted_successfully {
        let _ = std::fs::remove_dir_all(&extract_dir);
        return Err(format!("Failed to unpack zip archive {:?}", zip_path));
    }

    // Recursively locate ProcessHandler.exe or any .exe inside extract_dir
    fn find_exe(dir: &Path) -> Option<std::path::PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        let mut fallback_exe = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.eq_ignore_ascii_case("ProcessHandler.exe") {
                    return Some(path);
                } else if name.to_lowercase().ends_with(".exe") && fallback_exe.is_none() {
                    fallback_exe = Some(path);
                }
            } else if path.is_dir() {
                if let Some(found) = find_exe(&path) {
                    return Some(found);
                }
            }
        }
        fallback_exe
    }

    let found_exe = match find_exe(&extract_dir) {
        Some(p) => p,
        None => {
            let _ = std::fs::remove_dir_all(&extract_dir);
            return Err("No executable found inside downloaded zip archive".to_string());
        }
    };

    if target_exe_path.exists() {
        let _ = std::fs::remove_file(target_exe_path);
    }

    if let Err(e) = std::fs::copy(&found_exe, target_exe_path) {
        let _ = std::fs::remove_dir_all(&extract_dir);
        return Err(format!("Failed to copy extracted executable to {:?}: {e}", target_exe_path));
    }

    let _ = std::fs::remove_dir_all(&extract_dir);
    Ok(())
}

/// Downloads update package from `asset_url` with streaming progress updates,
/// unpacks executable if zipped, terminates daemon, swaps executable in place, and restarts both daemon and GUI.
pub async fn download_and_install<F>(asset_url: &str, progress_cb: F) -> Result<(), String>
where
    F: Fn(f32) + Send + 'static,
{
    let client = reqwest::Client::builder()
        .user_agent(concat!("ProcessHandler-Updater/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let mut response = client
        .get(asset_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to resolve current exe path: {e}"))?;
    let pending_path = exe_path.with_extension("pending");
    let download_path = exe_path.with_extension("download");

    if download_path.exists() {
        let _ = std::fs::remove_file(&download_path);
    }

    let mut file = std::fs::File::create(&download_path)
        .map_err(|e| format!("Failed to create download file {:?}: {e}", download_path))?;

    use std::io::Write;
    let mut downloaded: u64 = 0;

    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Error during download: {e}"))? {
        file.write_all(&chunk).map_err(|e| format!("Failed to write chunk: {e}"))?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let pct = (downloaded as f32 / total_size as f32).clamp(0.0, 1.0);
            progress_cb(pct);
        }
    }

    file.flush().map_err(|e| format!("Failed to flush download file: {e}"))?;
    drop(file);

    let metadata = std::fs::metadata(&download_path).map_err(|e| format!("Failed to verify downloaded file: {e}"))?;
    if metadata.len() == 0 {
        let _ = std::fs::remove_file(&download_path);
        return Err("Downloaded file is empty".to_string());
    }

    // Detect if downloaded file is a zip archive
    let is_zip = {
        let mut header = [0u8; 4];
        if let Ok(mut f) = std::fs::File::open(&download_path) {
            use std::io::Read;
            let read_bytes = f.read(&mut header).unwrap_or(0);
            (read_bytes >= 2 && &header[0..2] == b"PK") || asset_url.to_lowercase().contains(".zip")
        } else {
            asset_url.to_lowercase().contains(".zip")
        }
    };

    if is_zip {
        if let Err(e) = extract_exe_from_zip(&download_path, &pending_path) {
            let _ = std::fs::remove_file(&download_path);
            return Err(e);
        }
        let _ = std::fs::remove_file(&download_path);
    } else {
        if pending_path.exists() {
            let _ = std::fs::remove_file(&pending_path);
        }
        std::fs::rename(&download_path, &pending_path)
            .map_err(|e| format!("Failed to prepare pending binary: {e}"))?;
    }

    let pending_meta = std::fs::metadata(&pending_path).map_err(|e| format!("Failed to verify pending executable: {e}"))?;
    if pending_meta.len() == 0 {
        let _ = std::fs::remove_file(&pending_path);
        return Err("Pending executable file is empty".to_string());
    }

    // Stop daemon cleanly
    let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ShutdownDaemon);

    // Give daemon process a moment to exit and release file/mutex handles
    std::thread::sleep(std::time::Duration::from_millis(350));

    apply_update_and_restart(&exe_path, &pending_path)?;

    Ok(())
}

static GUI_CLOSED_DURING_UPDATE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_gui_closed_during_update(closed: bool) {
    GUI_CLOSED_DURING_UPDATE.store(closed, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_gui_closed_during_update() -> bool {
    GUI_CLOSED_DURING_UPDATE.load(std::sync::atomic::Ordering::SeqCst)
}

/// Performs executable swap and restarts both daemon and GUI.
pub fn apply_update_and_restart(exe_path: &Path, pending_path: &Path) -> Result<(), String> {
    let old_path = exe_path.with_extension("old");
    if old_path.exists() {
        let _ = std::fs::remove_file(&old_path);
    }

    let gui_pid = std::process::id();
    let old_str = old_path.to_string_lossy().to_string();
    let exe_str = exe_path.to_string_lossy().to_string();
    let pending_str = pending_path.to_string_lossy().to_string();

    let restart_args = if is_gui_closed_during_update() {
        " -ArgumentList '--daemon'"
    } else {
        ""
    };

    // Windows NTFS allows renaming a running executable file to .old and placing the new file at the original path.
    let in_place_renamed = std::fs::rename(exe_path, &old_path).is_ok()
        && std::fs::rename(pending_path, exe_path).is_ok();

    let ps_command = if in_place_renamed {
        format!(
            "Wait-Process -Id {gui_pid} -Timeout 10 -ErrorAction SilentlyContinue; \
             Start-Sleep -Milliseconds 600; \
             Remove-Item -LiteralPath '{old_str}' -Force -ErrorAction SilentlyContinue; \
             Start-Process -FilePath '{exe_str}'{restart_args}",
            gui_pid = gui_pid,
            old_str = old_str,
            exe_str = exe_str,
            restart_args = restart_args,
        )
    } else {
        format!(
            "Wait-Process -Id {gui_pid} -Timeout 10 -ErrorAction SilentlyContinue; \
             Start-Sleep -Milliseconds 600; \
             Move-Item -LiteralPath '{exe_str}' -Destination '{old_str}' -Force -ErrorAction SilentlyContinue; \
             Move-Item -LiteralPath '{pending_str}' -Destination '{exe_str}' -Force; \
             Start-Process -FilePath '{exe_str}'{restart_args}",
            gui_pid = gui_pid,
            old_str = old_str,
            exe_str = exe_str,
            pending_str = pending_str,
            restart_args = restart_args,
        )
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps_command]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.spawn().map_err(|e| format!("Failed to spawn restart script: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new(exe_path).spawn();
    }

    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(parse_version("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("v1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("V2.4.9"), Some((2, 4, 9)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("v1.1.0-beta.1"), Some((1, 1, 0)));
        assert_eq!(parse_version("v1.3.0+build.42"), Some((1, 3, 0)));
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("1.0.0", "v1.1.0"));
        assert!(is_newer_version("1.0.0", "v2.0.0"));
        assert!(is_newer_version("1.2.3", "v1.3.0"));
        assert!(is_newer_version("1.2.3", "v1.2.4"));

        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "v1.0.0"));
        assert!(!is_newer_version("1.1.0", "v1.0.9"));
        assert!(!is_newer_version("2.0.0", "v1.9.9"));
        assert!(!is_newer_version("1.0.5", "0.9.9"));
    }

    #[test]
    fn test_github_release_deserialization() {
        let json = r###"{
            "tag_name": "v1.2.0",
            "name": "ProcessHandler v1.2.0 - Studio Update",
            "body": "## What's Changed\n* Added auto-updater\n* Improved performance",
            "html_url": "https://github.com/psyattack/ProcessHandler/releases/tag/v1.2.0",
            "assets": [
                {
                    "name": "checksums.txt",
                    "browser_download_url": "https://github.com/psyattack/ProcessHandler/releases/download/v1.2.0/checksums.txt",
                    "size": 128
                },
                {
                    "name": "ProcessHandler-v1.2.0-Windows-x64.zip",
                    "browser_download_url": "https://github.com/psyattack/ProcessHandler/releases/download/v1.2.0/ProcessHandler-v1.2.0-Windows-x64.zip",
                    "size": 15728640
                }
            ]
        }"###;

        let release: GhRelease = serde_json::from_str(json).expect("failed to deserialize release");
        assert_eq!(release.tag_name, "v1.2.0");
        assert_eq!(release.assets.len(), 2);

        let zip_asset = release.assets.iter()
            .find(|a| {
                let name = a.name.to_lowercase();
                name.ends_with(".zip") && name.contains("processhandler")
            })
            .expect("should find ProcessHandler zip");

        assert_eq!(zip_asset.name, "ProcessHandler-v1.2.0-Windows-x64.zip");
        assert_eq!(zip_asset.size, 15728640);
        assert!(is_newer_version("1.0.0", &release.tag_name));
    }

    #[test]
    fn test_extract_exe_from_zip() {
        let temp_dir = std::env::temp_dir().join(format!("ph_test_zip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let source_exe = temp_dir.join("ProcessHandler.exe");
        std::fs::write(&source_exe, b"MZ_MOCK_EXECUTABLE_BINARY_DATA").unwrap();

        let zip_path = temp_dir.join("test_archive.zip");

        let mut zip_created = false;
        #[cfg(windows)]
        {
            let ps_script = format!(
                "Compress-Archive -Path '{}' -DestinationPath '{}' -Force",
                source_exe.to_string_lossy().replace('\'', "''"),
                zip_path.to_string_lossy().replace('\'', "''")
            );
            let status = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
                .status();
            if let Ok(s) = status {
                zip_created = s.success();
            }
        }

        if zip_created && zip_path.exists() {
            let extracted_target = temp_dir.join("Extracted_ProcessHandler.exe");
            let result = extract_exe_from_zip(&zip_path, &extracted_target);
            assert!(result.is_ok(), "Extraction failed: {:?}", result.err());
            assert!(extracted_target.exists());
            let extracted_data = std::fs::read(&extracted_target).unwrap();
            assert_eq!(extracted_data, b"MZ_MOCK_EXECUTABLE_BINARY_DATA");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
