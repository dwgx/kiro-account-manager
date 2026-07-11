// Kiro IDE 设置数据模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)] // 设置结构体需要多个布尔字段来表示不同的开关选项
pub struct KiroSettings {
    pub http_proxy: Option<String>,
    pub model_selection: Option<String>,
    pub enable_codebase_indexing: bool,
    pub trusted_commands_mode: Option<String>,
    pub custom_trusted_commands: Option<String>,
    // Agent 设置
    pub agent_autonomy: Option<String>,
    pub enable_tab_autocomplete: bool,
    pub usage_summary: bool,
    pub enable_debug_logs: bool,
    // 通知设置
    pub notify_action_required: bool,
    pub notify_failure: bool,
    pub notify_success: bool,
    pub notify_billing: bool,
    // 新增设置
    pub trusted_tools: Vec<String>,
    pub reference_tracker: bool,
    pub configure_mcp: String, // "Enabled" | "Disabled"
    // 遥测设置
    pub telemetry_content_collection: bool,
    pub telemetry_usage_analytics: bool,
    pub telemetry_edit_stats: bool,
    pub telemetry_feedback: bool,
}

impl Default for KiroSettings {
    fn default() -> Self {
        Self {
            http_proxy: None,
            model_selection: Some("claude-sonnet-4.5".to_string()),
            enable_codebase_indexing: true,
            trusted_commands_mode: Some("none".to_string()),
            custom_trusted_commands: None,
            agent_autonomy: Some("Supervised".to_string()),
            enable_tab_autocomplete: true,
            usage_summary: true,
            enable_debug_logs: false,
            notify_action_required: true,
            notify_failure: true,
            notify_success: true,
            notify_billing: true,
            trusted_tools: vec![],
            reference_tracker: false,
            configure_mcp: "Enabled".to_string(),
            telemetry_content_collection: false,
            telemetry_usage_analytics: false,
            telemetry_edit_stats: false,
            telemetry_feedback: false,
        }
    }
}
