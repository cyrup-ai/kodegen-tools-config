use crate::{ConfigManager, get_system_info};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::config::{GetConfigArgs, GetConfigPromptArgs};
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
        "config_get"
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

    async fn execute(&self, _args: Self::Args) -> Result<Vec<Content>, McpError> {
        let mut config = self.config_manager.get_config();
        
        // Refresh system info with current values
        config.system_info = get_system_info();
        config.save_error_count = ConfigManager::get_save_error_count();
        
        let mut contents = Vec::new();
        
        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        let system_info = &config.system_info;
        let summary = format!(
            "⚙️  Server Configuration\n\
             \n\
             Security:\n\
             • Blocked commands: {}\n\
             • Allowed directories: {}\n\
             \n\
             Shell:\n\
             • Default: {}\n\
             \n\
             Limits:\n\
             • Read limit: {} lines\n\
             • Write limit: {} lines\n\
             \n\
             System:\n\
             • Platform: {} ({})\n\
             • OS: {}\n\
             • Kernel: {}\n\
             • CPU cores: {}\n\
             • Memory: {} used, {} available of {} total",
            if config.blocked_commands.is_empty() {
                "none".to_string()
            } else {
                config.blocked_commands.join(", ")
            },
            if config.allowed_directories.is_empty() {
                "all (unrestricted)".to_string()
            } else {
                format!("{} paths", config.allowed_directories.len())
            },
            config.default_shell,
            config.file_read_line_limit,
            config.file_write_line_limit,
            system_info.platform,
            system_info.arch,
            system_info.os_version,
            system_info.kernel_version,
            system_info.cpu_count,
            system_info.memory.used_mb,
            system_info.memory.available_mb,
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
