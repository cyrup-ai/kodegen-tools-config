//! Config Category HTTP Server
//!
//! Manual implementation - cannot use kodegen_server_http due to circular dependency
//! (mcp-server-http depends on tools-config::ConfigManager)

use anyhow::Result;
use clap::Parser;
use kodegen_mcp_tool::Tool;
use kodegen_utils::usage_tracker::UsageTracker;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::router::{prompt::PromptRouter, tool::ToolRouter},
    model::*,
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpService, StreamableHttpServerConfig,
        session::local::LocalSessionManager,
    },
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use axum::Router;
use tower_http::cors::CorsLayer;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(name = "kodegen-config")]
#[command(about = "Config tools HTTP server")]
struct Args {
    /// HTTP server bind address
    #[arg(long)]
    http: SocketAddr,

    /// TLS certificate path
    #[arg(long, requires = "tls_key")]
    tls_cert: Option<PathBuf>,

    /// TLS private key path
    #[arg(long, requires = "tls_cert")]
    tls_key: Option<PathBuf>,
}

#[derive(Clone)]
struct ConfigServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    usage_tracker: UsageTracker,
    config_manager: kodegen_tools_config::ConfigManager,
}

impl ServerHandler for ConfigServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("KODEGEN Config Category Server".to_string()),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
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
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let items = self.tool_router.list_all();
        Ok(ListToolsResult::with_all_items(items))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
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
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let items = self.prompt_router.list_all();
        Ok(ListPromptsResult::with_all_items(items))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Err(McpError::resource_not_found(
            "resource_not_found",
            Some(serde_json::json!({ "uri": request.uri })),
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        // Store client info (fire-and-forget, errors logged in background task)
        let _ = self.config_manager.set_client_info(request.client_info).await;
        Ok(self.get_info())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args = Args::parse();
    let instance_id = chrono::Utc::now().format("%Y%m%d-%H%M%S-config").to_string();

    let config_manager = kodegen_tools_config::ConfigManager::new();
    config_manager.init().await?;
    let usage_tracker = UsageTracker::new(instance_id.clone());

    kodegen_mcp_tool::tool_history::init_global_history(instance_id).await;

    let tool_router = ToolRouter::new();
    let prompt_router = PromptRouter::new();

    // Register config tools
    let tool = std::sync::Arc::new(kodegen_tools_config::GetConfigTool::new(config_manager.clone()));
    let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
    let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());

    let tool = std::sync::Arc::new(kodegen_tools_config::SetConfigValueTool::new(config_manager.clone()));
    let tool_router = tool_router.with_route(tool.clone().arc_into_tool_route());
    let prompt_router = prompt_router.with_route(tool.arc_into_prompt_route());

    let server = ConfigServer {
        tool_router,
        prompt_router,
        usage_tracker,
        config_manager,
    };

    let tls_config = args.tls_cert.zip(args.tls_key);
    let protocol = if tls_config.is_some() { "https" } else { "http" };
    log::info!("Starting config HTTP server on {protocol}://{}", args.http);

    let (completion_tx, completion_rx) = oneshot::channel();
    let ct = CancellationToken::new();

    // Create session manager for stateful HTTP
    let session_manager = Arc::new(LocalSessionManager::default());

    // Create service factory closure
    let service_factory = {
        let server = server.clone();
        move || Ok::<_, std::io::Error>(server.clone())
    };

    // Create StreamableHttpService
    let http_service = StreamableHttpService::new(
        service_factory,
        session_manager,
        StreamableHttpServerConfig {
            stateful_mode: true,
            sse_keep_alive: Some(Duration::from_secs(15)),
        },
    );

    // Build Axum router with CORS
    let router = Router::new()
        .nest_service("/", http_service)
        .layer(CorsLayer::permissive());

    let axum_handle = axum_server::Handle::new();
    let shutdown_handle = axum_handle.clone();

    let server_task = if let Some((cert_path, key_path)) = tls_config {
        log::info!("Loading TLS certificate from: {cert_path:?}");
        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await?;

        tokio::spawn(async move {
            if let Err(e) = axum_server::bind_rustls(args.http, rustls_config)
                .handle(axum_handle)
                .serve(router.into_make_service())
                .await
            {
                log::error!("HTTP server error: {e}");
            }
        })
    } else {
        tokio::spawn(async move {
            if let Err(e) = axum_server::bind(args.http)
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

    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let ctrl_c = tokio::signal::ctrl_c();
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sighup = signal(SignalKind::hangup())?;

        tokio::select! {
            _ = ctrl_c => log::debug!("Received SIGINT"),
            _ = sigterm.recv() => log::debug!("Received SIGTERM"),
            _ = sighup.recv() => log::debug!("Received SIGHUP"),
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
    }

    log::info!("Shutdown signal received");
    ct.cancel();

    match tokio::time::timeout(std::time::Duration::from_secs(30), completion_rx).await {
        Ok(Ok(())) => log::info!("Server shutdown completed"),
        Ok(Err(_)) => log::warn!("Completion channel closed"),
        Err(_) => log::warn!("Shutdown timeout"),
    }

    log::info!("Config server stopped");
    Ok(())
}
