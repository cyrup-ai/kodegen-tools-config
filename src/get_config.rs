use crate::{ConfigManager, get_system_info};
use kodegen_mcp_tool::{Tool, ToolExecutionContext};
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::config::{GetConfigArgs, GetConfigPromptArgs, CONFIG_GET};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct GetConfigTool {
    config_manager: ConfigManager,
}

impl GetConfigTool {
    #[must_use]
    pub fn new(config_manager: ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for GetConfigTool {
    type Args = GetConfigArgs;
    type PromptArgs = GetConfigPromptArgs;

    fn name() -> &'static str {
        CONFIG_GET
    }

    fn description() -> &'static str {
        "Get complete server configuration including security settings (blocked commands, \
         allowed directories), shell preferences, resource limits, and live system diagnostics \
         (platform, architecture, OS version, kernel version, hostname, CPU count, memory usage)."
    }

    fn read_only() -> bool {
        true
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![] // No arguments needed
    }

    async fn execute(&self, _args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let mut config = self.config_manager.get_config();

        // Refresh system info with current values
        config.system_info = get_system_info();
        config.save_error_count = ConfigManager::get_save_error_count();

        let mut contents = Vec::new();

        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        let system_info = &config.system_info;
        // Line 1: Cyan header with gear icon
        // Line 2: Compact metadata with info icon
        let summary = format!(
            "\x1b[36m󰒓 Config: Complete Server Configuration\x1b[0m\n\
              Shell: {} · Platform: {} · CPU: {} cores · Memory: {} MB used / {} MB total",
            config.default_shell,
            system_info.platform,
            system_info.cpu_count,
            system_info.memory.used_mb,
            system_info.memory.total_mb
        );
        contents.push(Content::text(summary));

        // ========================================
        // Content[1]: Machine-Parseable JSON
        // ========================================
        let metadata = json!({
            "success": true,
            "config": config
        });
        let json_str = serde_json::to_string_pretty(&metadata)
            .unwrap_or_else(|_| "{}".to_string());
        contents.push(Content::text(json_str));

        Ok(contents)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I check server configuration?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use config_get to retrieve the current server configuration. \
                     This shows blocked commands, allowed directories, shell settings, \
                     and line limits.",
                ),
            },
        ])
    }
}
