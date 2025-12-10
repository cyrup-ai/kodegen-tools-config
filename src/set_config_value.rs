use crate::ConfigManager;
use kodegen_mcp_schema::{Tool, ToolExecutionContext, ToolResponse};
use kodegen_mcp_schema::McpError;
use kodegen_mcp_schema::config::{SetConfigValueArgs, ConfigSetOutput, SetConfigValuePrompts, CONFIG_SET};

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
    type Prompts = SetConfigValuePrompts;

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

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<ToolResponse<ConfigSetOutput>, McpError> {
        // Set the value
        self.config_manager
            .set_value(&args.key, args.value.clone())
            .await?;

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

        // Human-readable summary
        let summary = format!(
            "\x1b[33mConfig Updated: {}\x1b[0m\n\
             Value: {} · Type: {}",
            args.key,
            value_display,
            type_str
        );

        let message = format!("Config key '{}' set to {}", args.key, value_display);

        Ok(ToolResponse::new(
            summary,
            ConfigSetOutput {
                success: true,
                key: args.key,
                message,
            },
        ))
    }
}
