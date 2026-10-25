//! Discord Rich Presence client for ProcessHandler
//! Communicates via Windows Named Pipe (\\.\pipe\discord-ipc-0..9)
//! Completely asynchronous, lightweight, zero external Discord dependencies.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::sync::Mutex;
use crate::models::DiscordRpcButton;

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Discord IPC OpCodes
const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;

pub struct DiscordRpcClient {
    pipe: NamedPipeClient,
    pub current_app_id: String,
}

impl DiscordRpcClient {
    /// Attempts to connect to any available Discord IPC pipe (0..9)
    pub async fn connect(app_id: &str) -> Result<Self, String> {
        let mut last_err = String::new();
        for pipe_num in 0..10 {
            let pipe_path = format!(r"\\.\pipe\discord-ipc-{}", pipe_num);
            match ClientOptions::new().open(&pipe_path) {
                Ok(mut pipe) => {
                    // Send Handshake
                    let handshake_payload = json!({
                        "v": 1,
                        "client_id": app_id
                    });
                    if let Err(e) = Self::send_raw(&mut pipe, OP_HANDSHAKE, &handshake_payload).await {
                        last_err = format!("Handshake failed on {pipe_path}: {e}");
                        continue;
                    }

                    // Read Handshake Response
                    match Self::read_raw(&mut pipe).await {
                        Ok((_opcode, resp)) => {
                            if resp.get("evt").and_then(Value::as_str) == Some("ERROR") {
                                return Err(format!("Discord handshake error: {:?}", resp));
                            }
                            return Ok(Self {
                                pipe,
                                current_app_id: app_id.to_string(),
                            });
                        }
                        Err(e) => {
                            last_err = format!("Failed reading handshake response on {pipe_path}: {e}");
                        }
                    }
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }
        }
        Err(format!("Could not connect to Discord IPC (is Discord running?): {last_err}"))
    }

    async fn send_raw(pipe: &mut NamedPipeClient, opcode: u32, payload: &Value) -> Result<(), String> {
        let bytes = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
        let len = bytes.len() as u32;

        let mut header = [0u8; 8];
        header[0..4].copy_from_slice(&opcode.to_le_bytes());
        header[4..8].copy_from_slice(&len.to_le_bytes());

        pipe.write_all(&header).await.map_err(|e| e.to_string())?;
        pipe.write_all(&bytes).await.map_err(|e| e.to_string())?;
        pipe.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn read_raw(pipe: &mut NamedPipeClient) -> Result<(u32, Value), String> {
        let mut header = [0u8; 8];
        tokio::time::timeout(Duration::from_millis(1500), pipe.read_exact(&mut header))
            .await
            .map_err(|_| "Discord IPC read timed out".to_string())?
            .map_err(|e| e.to_string())?;

        let opcode = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        if len > 10 * 1024 * 1024 {
            return Err("Discord IPC response packet too large".to_string());
        }

        let mut body = vec![0u8; len];
        tokio::time::timeout(Duration::from_millis(1500), pipe.read_exact(&mut body))
            .await
            .map_err(|_| "Discord IPC body read timed out".to_string())?
            .map_err(|e| e.to_string())?;

        let val: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
        Ok((opcode, val))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn set_activity(
        &mut self,
        details: &str,
        state: &str,
        large_image: Option<&str>,
        large_text: Option<&str>,
        small_image: Option<&str>,
        small_text: Option<&str>,
        show_elapsed: bool,
        buttons: &[DiscordRpcButton],
    ) -> Result<(), String> {
        let nonce = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();

        let mut activity = json!({});

        if !details.is_empty() {
            activity["details"] = json!(details);
        }
        if !state.is_empty() {
            activity["state"] = json!(state);
        }

        if show_elapsed {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            activity["timestamps"] = json!({
                "start": now
            });
        }

        let mut assets = json!({});
        if let Some(img) = large_image {
            if !img.is_empty() {
                assets["large_image"] = json!(img);
                if let Some(txt) = large_text {
                    if !txt.is_empty() {
                        assets["large_text"] = json!(txt);
                    }
                }
            }
        }
        if let Some(img) = small_image {
            if !img.is_empty() {
                assets["small_image"] = json!(img);
                if let Some(txt) = small_text {
                    if !txt.is_empty() {
                        assets["small_text"] = json!(txt);
                    }
                }
            }
        }
        if assets.as_object().is_some_and(|o| !o.is_empty()) {
            activity["assets"] = assets;
        }

        if !buttons.is_empty() {
            let btn_array: Vec<Value> = buttons
                .iter()
                .filter(|b| !b.label.is_empty() && !b.url.is_empty())
                .take(2) // Discord allows at most 2 buttons
                .map(|b| {
                    json!({
                        "label": b.label,
                        "url": b.url
                    })
                })
                .collect();
            if !btn_array.is_empty() {
                activity["buttons"] = json!(btn_array);
            }
        }

        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": activity
            },
            "nonce": nonce
        });

        Self::send_raw(&mut self.pipe, OP_FRAME, &payload).await?;
        // Drain optional confirmation response
        let _ = Self::read_raw(&mut self.pipe).await;
        Ok(())
    }

    pub async fn clear_activity(&mut self) -> Result<(), String> {
        let nonce = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": Value::Null
            },
            "nonce": nonce
        });

        Self::send_raw(&mut self.pipe, OP_FRAME, &payload).await?;
        let _ = Self::read_raw(&mut self.pipe).await;
        Ok(())
    }
}

// Global thread-safe client manager
static GLOBAL_DISCORD: Mutex<Option<DiscordRpcClient>> = Mutex::const_new(None);

#[allow(clippy::too_many_arguments)]
pub async fn update_presence(
    app_id: &str,
    details: &str,
    state: &str,
    large_image: Option<&str>,
    large_text: Option<&str>,
    small_image: Option<&str>,
    small_text: Option<&str>,
    show_elapsed: bool,
    buttons: &[DiscordRpcButton],
) -> Result<(), String> {
    let mut lock = GLOBAL_DISCORD.lock().await;

    // Check if we need to reconnect (different app_id or client dropped)
    let need_connect = match &*lock {
        Some(client) => client.current_app_id != app_id,
        None => true,
    };

    if need_connect {
        let new_client = DiscordRpcClient::connect(app_id).await?;
        *lock = Some(new_client);
    }

    if let Some(client) = lock.as_mut() {
        if let Err(err) = client
            .set_activity(
                details,
                state,
                large_image,
                large_text,
                small_image,
                small_text,
                show_elapsed,
                buttons,
            )
            .await
        {
            // If pipe broke, reset client so next attempt reconnects
            *lock = None;
            return Err(format!("Failed to set Discord activity: {err}"));
        }
    }
    Ok(())
}

pub async fn clear_presence() -> Result<(), String> {
    let mut lock = GLOBAL_DISCORD.lock().await;
    if let Some(client) = lock.as_mut() {
        let _ = client.clear_activity().await;
    }
    *lock = None;
    Ok(())
}
