use super::citation::{
    parse_citation_event, parse_code_reference_event, parse_supplementary_web_links_event,
};
use super::types::KiroEvent;

fn json_value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

pub fn parse_kiro_event_full(json_str: &str) -> Option<KiroEvent> {
    let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

    // 添加调试：只记录真正的 token/usage 事件（meteringEvent、contextUsageEvent、metadataEvent）
    let json_lower = json_str.to_lowercase();
    let is_metering_or_usage_event = json_lower.contains("\"meteringevent\"")
        || json_lower.contains("\"contextusageevent\"")
        || json_lower.contains("\"metadataevent\"")
        || (json_lower.contains("\"usage\"") && json_lower.contains("\"inputtokens\""));

    if is_metering_or_usage_event {
        log::debug!(
            "[Token 解析] 发现 token/usage 事件: bytes={}, chars={}, top_keys={:?}",
            json_str.len(),
            json_str.chars().count(),
            value
                .as_object()
                .map(|object| object.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        );
    }

    // 解析 metadataEvent 中的 tokenUsage
    // Kiro IDE 的 token 信息在 metadataEvent.tokenUsage 中
    if let Some(metadata_event) = value.get("metadataEvent").and_then(|item| item.as_object()) {
        log::debug!(
            "[Token 解析] 发现 metadataEvent: {:?}",
            metadata_event.keys().collect::<Vec<_>>()
        );

        if let Some(token_usage) = metadata_event
            .get("tokenUsage")
            .and_then(|item| item.as_object())
        {
            // Kiro IDE 的字段名：
            // - uncachedInputTokens (未缓存的输入 tokens)
            // - cacheReadInputTokens (缓存读取 tokens)
            // - cacheWriteInputTokens (缓存写入 tokens)
            // - outputTokens (输出 tokens)
            // - totalTokens (总 tokens)
            let uncached_input_tokens = token_usage
                .get("uncachedInputTokens")
                .and_then(|item| item.as_i64())
                .unwrap_or(0) as i32;
            let cache_read_input_tokens = token_usage
                .get("cacheReadInputTokens")
                .and_then(|item| item.as_i64())
                .map(|v| v as i32);
            let cache_creation_input_tokens = token_usage
                .get("cacheWriteInputTokens")
                .and_then(|item| item.as_i64())
                .map(|v| v as i32);
            let output_tokens = token_usage
                .get("outputTokens")
                .and_then(|item| item.as_i64())
                .unwrap_or(0) as i32;

            // input_tokens 仅为未缓存的输入（符合 Anthropic 官方规范）
            // 客户端会单独看到 cache_read_input_tokens 和 cache_creation_input_tokens 字段
            // 三者相加才是真实总输入量。如果这里包含 cache_*，客户端账单会双重计费虚高
            // 参考：chaogei/Kiro-account-manager v1.6.6 修复
            let input_tokens = uncached_input_tokens;

            log::info!(
                "[Token 解析] ✅ 发现 metadataEvent.tokenUsage: keys={:?}, 未缓存输入={}, 缓存读取={:?}, 缓存写入={:?}, 输出={}",
                token_usage.keys().cloned().collect::<Vec<_>>(),
                uncached_input_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                output_tokens
            );
            log::info!(
                "[Token 解析] 已解析: 未缓存输入={}, 缓存读取={:?}, 缓存写入={:?}, 输出={}, 总输入={}",
                uncached_input_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                output_tokens,
                input_tokens
            );

            if input_tokens > 0 || output_tokens > 0 {
                return Some(KiroEvent::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_input_tokens,
                    cache_creation_input_tokens,
                });
            } else {
                log::warn!("[Token 解析] ⚠️ 发现 tokenUsage 但所有 token 都为 0");
            }
        } else {
            log::warn!("[Token 解析] ⚠️ 发现 metadataEvent 但没有 tokenUsage 字段");
        }
    }

    // 兼容旧格式：解析顶层的 usage 字段（用于其他格式的响应）
    if let Some(usage) = value.get("usage").and_then(|item| item.as_object()) {
        let input_tokens = usage
            .get("inputTokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|item| item.as_i64())
            .unwrap_or(0) as i32;
        let output_tokens = usage
            .get("outputTokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|item| item.as_i64())
            .unwrap_or(0) as i32;

        let cache_read_input_tokens = usage
            .get("cachedReadTokens")
            .or_else(|| usage.get("cacheReadInputTokens"))
            .or_else(|| usage.get("cache_read_input_tokens"))
            .and_then(|item| item.as_i64())
            .map(|v| v as i32);
        let cache_creation_input_tokens = usage
            .get("cachedWriteTokens")
            .or_else(|| usage.get("cacheCreationInputTokens"))
            .or_else(|| usage.get("cache_creation_input_tokens"))
            .and_then(|item| item.as_i64())
            .map(|v| v as i32);

        log::info!(
            "[Token 解析] 发现旧格式 usage: keys={:?}, input={}, output={}, cache_read={:?}, cache_creation={:?}",
            usage.keys().cloned().collect::<Vec<_>>(),
            input_tokens,
            output_tokens,
            cache_read_input_tokens,
            cache_creation_input_tokens
        );

        if input_tokens > 0
            || output_tokens > 0
            || cache_read_input_tokens.is_some()
            || cache_creation_input_tokens.is_some()
        {
            return Some(KiroEvent::Usage {
                input_tokens,
                output_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
            });
        }
    }

    // 解析 contextUsageEvent
    if let Some(context_event) = value
        .get("contextUsageEvent")
        .and_then(|item| item.as_object())
    {
        if let Some(percentage) = context_event
            .get("contextUsagePercentage")
            .and_then(|item| item.as_f64())
        {
            return Some(KiroEvent::ContextUsage {
                percentage: percentage as f32,
            });
        }
    }

    if let Some(metering) = value.get("meteringEvent").and_then(|item| item.as_object()) {
        let unit = metering
            .get("unit")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let unit_plural = metering
            .get("unitPlural")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let usage = metering
            .get("usage")
            .and_then(|item| item.as_f64())
            .unwrap_or(0.0);

        if !unit.is_empty() {
            return Some(KiroEvent::Metering {
                unit,
                unit_plural,
                usage,
            });
        }
    }
    if let Some(text) = parse_reasoning_text(&value) {
        if !text.is_empty() {
            return Some(KiroEvent::Thinking(text));
        }
    }
    // 提取 reasoning signature（可能在单独的事件中，也可能和 text 一起）
    if let Some(sig) = parse_reasoning_signature(&value) {
        if !sig.is_empty() {
            return Some(KiroEvent::ThinkingSignature(sig));
        }
    }

    // 解析 codeReferenceEvent
    if let Some(code_ref) = value.get("codeReferenceEvent") {
        if let Some(citation) = parse_code_reference_event(code_ref) {
            return Some(KiroEvent::Citation {
                text: citation.text,
                link: citation.link,
                target: citation.target,
            });
        }
    }

    // 解析 supplementaryWebLinksEvent
    if let Some(web_links) = value.get("supplementaryWebLinksEvent") {
        if let Some(citation) = parse_supplementary_web_links_event(web_links) {
            return Some(KiroEvent::Citation {
                text: citation.text,
                link: citation.link,
                target: citation.target,
            });
        }
    }

    if let Some(citation) = parse_citation_event(&value) {
        return Some(KiroEvent::Citation {
            text: citation.text,
            link: citation.link,
            target: citation.target,
        });
    }

    if let Some(tool_use_id) = value
        .get("toolUseId")
        .and_then(|item| item.as_str())
        .or_else(|| {
            value
                .get("toolUseEvent")
                .and_then(|e| e.get("toolUseId"))
                .and_then(|item| item.as_str())
        })
    {
        // 支持 toolUseEvent 包装格式
        let tool_data = value.get("toolUseEvent").unwrap_or(&value);

        let name = tool_data
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();

        if tool_data.get("stop").and_then(|item| item.as_bool()) == Some(true) {
            return Some(KiroEvent::ToolUseStop {
                id: tool_use_id.to_string(),
            });
        }

        if let Some(input) = tool_data.get("input") {
            let input_delta = if let Some(text) = input.as_str() {
                log::trace!(
                    "[Tool Use] 收到 input 字符串分片: id={tool_use_id}, byte_len={}, char_len={}",
                    text.len(),
                    text.chars().count()
                );
                text.to_string()
            } else if input.is_object() || input.is_array() {
                let serialized = serde_json::to_string(input).unwrap_or_default();
                log::trace!(
                    "[Tool Use] 收到 input 对象/数组分片: id={tool_use_id}, value_type={}, byte_len={}, char_len={}",
                    json_value_kind(input),
                    serialized.len(),
                    serialized.chars().count()
                );
                serialized
            } else {
                log::warn!(
                    "[Tool Use] input 类型未知: value_type={}",
                    json_value_kind(input)
                );
                String::new()
            };
            if !input_delta.is_empty() {
                log::trace!(
                    "[Tool Use] 解析 ToolUseInputDelta: id={tool_use_id}, byte_len={}",
                    input_delta.len()
                );
                return Some(KiroEvent::ToolUseInputDelta {
                    id: tool_use_id.to_string(),
                    name: if name.is_empty() { None } else { Some(name) },
                    input_delta,
                });
            }
        }

        if !name.is_empty() {
            log::debug!(
                "[Tool Use] 发送 ToolUseStart: id={}, name={}",
                tool_use_id,
                name
            );
            return Some(KiroEvent::ToolUseStart {
                id: tool_use_id.to_string(),
                name,
            });
        }
    }

    if let Some(tool) = value
        .get("assistantResponseEvent")
        .and_then(|item| item.get("toolUses"))
        .and_then(|item| item.as_array())
        .and_then(|items| items.first())
    {
        let id = tool
            .get("toolUseId")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();
        let name = tool
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .to_string();

        if !name.is_empty() {
            return Some(KiroEvent::ToolUseStart { id, name });
        }
    }

    parse_text_content(&value).map(KiroEvent::Text)
}

fn parse_text_content(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.get("content").and_then(|item| item.as_str()) {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("delta")
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("contentBlockDelta")
        .and_then(|item| item.get("delta"))
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    if let Some(text) = value
        .get("assistantResponseEvent")
        .and_then(|item| item.get("content"))
        .and_then(|item| item.as_str())
    {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    None
}

fn parse_reasoning_text(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value
        .get("reasoningContentEvent")
        .and_then(|item| item.get("text"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("delta")
        .and_then(|item| item.get("thinking"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    if let Some(text) = value
        .get("contentBlockDelta")
        .and_then(|item| item.get("delta"))
        .and_then(|item| item.get("thinking"))
        .and_then(|item| item.as_str())
    {
        return Some(text.to_string());
    }

    None
}

/// 从 reasoningContentEvent 中提取 signature
fn parse_reasoning_signature(value: &serde_json::Value) -> Option<String> {
    value
        .get("reasoningContentEvent")
        .and_then(|item| item.get("signature"))
        .and_then(|item| item.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kiro_event_full_reads_text_tool_and_usage_events() {
        assert_eq!(
            parse_kiro_event_full(r#"{"assistantResponseEvent":{"content":"hello"}}"#),
            Some(KiroEvent::Text("hello".to_string()))
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"toolUseId":"tool_1","name":"search_docs"}"#),
            Some(KiroEvent::ToolUseStart {
                id: "tool_1".to_string(),
                name: "search_docs".to_string(),
            })
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"toolUseId":"tool_1","input":{"q":"gateway"}}"#),
            Some(KiroEvent::ToolUseInputDelta {
                id: "tool_1".to_string(),
                name: None,
                input_delta: "{\"q\":\"gateway\"}".to_string(),
            })
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"toolUseId":"tool_1","stop":true}"#),
            Some(KiroEvent::ToolUseStop {
                id: "tool_1".to_string(),
            })
        );
        assert_eq!(
            parse_kiro_event_full(r#"{"usage":{"inputTokens":12,"outputTokens":34}}"#),
            Some(KiroEvent::Usage {
                input_tokens: 12,
                output_tokens: 34,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            })
        );
    }

    #[test]
    fn parse_kiro_event_full_reads_reasoning_content() {
        assert_eq!(
            parse_kiro_event_full(r#"{"reasoningContentEvent":{"text":"分析中"}}"#),
            Some(KiroEvent::Thinking("分析中".to_string()))
        );
    }

    #[test]
    fn parse_kiro_event_full_reads_metering_event() {
        assert_eq!(
            parse_kiro_event_full(
                r#"{"meteringEvent":{"unit":"credit","unitPlural":"credits","usage":0.3876425741791045}}"#
            ),
            Some(KiroEvent::Metering {
                unit: "credit".to_string(),
                unit_plural: "credits".to_string(),
                usage: 0.3876425741791045,
            })
        );
    }

    #[test]
    fn parse_kiro_event_full_keeps_tool_name_with_wrapped_input() {
        assert_eq!(
            parse_kiro_event_full(
                r#"{"toolUseEvent":{"toolUseId":"tool_1","name":"server_health","input":"{}"}}"#
            ),
            Some(KiroEvent::ToolUseInputDelta {
                id: "tool_1".to_string(),
                name: Some("server_health".to_string()),
                input_delta: "{}".to_string(),
            })
        );
    }

    #[test]
    fn parse_kiro_event_full_reads_citation_events() {
        assert_eq!(
            parse_kiro_event_full(
                r#"{"target":{"range":{"start":2,"end":5}},"citationText":"Rust","citationLink":"https://example.com/rust"}"#
            ),
            Some(KiroEvent::Citation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: serde_json::json!({ "range": { "start": 2, "end": 5 } }),
            })
        );
        assert_eq!(
            parse_kiro_event_full(
                r#"{"target":{"location":6},"citationText":"Rust","citationLink":"https://example.com/location"}"#
            ),
            Some(KiroEvent::Citation {
                text: Some("Rust".to_string()),
                link: "https://example.com/location".to_string(),
                target: serde_json::json!({ "location": 6 }),
            })
        );
    }
}

