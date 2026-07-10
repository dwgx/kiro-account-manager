use serde::{Deserialize, Serialize};

use super::helpers::{default_budget_tokens, default_stream, deserialize_budget_tokens};

/// Thinking 配置
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Thinking {
    #[serde(rename = "type")]
    pub thinking_type: String,
    #[serde(
        default = "default_budget_tokens",
        deserialize_with = "deserialize_budget_tokens"
    )]
    pub budget_tokens: i32,
}

impl Thinking {
    /// 是否启用了 thinking（enabled 或 adaptive）
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.thinking_type == "enabled" || self.thinking_type == "adaptive"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRequest {
    pub model: String,
    pub messages: Vec<NormalizedMessage>,
    #[serde(default = "default_stream")]
    pub stream: bool,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<serde_json::Value>,
    #[serde(default)]
    pub previous_response_id: Option<String>,
    #[serde(default)]
    pub thinking: Option<Thinking>,
    /// 工具名称反向映射表：sanitized_name -> original_name
    /// 用于在响应中还原工具名称（camelCase -> snake_case）
    #[serde(skip)]
    pub tool_name_map: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMessage {
    pub role: String,
    pub content: Option<serde_json::Value>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}
