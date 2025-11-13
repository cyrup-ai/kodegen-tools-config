use crate::ConfigManager;
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::config::{SetConfigValueArgs, SetConfigValuePromptArgs};
use rmcp::model::{Content, PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::json;

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct SetConfigValueTool {
    config_manager: ConfigManager,
}

impl SetConfigValueTool {
    #[must_use]
    pub fn new(config_manager: ConfigManager) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for SetConfigValueTool {
    type Args = SetConfigValueArgs;
    type PromptArgs = SetConfigValuePromptArgs;

    fn name() -> &'static str {
        "config_set"
    }

    fn description() -> &'static str {
        "Set a specific configuration value by key.\n\n\
         WARNING: Should be used in a separate chat from file operations and \n\
         command execution to prevent security issues.\n\n\
         Config keys include:\n\
         - blocked_commands (array)\n\
         - default_shell (string)\n\
         - allowed_directories (array of paths)\n\
         - file_read_line_limit (number, max lines for fs_read_file)\n\
         - file_write_line_limit (number, max lines per fs_write_file call)\n\n\
         IMPORTANT: Setting allowed_directories to an empty array ([]) allows full access \n\
         to the entire file system."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![] // No prompt arguments needed
    }

    async fn execute(&self, args: Self::Args) -> Result<Vec<Content>, McpError> {
        // Set the value
        self.config_manager
            .set_value(&args.key, args.value.clone())
            .await?;
        
        // Get updated config
        let updated_config = self.config_manager.get_config();
        
        let mut contents = Vec::new();
        
        // ========================================
        // Content[0]: Human-Readable Summary
        // ========================================
        
        // Format the value for display
        let value_display = match &args.value {
            crate::ConfigValue::String(s) => format!("\"{}\"", s),
            crate::ConfigValue::Number(n) => n.to_string(),
            crate::ConfigValue::Boolean(b) => b.to_string(),
            crate::ConfigValue::Array(arr) => {
                if arr.is_empty() {
                    "[] (empty)".to_string()
                } else if arr.len() <= 3 {
                    format!("[{}]", arr.join(", "))
                } else {
                    format!("[{}, ... {} total]", arr[0], arr.len())
                }
            }
        };
        
        // Contextual messages based on what changed
        let context_info = match args.key.as_str() {
            "blocked_commands" => "Commands in this list will be rejected by the terminal tool.",
            "allowed_directories" => "Only paths within these directories can be accessed (empty = unrestricted).",
            "default_shell" => "This shell will be used for all command executions.",
            "file_read_line_limit" => "Maximum lines that can be read from a file in a single operation.",
            "file_write_line_limit" => "Maximum lines that can be written to a file in a single operation.",
            _ => "Configuration value updated successfully."
        };
        
        let summary = format!(
            "✅ Configuration Updated\n\
             \n\
             Setting: {}\n\
             New value: {}\n\
             \n\
             {}\n\
             \n\
             To view full configuration, use config_get.",
            args.key,
            value_display,
            context_info
        );
        contents.push(Content::text(summary));
        
        // ========================================
        // Content[1]: Machine-Parseable JSON
        // ========================================
        let metadata = json!({
            "success": true,
            "key": args.key,
            "value": args.value,
            "updated_config": updated_config
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
                content: PromptMessageContent::text("How do I update server configuration?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use config_set to update configuration. Examples:\n\n\
                     Block additional commands:\n\
                     {\"key\": \"blocked_commands\", \"value\": [\"rm\", \"sudo\", \"wget\"]}\n\n\
                     Change shell:\n\
                     {\"key\": \"default_shell\", \"value\": \"/bin/bash\"}\n\n\
                     Restrict directories:\n\
                     {\"key\": \"allowed_directories\", \"value\": [\"/home/user/projects\"]}\n\n\
                     Adjust line limits:\n\
                     {\"key\": \"file_read_line_limit\", \"value\": 2000}",
                ),
            },
        ])
    }
}
