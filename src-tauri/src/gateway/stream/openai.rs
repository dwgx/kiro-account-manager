use super::types::AggregatedKiroResponse;
use crate::gateway::models::{
    OpenAIChatChoice, OpenAIChatChunk, OpenAIChatChunkChoice, OpenAIChatDelta, OpenAIChatResponse,
    OpenAIChatResponseMessage, OpenAIChatUsage, OpenAIResponseToolCall, OpenAIToolCallFunction,
    OpenAIUsage,
};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub fn build_openai_chunk(
    completion_id: &str,
    created: i64,
    model: &str,
    delta: OpenAIChatDelta,
    finish_reason: Option<String>,
    usage: Option<OpenAIChatUsage>,
) -> OpenAIChatChunk {
    OpenAIChatChunk {
        id: completion_id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created,
        model: model.to_string(),
        choices: vec![OpenAIChatChunkChoice {
            index: 0,
            delta,
            finish_reason,
        }],
        usage,
    }
}

pub fn build_openai_response(
    model: &str,
    aggregated: &AggregatedKiroResponse,
) -> OpenAIChatResponse {
    let completion_id = format!("chatcmpl-{}", Uuid::new_v4().simple());
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut message = OpenAIChatResponseMessage {
        role: "assistant".to_string(),
        content: if aggregated.text.is_empty() {
            None
        } else {
            Some(aggregated.text.clone())
        },
        tool_calls: None,
    };

    let finish_reason = if !aggregated.tool_calls.is_empty() {
        let tool_calls: Vec<OpenAIResponseToolCall> = aggregated
            .tool_calls
            .iter()
            .map(|(id, name, arguments)| OpenAIResponseToolCall {
                id: id.clone(),
                call_type: "function".to_string(),
                function: OpenAIToolCallFunction {
                    name: name.clone(),
                    arguments: arguments.clone(),
                },
            })
            .collect();
        message.tool_calls = Some(tool_calls);
        "tool_calls".to_string()
    } else {
        "stop".to_string()
    };

    OpenAIChatResponse {
        id: completion_id,
        object: "chat.completion".to_string(),
        created,
        model: model.to_string(),
        choices: vec![OpenAIChatChoice {
            index: 0,
            message,
            finish_reason: Some(finish_reason),
        }],
        usage: OpenAIUsage {
            input_tokens: aggregated.input_tokens,
            output_tokens: aggregated.output_tokens,
        },
    }
}
