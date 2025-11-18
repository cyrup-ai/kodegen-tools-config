mod get_config;
mod set_config_value;

pub use get_config::GetConfigTool;
pub use set_config_value::SetConfigValueTool;

// Re-export ConfigManager and types from infrastructure crate
pub use kodegen_config_manager::{ConfigManager, ConfigValue, ServerConfig, get_system_info};

/// Start the config HTTP server programmatically
///
/// Returns a ServerHandle for graceful shutdown control.
/// This function is non-blocking - the server runs in background tasks.
///
/// # Arguments
/// * `addr` - Socket address to bind to (e.g., "127.0.0.1:30441")
/// * `tls_cert` - Optional path to TLS certificate file
/// * `tls_key` - Optional path to TLS private key file
///
/// # Returns
/// ServerHandle for graceful shutdown, or error if startup fails
pub async fn start_server(
    addr: std::net::SocketAddr,
    tls_cert: Option<std::path::PathBuf>,
    tls_key: Option<std::path::PathBuf>,
) -> anyhow::Result<kodegen_server_http::ServerHandle> {
    use kodegen_server_http::{create_http_server, Managers, RouterSet, register_tool};
    use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
    use std::time::Duration;

    let tls_config = match (tls_cert, tls_key) {
        (Some(cert), Some(key)) => Some((cert, key)),
        _ => None,
    };

    let shutdown_timeout = Duration::from_secs(30);
    let session_keep_alive = Duration::ZERO;

    create_http_server("config", addr, tls_config, shutdown_timeout, session_keep_alive, |config, _tracker| {
        let config = config.clone();
        Box::pin(async move {
            let tool_router = ToolRouter::new();
            let prompt_router = PromptRouter::new();
            let managers = Managers::new();

            // Register config tools
            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                GetConfigTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                SetConfigValueTool::new(config.clone()),
            );

            Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
    })
    .await
}
