use crate::ConfigManager;
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::config::{SetConfigValueArgs, SetConfigValuePromptArgs, CONFIG_SET};
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
        CONFIG_SET
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
        
        // Determine the type string from the ConfigValue variant
        let type_str = match &args.value {
            crate::ConfigValue::String(_) => "string",
            crate::ConfigValue::Number(_) => "number",
            crate::ConfigValue::Boolean(_) => "boolean",
            crate::ConfigValue::Array(_) => "array",
        };

        // Format the 2-line output using ANSI codes
        let summary = format!(
            "\x1b[33m󰒓 Config Updated: {}\x1b[0m\n\
             󰄬 Value: {} · Type: {}",
            args.key,
            value_display,
            type_str
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
