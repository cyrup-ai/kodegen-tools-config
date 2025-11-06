use crate::system_info::{ClientInfo, ClientRecord, SystemInfo, get_system_info};
use serde::{Deserialize, Serialize};

// ============================================================================
// DEFAULT VALUE FUNCTIONS
// ============================================================================

pub(crate) fn default_fuzzy_search_threshold() -> f64 {
    0.7
}

pub(crate) fn default_http_connection_timeout_secs() -> u64 {
    5
}

pub(crate) fn default_path_validation_timeout_ms() -> u64 {
    30_000  // 30 seconds (increased from hardcoded 10s)
}

// ============================================================================
// SERVER CONFIG
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Commands that cannot be executed
    pub blocked_commands: Vec<String>,

    /// Default shell for command execution
    pub default_shell: String,

    /// Directories the server can access (empty = full access)
    pub allowed_directories: Vec<String>,

    /// Directories the server cannot access
    pub denied_directories: Vec<String>,

    /// Max lines for file read operations
    pub file_read_line_limit: usize,

    /// Max lines per file write operation
    pub file_write_line_limit: usize,

    /// Minimum similarity ratio (0.0-1.0) for fuzzy search suggestions
    /// Default: 0.7 (70% similarity required)
    #[serde(default = "default_fuzzy_search_threshold")]
    pub fuzzy_search_threshold: f64,

    /// HTTP connection timeout in seconds (default: 5)
    #[serde(default = "default_http_connection_timeout_secs")]
    pub http_connection_timeout_secs: u64,

    /// Path validation timeout in milliseconds (default: 30000ms = 30 seconds)
    /// Increase for slow network filesystems (NFS, SMB, S3FS)
    #[serde(default = "default_path_validation_timeout_ms")]
    pub path_validation_timeout_ms: u64,

    /// Currently connected client (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_client: Option<ClientInfo>,

    /// History of all clients that have connected
    #[serde(default)]
    pub client_history: Vec<ClientRecord>,

    /// System diagnostic information (populated on every `get_config` call)
    pub system_info: SystemInfo,

    /// Total config save failures (populated on get_config call)
    #[serde(default)]
    pub save_error_count: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            blocked_commands: vec![
                "rm".to_string(),
                "rmdir".to_string(),
                "del".to_string(),
                "format".to_string(),
                "dd".to_string(),
                "shred".to_string(),
                "sudo".to_string(),
                "su".to_string(),
                "passwd".to_string(),
                "useradd".to_string(),
                "userdel".to_string(),
                "chmod".to_string(),
                "chown".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                "poweroff".to_string(),
            ],
            default_shell: if cfg!(windows) {
                "powershell.exe".to_string()
            } else {
                "/bin/sh".to_string()
            },
            allowed_directories: Vec::new(),
            denied_directories: Vec::new(),
            file_read_line_limit: 1000,
            file_write_line_limit: 50,
            fuzzy_search_threshold: 0.7,
            http_connection_timeout_secs: 5,
            path_validation_timeout_ms: 30_000,
            current_client: None,
            client_history: Vec::new(),
            system_info: get_system_info(),
            save_error_count: 0,
        }
    }
}
