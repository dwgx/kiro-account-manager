use super::event_parser::parse_kiro_event_full;
use super::types::{AggregatedCitation, AggregatedKiroResponse, KiroEvent};

pub fn deduplicate_tool_calls(
    tool_calls: Vec<(String, String, String)>,
) -> Vec<(String, String, String)> {
    use std::collections::HashMap;

    if tool_calls.is_empty() {
        return tool_calls;
    }

    let mut by_id: HashMap<String, (String, String, String)> = HashMap::new();
    let mut order = Vec::new();

    for (id, name, args) in tool_calls {
        if !by_id.contains_key(&id) {
            order.push(id.clone());
        }
        by_id.insert(id.clone(), (id, name, args));
    }

    order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect()
}

/// 从已切分好的 JSON payload 数组直接聚合（每帧直接解析，无需 extract_json 重新切分）
pub fn aggregate_kiro_response_from_payloads(payloads: &[String]) -> AggregatedKiroResponse {
    let mut aggregated = AggregatedKiroResponse::default();
    let mut tool_accumulators: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    let mut found_usage = false;
    let mut json_count = 0;
    let mut event_counts = std::collections::HashMap::new();

    log::info!("[聚合] 开始聚合，共 {} 个 payload", payloads.len());

    for json_str in payloads {
        let json_str = json_str.trim();
        if json_str.is_empty() {
            continue;
        }
        json_count += 1;

        log::debug!(
            "[聚合] JSON #{}: bytes={}, chars={}",
            json_count,
            json_str.len(),
            json_str.chars().count()
        );

        if let Some(event) = parse_kiro_event_full(json_str) {
            let event_type = match &event {
                KiroEvent::Text(_) => "Text",
                KiroEvent::Thinking(_) => "Thinking",
                KiroEvent::ThinkingSignature(_) => "ThinkingSignature",
                KiroEvent::ToolUseStart { .. } => "ToolUseStart",
                KiroEvent::ToolUseInputDelta { .. } => "ToolUseInputDelta",
                KiroEvent::ToolUseStop { .. } => "ToolUseStop",
                KiroEvent::Usage { .. } => "Usage",
                KiroEvent::ContextUsage { .. } => "ContextUsage",
                KiroEvent::Metering { .. } => "Metering",
                KiroEvent::Citation { .. } => "Citation",
            };
            *event_counts.entry(event_type).or_insert(0) += 1;

            match event {
                KiroEvent::Text(text) => {
                    log::debug!("[聚合] 文本事件: {} 字符", text.len());
                    aggregated.text.push_str(&text);
                }
                KiroEvent::Thinking(text) => {
                    log::debug!("[聚合] 思考事件: {} 字符", text.len());
                    aggregated.thinking.push_str(&text);
                }
                KiroEvent::ThinkingSignature(sig) => {
                    log::debug!("[聚合] 思考签名: {} 字符", sig.len());
                    aggregated.thinking_signature = Some(sig);
                }
                KiroEvent::ToolUseStart { id, name } => {
                    log::debug!("[聚合] 工具使用开始: id={}, name={}", id, name);
                    let entry = tool_accumulators
                        .entry(id)
                        .or_insert((String::new(), String::new()));
                    if entry.0.is_empty() {
                        entry.0 = name;
                    }
                }
                KiroEvent::ToolUseInputDelta {
                    id,
                    name,
                    input_delta,
                } => {
                    log::debug!(
                        "[聚合] 工具输入增量: id={}, delta_len={}",
                        id,
                        input_delta.len()
                    );
                    if let Some((existing_name, current_input)) = tool_accumulators.get_mut(&id) {
                        if existing_name.is_empty() {
                            if let Some(name) = name {
                                *existing_name = name;
                            }
                        }
                        if input_delta.trim_start().starts_with('{')
                            && current_input.trim_start().starts_with('{')
                        {
                            log::debug!("[聚合] 检测到完整 JSON input，替换而不是追加");
                            *current_input = input_delta;
                        } else {
                            current_input.push_str(&input_delta);
                        }
                    } else {
                        tool_accumulators.insert(id, (name.unwrap_or_default(), input_delta));
                    }
                }
                KiroEvent::ToolUseStop { id } => {
                    log::debug!("[聚合] 工具使用结束: id={}", id);
                    if let Some((name, input)) = tool_accumulators.remove(&id) {
                        log::debug!(
                            "[聚合] 完整的工具调用: id={}, name={}, input_len={}",
                            id,
                            name,
                            input.len()
                        );
                        aggregated.tool_calls.push((id, name, input));
                    } else {
                        log::warn!("[聚合] ⚠️  工具使用结束但未找到对应的累加器: id={}", id);
                    }
                }
                KiroEvent::Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_input_tokens,
                    cache_creation_input_tokens,
                } => {
                    found_usage = true;
                    aggregated.input_tokens = input_tokens;
                    aggregated.output_tokens = output_tokens;
                    aggregated.cache_read_input_tokens = cache_read_input_tokens;
                    aggregated.cache_creation_input_tokens = cache_creation_input_tokens;
                    log::info!(
                        "[聚合] ✅ 发现 usage 信息: input={}, output={}, cache_read={:?}, cache_creation={:?}",
                        input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens
                    );
                }
                KiroEvent::ContextUsage { percentage } => {
                    log::debug!("[聚合] 上下文使用: {}%", percentage);
                    aggregated.context_usage_percentage = Some(percentage);
                }
                KiroEvent::Metering { usage, .. } => {
                    log::debug!("[聚合] 计量: {}", usage);
                    aggregated.metering_usage = Some(usage);
                }
                KiroEvent::Citation { text, link, target } => {
                    log::debug!("[聚合] 引用: link={}", link);
                    aggregated
                        .citations
                        .push(AggregatedCitation { text, link, target });
                }
            }
        } else {
            log::warn!(
                "[聚合] 解析事件失败: json_index={}, bytes={}, chars={}",
                json_count,
                json_str.len(),
                json_str.chars().count()
            );
        }
    }

    // 收集未关闭的 tool_accumulators
    for (id, (name, input)) in tool_accumulators {
        if !name.is_empty() || !input.is_empty() {
            log::warn!("[聚合] ⚠️ 未关闭的工具调用: id={}, name={}", id, name);
            aggregated.tool_calls.push((id, name, input));
        }
    }

    aggregated.tool_calls = deduplicate_tool_calls(aggregated.tool_calls);

    log::info!("[聚合] 完成: 处理了 {} 个 JSON 对象", json_count);
    log::info!("[聚合] 事件计数: {:?}", event_counts);
    log::info!(
        "[聚合] 结果: 文本={} 字符, 思考={} 字符, 工具调用={}, 引用={}",
        aggregated.text.len(),
        aggregated.thinking.len(),
        aggregated.tool_calls.len(),
        aggregated.citations.len()
    );

    if !found_usage {
        log::warn!("[聚合] ⚠️ Kiro 响应中未找到 usage 信息!");
    }

    aggregated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_kiro_response_from_payloads_keeps_name_when_input_arrives_first() {
        let payloads = vec![
            r#"{"toolUseEvent":{"toolUseId":"tool_1","name":"server_health","input":"{}"}}"#
                .to_string(),
            r#"{"toolUseEvent":{"toolUseId":"tool_1","stop":true}}"#.to_string(),
        ];

        let aggregated = aggregate_kiro_response_from_payloads(&payloads);

        assert_eq!(
            aggregated.tool_calls,
            vec![(
                "tool_1".to_string(),
                "server_health".to_string(),
                "{}".to_string()
            )]
        );
    }

    #[test]
    fn aggregate_kiro_response_from_payloads_concatenates_string_input_deltas() {
        let payloads = vec![
            r#"{"toolUseEvent":{"toolUseId":"tool_1","name":"todo_write","input":"{\"todos\":[{\"content\":\""}}"#.to_string(),
            r#"{"toolUseEvent":{"toolUseId":"tool_1","input":"plan"}}"#.to_string(),
            r#"{"toolUseEvent":{"toolUseId":"tool_1","input":"a\",\"status\":\"pending\"}]}"}}"#.to_string(),
            r#"{"toolUseEvent":{"toolUseId":"tool_1","stop":true}}"#.to_string(),
        ];

        let aggregated = aggregate_kiro_response_from_payloads(&payloads);

        assert_eq!(aggregated.tool_calls.len(), 1);
        assert_eq!(aggregated.tool_calls[0].0, "tool_1");
        assert_eq!(aggregated.tool_calls[0].1, "todo_write");
        assert_eq!(
            aggregated.tool_calls[0].2,
            r#"{"todos":[{"content":"plana","status":"pending"}]}"#
        );
    }

    #[test]
    fn aggregate_kiro_response_from_payloads_collects_citations() {
        let payloads = vec![
            r#"{"assistantResponseEvent":{"content":"Hello Rust"}}"#.to_string(),
            r#"{"target":{"range":{"start":6,"end":10}},"citationText":"Rust","citationLink":"https://example.com/rust"}"#.to_string(),
        ];

        let aggregated = aggregate_kiro_response_from_payloads(&payloads);

        assert_eq!(aggregated.text, "Hello Rust");
        assert_eq!(
            aggregated.citations,
            vec![AggregatedCitation {
                text: Some("Rust".to_string()),
                link: "https://example.com/rust".to_string(),
                target: serde_json::json!({ "range": { "start": 6, "end": 10 } }),
            }]
        );
    }

    #[test]
    fn deduplicate_tool_calls_keeps_latest_args_per_id() {
        let input = vec![
            ("tool_1".to_string(), "search".to_string(), "{}".to_string()),
            (
                "tool_1".to_string(),
                "search".to_string(),
                "{\"q\":\"gateway\"}".to_string(),
            ),
            (
                "tool_2".to_string(),
                "open".to_string(),
                "{\"path\":\"README.md\"}".to_string(),
            ),
        ];

        let deduped = deduplicate_tool_calls(input);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].0, "tool_1");
        assert_eq!(deduped[0].2, "{\"q\":\"gateway\"}");
    }
}

