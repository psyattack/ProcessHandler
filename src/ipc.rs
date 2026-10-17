pub mod client;
pub mod protocol;

pub use client::IpcClient;
pub use protocol::*;

pub const PIPE_NAME: &str = r"\\.\pipe\ProcessHandler_Daemon_v2";
