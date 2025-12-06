use crate::{ConfigManager, get_system_info};
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::McpError;
use kodegen_mcp_schema::config::{GetConfigArgs, ConfigGetOutput, ConfigGetPrompts, CONFIG_GET};

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
    type Prompts = ConfigGetPrompts;

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

    async fn execute(&self, _args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<ConfigGetOutput>, McpError> {
        let mut config = self.config_manager.get_config();

        // Refresh system info with current values
        config.system_info = get_system_info();
        config.save_error_count = ConfigManager::get_save_error_count();

        // Human-readable summary
        let system_info = &config.system_info;
        let summary = format!(
            "\x1b[36m󰒓 Config: Complete Server Configuration\x1b[0m\n\
              Shell: {} · Platform: {} · CPU: {} cores · Memory: {} MB used / {} MB total",
            config.default_shell,
            system_info.platform,
            system_info.cpu_count,
            system_info.memory.used_mb,
            system_info.memory.total_mb
        );

        // Serialize config to JSON value (avoids circular dependency between schema and config-manager)
        let config_json = serde_json::to_value(&config)
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to serialize config: {}", e)))?;

        Ok(ToolResponse::new(
            summary,
            ConfigGetOutput {
                success: true,
                config: config_json,
            },
        ))
    }
}
