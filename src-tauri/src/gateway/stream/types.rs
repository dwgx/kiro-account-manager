#[derive(Debug, Clone, PartialEq)]
pub enum KiroEvent {
    Text(String),
    Thinking(String),
    ThinkingSignature(String),
    ToolUseStart {
        id: String,
        name: String,
    },
    ToolUseInputDelta {
        id: String,
        name: Option<String>,
        input_delta: String,
    },
    ToolUseStop {
        id: String,
    },
    Usage {
        input_tokens: i32,
        output_tokens: i32,
        cache_read_input_tokens: Option<i32>,
        cache_creation_input_tokens: Option<i32>,
    },
    ContextUsage {
        percentage: f32,
    },
    Metering {
        unit: String,
        unit_plural: String,
        usage: f64,
    },
    Citation {
        text: Option<String>,
        link: String,
        target: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedCitation {
    pub text: Option<String>,
    pub link: String,
    pub target: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AggregatedKiroResponse {
    pub text: String,
    pub thinking: String,
    pub thinking_signature: Option<String>,
    pub tool_calls: Vec<(String, String, String)>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_input_tokens: Option<i32>,
    pub cache_creation_input_tokens: Option<i32>,
    pub context_usage_percentage: Option<f32>,
    pub metering_usage: Option<f64>,
    pub citations: Vec<AggregatedCitation>,
}
