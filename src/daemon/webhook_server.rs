use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use crate::daemon::engine::DaemonEngine;
use crate::models::Trigger;

pub const DEFAULT_WEBHOOK_PORT: u16 = 48123;

pub struct WebhookServer {
    engine: Arc<Mutex<DaemonEngine>>,
    port: u16,
}

impl WebhookServer {
    pub fn new(engine: Arc<Mutex<DaemonEngine>>, port: u16) -> Self {
        Self { engine, port }
    }

    pub async fn run(&self) {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => {
                println!("[DAEMON] Webhook server listening on http://{}", addr);
                l
            }
            Err(e) => {
                eprintln!("[DAEMON] Failed to bind webhook server on {}: {}", addr, e);
                return;
            }
        };

        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            let engine_clone = Arc::clone(&self.engine);
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };

                let req = String::from_utf8_lossy(&buf[..n]);
                let first_line = req.lines().next().unwrap_or_default();
                let parts: Vec<&str> = first_line.split_whitespace().collect();

                if parts.len() >= 2 {
                    let path = parts[1];
                    let prefix = "/api/v1/trigger/";
                    if let Some(target) = path.strip_prefix(prefix) {
                        let target_clean = target.trim_end_matches('/').to_string();
                        let mut eng = engine_clone.lock().await;

                        let matching_rule = eng.config.rules.iter().find(|r| {
                            if !r.is_enabled {
                                return false;
                            }
                            if r.id == target_clean {
                                return true;
                            }
                            if let Some(Trigger::Webhook { endpoint_path }) = &r.trigger {
                                if endpoint_path == &target_clean || endpoint_path.trim_start_matches('/') == target_clean {
                                    return true;
                                }
                            }
                            false
                        }).cloned();

                        if let Some(rule) = matching_rule {
                            let _ = eng.try_execute_rule(&rule, &format!("Webhook ({})", target_clean), 0).await;
                            let body = format!("{{\"status\":\"triggered\",\"rule\":\"{}\"}}\n", rule.name);
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            return;
                        } else {
                            let body = format!("{{\"status\":\"error\",\"message\":\"Rule '{}' not found or disabled\"}}\n", target_clean);
                            let resp = format!(
                                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            return;
                        }
                    }
                }

                let body = "{\"status\":\"error\",\"message\":\"Invalid endpoint. Use /api/v1/trigger/<endpoint_or_rule_id>\"}\n";
                let resp = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    }
}
