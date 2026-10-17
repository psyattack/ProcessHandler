use std::io::{Read, Write};
use crate::ipc::{IpcRequest, IpcResponse, PIPE_NAME};

pub struct IpcClient;

impl IpcClient {
    pub fn is_daemon_running() -> bool {
        Self::call(&IpcRequest::Ping).is_ok()
    }

    pub fn call(req: &IpcRequest) -> Result<IpcResponse, String> {
        let req_bytes = serde_json::to_vec(req).map_err(|e| e.to_string())?;
        let len_bytes = (req_bytes.len() as u32).to_le_bytes();

        let mut pipe = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(PIPE_NAME)
            .map_err(|e| format!("Cannot connect to daemon pipe: {e}"))?;

        // Write length + payload
        pipe.write_all(&len_bytes).map_err(|e| e.to_string())?;
        pipe.write_all(&req_bytes).map_err(|e| e.to_string())?;
        pipe.flush().map_err(|e| e.to_string())?;

        // Read response length
        let mut resp_len_buf = [0u8; 4];
        pipe.read_exact(&mut resp_len_buf).map_err(|e| e.to_string())?;
        let resp_len = u32::from_le_bytes(resp_len_buf) as usize;

        if resp_len > 16 * 1024 * 1024 {
            return Err("Response payload too large".into());
        }

        let mut resp_buf = vec![0u8; resp_len];
        pipe.read_exact(&mut resp_buf).map_err(|e| e.to_string())?;

        let response: IpcResponse = serde_json::from_slice(&resp_buf).map_err(|e| e.to_string())?;
        Ok(response)
    }
}
