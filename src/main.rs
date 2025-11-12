//! Config Category HTTP Server
//!
//! Serves configuration tools via HTTP/HTTPS transport using kodegen_server_http.

use anyhow::Result;
use kodegen_server_http::{run_http_server, Managers, RouterSet, register_tool};
use rmcp::handler::server::router::{prompt::PromptRouter, tool::ToolRouter};

#[tokio::main]
async fn main() -> Result<()> {
    run_http_server("config", |config, _tracker| {
        let config = config.clone();
        Box::pin(async move {
            let tool_router = ToolRouter::new();
            let prompt_router = PromptRouter::new();
            let managers = Managers::new();

            // Register config tools
            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                kodegen_tools_config::GetConfigTool::new(config.clone()),
            );

            let (tool_router, prompt_router) = register_tool(
                tool_router,
                prompt_router,
                kodegen_tools_config::SetConfigValueTool::new(config.clone()),
            );

            Ok(RouterSet::new(tool_router, prompt_router, managers))
        })
    }).await
}
