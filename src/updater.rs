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
    let clean = v.trim().trim_start_matches(|c| c == 'v' || c == 'V');
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
            let c_clean = current.trim().trim_start_matches(|c| c == 'v' || c == 'V');
            let r_clean = remote.trim().trim_start_matches(|c| c == 'v' || c == 'V');
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

    let remote_ver = release.tag_name.trim().trim_start_matches(|c| c == 'v' || c == 'V').to_string();

    if !is_newer_version(current_version, &release.tag_name) {
        return Ok(None);
    }

    // Find the standalone binary executable asset
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("ProcessHandler.exe"))
        .or_else(|| release.assets.iter().find(|a| a.name.to_lowercase().ends_with(".exe")))
        .or_else(|| release.assets.first());

    let (asset_url, asset_name, asset_size) = match asset {
        Some(a) => (a.browser_download_url.clone(), a.name.clone(), a.size),
        None => return Err(format!("Release v{} does not contain an executable asset", remote_ver)),
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

/// Downloads raw binary from `asset_url` with streaming progress updates,
/// terminates daemon, swaps executable in place, and restarts both daemon and GUI.
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
        .map_err(|e| format!("Failed to start binary download: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to resolve current exe path: {e}"))?;
    let pending_path = exe_path.with_extension("pending");

    let mut file = std::fs::File::create(&pending_path)
        .map_err(|e| format!("Failed to create pending file {:?}: {e}", pending_path))?;

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

    file.flush().map_err(|e| format!("Failed to flush file: {e}"))?;
    drop(file);

    let metadata = std::fs::metadata(&pending_path).map_err(|e| format!("Failed to verify downloaded binary: {e}"))?;
    if metadata.len() == 0 {
        let _ = std::fs::remove_file(&pending_path);
        return Err("Downloaded binary file is empty".to_string());
    }

    // Stop daemon cleanly
    let _ = crate::ipc::IpcClient::call(&crate::ipc::IpcRequest::ShutdownDaemon);

    // Give daemon process a moment to exit and release file/mutex handles
    std::thread::sleep(std::time::Duration::from_millis(350));

    apply_update_and_restart(&exe_path, &pending_path)?;

    Ok(())
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

    // Windows NTFS allows renaming a running executable file to .old and placing the new file at the original path.
    let in_place_renamed = std::fs::rename(exe_path, &old_path).is_ok()
        && std::fs::rename(pending_path, exe_path).is_ok();

    let ps_command = if in_place_renamed {
        format!(
            "Wait-Process -Id {gui_pid} -Timeout 10 -ErrorAction SilentlyContinue; \
             Start-Sleep -Milliseconds 600; \
             Remove-Item -LiteralPath '{old_str}' -Force -ErrorAction SilentlyContinue; \
             Start-Process -FilePath '{exe_str}'",
            gui_pid = gui_pid,
            old_str = old_str,
            exe_str = exe_str,
        )
    } else {
        format!(
            "Wait-Process -Id {gui_pid} -Timeout 10 -ErrorAction SilentlyContinue; \
             Start-Sleep -Milliseconds 600; \
             Move-Item -LiteralPath '{exe_str}' -Destination '{old_str}' -Force -ErrorAction SilentlyContinue; \
             Move-Item -LiteralPath '{pending_str}' -Destination '{exe_str}' -Force; \
             Start-Process -FilePath '{exe_str}'",
            gui_pid = gui_pid,
            old_str = old_str,
            exe_str = exe_str,
            pending_str = pending_str,
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
                    "name": "ProcessHandler.exe",
                    "browser_download_url": "https://github.com/psyattack/ProcessHandler/releases/download/v1.2.0/ProcessHandler.exe",
                    "size": 15728640
                }
            ]
        }"###;

        let release: GhRelease = serde_json::from_str(json).expect("failed to deserialize release");
        assert_eq!(release.tag_name, "v1.2.0");
        assert_eq!(release.assets.len(), 2);

        let exe_asset = release.assets.iter()
            .find(|a| a.name.eq_ignore_ascii_case("ProcessHandler.exe"))
            .expect("should find ProcessHandler.exe");

        assert_eq!(exe_asset.name, "ProcessHandler.exe");
        assert_eq!(exe_asset.size, 15728640);
        assert!(is_newer_version("1.0.0", &release.tag_name));
    }
}
