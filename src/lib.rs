mod get_config;
mod set_config_value;

pub use get_config::GetConfigTool;
pub use set_config_value::SetConfigValueTool;

// Re-export ConfigManager and types from infrastructure crate
pub use kodegen_config_manager::{ConfigManager, ConfigValue, ServerConfig, get_system_info};

// ConfigServer type definition for manual HTTP server
#[derive(Clone)]
struct ConfigServer {
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
    prompt_router: rmcp::handler::server::router::prompt::PromptRouter<Self>,
    usage_tracker: kodegen_utils::usage_tracker::UsageTracker,
    config_manager: ConfigManager,
}

impl rmcp::ServerHandler for ConfigServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: rmcp::model::Implementation::from_build_env(),
            instructions: Some("KODEGEN Config Category Server".to_string()),
        }
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let tool_name = request.name.clone();
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        let result = self.tool_router.call(tcc).await;

        if result.is_ok() {
            self.usage_tracker.track_success(&tool_name);
        } else {
            self.usage_tracker.track_failure(&tool_name);
        }

        result
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        let items = self.tool_router.list_all();
        Ok(rmcp::model::ListToolsResult::with_all_items(items))
    }

    async fn get_prompt(
        &self,
        request: rmcp::model::GetPromptRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::GetPromptResult, rmcp::ErrorData> {
        let pcc = rmcp::handler::server::prompt::PromptContext::new(
            self,
            request.name,
            request.arguments,
            context,
        );
        self.prompt_router.get_prompt(pcc).await
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListPromptsResult, rmcp::ErrorData> {
        let items = self.prompt_router.list_all();
        Ok(rmcp::model::ListPromptsResult::with_all_items(items))
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListResourcesResult, rmcp::ErrorData> {
        Ok(rmcp::model::ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParam,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ReadResourceResult, rmcp::ErrorData> {
        Err(rmcp::ErrorData::resource_not_found(
            "resource_not_found",
            Some(serde_json::json!({ "uri": request.uri })),
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListResourceTemplatesResult, rmcp::ErrorData> {
        Ok(rmcp::model::ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        request: rmcp::model::InitializeRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::InitializeResult, rmcp::ErrorData> {
        let _ = self.config_manager.set_client_info(request.client_info).await;
        Ok(self.get_info())
    }
}

/// Start the config tools HTTP server programmatically
///
/// This function uses a manual HTTP server implementation because kodegen_server_http
/// depends on kodegen_config_manager::ConfigManager, creating a circular dependency.
///
/// Returns a ServerHandle for graceful shutdown control.
/// This function is non-blocking - the server runs in background tasks.
///
/// # Arguments
/// * `addr` - Socket address to bind to
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
    use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, StreamableHttpServerConfig,
        session::local::LocalSessionManager,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use axum::Router;
    use tower_http::cors::CorsLayer;
    use tokio::sync::oneshot;
    use tokio_util::sync::CancellationToken;
    use kodegen_mcp_tool::Tool;

    let _ = env_logger::try_init();
    let _ = rustls::crypto::ring::default_provider().install_default();

    let instance_id = chrono::Utc::now().format("%Y%m%d-%H%M%S-config").to_string();

    let config_manager = ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = kodegen_utils::usage_tracker::UsageTracker::new(instance_id.clone());

    kodegen_mcp_tool::tool_history::init_global_history(instance_id).await;

    let tool_router = ToolRouter::new();
    let prompt_router = PromptRouter::new();

    // Register config tools (manual registration - no register_tool helper)
    let tool = std::sync::Arc::new(GetConfigTool::new(config_manager.clone()));
    let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
    let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());

    let tool = std::sync::Arc::new(SetConfigValueTool::new(config_manager.clone()));
    let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
    let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());

    let server = ConfigServer {
        tool_router,
        prompt_router,
        usage_tracker,
        config_manager,
    };

    let tls_config = tls_cert.zip(tls_key);
    let protocol = if tls_config.is_some() { "https" } else { "http" };
    log::info!("Starting config HTTP server on {protocol}://{}", addr);

    let (completion_tx, completion_rx) = oneshot::channel();
    let ct = CancellationToken::new();

    let session_manager = Arc::new(LocalSessionManager::default());

    let service_factory = {
        let server = server.clone();
        move || Ok::<_, std::io::Error>(server.clone())
    };

    let http_service = StreamableHttpService::new(
        service_factory,
        session_manager,
        StreamableHttpServerConfig {
            stateful_mode: true,
            sse_keep_alive: Some(Duration::from_secs(15)),
        },
    );

    let router = Router::new()
        .fallback_service(http_service)
        .layer(CorsLayer::permissive());

    let axum_handle = axum_server::Handle::new();
    let shutdown_handle = axum_handle.clone();

    let server_task = if let Some((cert_path, key_path)) = tls_config {
        log::info!("Loading TLS certificate from: {cert_path:?}");
        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await?;

        tokio::spawn(async move {
            if let Err(e) = axum_server::bind_rustls(addr, rustls_config)
                .handle(axum_handle)
                .serve(router.into_make_service())
                .await
            {
                log::error!("HTTP server error: {e}");
            }
        })
    } else {
        tokio::spawn(async move {
            if let Err(e) = axum_server::bind(addr)
                .handle(axum_handle)
                .serve(router.into_make_service())
                .await
            {
                log::error!("HTTP server error: {e}");
            }
        })
    };

    let ct_clone = ct.clone();
    tokio::spawn(async move {
        ct_clone.cancelled().await;
        log::debug!("Cancellation token fired");
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(20)));
        let _ = server_task.await;
        let _ = completion_tx.send(());
    });

    // Return ServerHandle for graceful shutdown control
    Ok(kodegen_server_http::ServerHandle::new(ct, completion_rx))
}
