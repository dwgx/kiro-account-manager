use crate::gateway::models::{KiroInputSchema, KiroTool, KiroToolSpec, Tool, ToolFunction};
use serde_json::{json, Map, Value};

pub const TOOL_DESCRIPTION_MAX_LENGTH: usize = 10237;

pub fn convert_openai_chat_tools(
    tools: Option<&Value>,
) -> (Option<Vec<Tool>>, std::collections::HashMap<String, String>) {
    convert_responses_tools(tools)
}

/// 规范化工具名称为 Kiro API 接受的格式
///
/// Kiro API 要求工具名称必须是纯 camelCase 格式（不能包含下划线或横杠）
/// 将分隔符（_、-、多下划线命名空间前缀）转换为 camelCase 边界
pub(super) fn sanitize_tool_name(name: &str) -> String {
    // 按下划线和横杠分割
    let parts: Vec<&str> = name
        .split(|c| c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return "tool".to_string();
    }

    // 构建 camelCase：第一部分小写开头，其余部分首字母大写
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // 第一部分：首字母小写
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push_str(&first.to_lowercase().to_string());
                result.push_str(chars.as_str());
            }
        } else {
            // 其余部分：首字母大写
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push_str(&first.to_uppercase().to_string());
                result.push_str(chars.as_str());
            }
        }
    }

    if result.is_empty() {
        "tool".to_string()
    } else {
        result
    }
}

/// 缩短工具名称以符合 Kiro API 的 64 字符限制
///
/// 对于 MCP 工具（格式：mcp__server__tool），尝试缩短为 mcp__tool
/// 其他工具直接截断到 64 字符
pub(super) fn shorten_tool_name(name: &str) -> String {
    if name.len() <= 64 {
        return name.to_string();
    }

    // MCP 工具：mcp__server__tool -> mcp__tool
    if name.starts_with("mcp__") {
        if let Some(last_idx) = name.rfind("__") {
            if last_idx > 5 {
                let shortened = format!("mcp__{}", &name[last_idx + 2..]);
                if shortened.len() <= 64 {
                    return shortened;
                }
            }
        }
    }

    // 直接截断到 64 字符
    name.chars().take(64).collect()
}

/// 转换 Anthropic 工具定义，返回 (Tool, Option<(sanitized_name, original_name)>)
pub(super) fn convert_anthropic_tool(
    tool: &crate::gateway::models::AnthropicTool,
) -> (Tool, Option<(String, String)>) {
    let sanitized = shorten_tool_name(&sanitize_tool_name(&tool.name));

    // 截断超长描述（和 Kiro-Go 保持一致）
    let description = tool.description.as_ref().map(|desc| {
        if desc.len() > TOOL_DESCRIPTION_MAX_LENGTH {
            // 按字节上限截断，但回退到最近的字符边界，避免在多字节字符（中文/emoji）中间切断导致 panic
            let mut end = TOOL_DESCRIPTION_MAX_LENGTH;
            while end > 0 && !desc.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &desc[..end])
        } else {
            desc.clone()
        }
    });

    let converted_tool = Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: sanitized.clone(),
            description,
            parameters: Some(normalize_json_schema(tool.input_schema.clone())),
        },
        cache_control: tool.cache_control.clone(),
    };

    // 如果工具名被修改，记录映射关系
    let mapping = if sanitized != tool.name {
        Some((sanitized, tool.name.clone()))
    } else {
        None
    };

    (converted_tool, mapping)
}

pub(super) fn convert_responses_tools(
    tools: Option<&Value>,
) -> (Option<Vec<Tool>>, std::collections::HashMap<String, String>) {
    let mut tool_name_map = std::collections::HashMap::new();

    let Some(items) = tools.and_then(Value::as_array) else {
        return (None, tool_name_map);
    };

    let converted: Vec<Tool> = items
        .iter()
        .filter_map(|item| {
            let (tool, mapping) = convert_responses_tool(item)?;
            if let Some((sanitized, original)) = mapping {
                tool_name_map.insert(sanitized, original);
            }
            Some(tool)
        })
        .collect();

    if converted.is_empty() {
        (None, tool_name_map)
    } else {
        (Some(converted), tool_name_map)
    }
}

pub(super) fn convert_responses_tool(item: &Value) -> Option<(Tool, Option<(String, String)>)> {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();

    if item.get("function").is_some() {
        let mut tool: Tool = serde_json::from_value(item.clone()).ok()?;
        let original_name = tool.function.name.clone();
        let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));
        tool.function.name = sanitized_name.clone();

        let mapping = if sanitized_name != original_name {
            Some((sanitized_name, original_name))
        } else {
            None
        };

        return Some((tool, mapping));
    }

    // 修复：MCP 工具缺少 type 字段导致之前被跳过
    // MCP 格式：{ "name": "...", "description": "...", "inputSchema": {...} }
    // 转换为 OpenAI 格式：{ "type": "function", "function": { "name": "...", "parameters": {...} } }
    if item_type.is_empty() && item.get("name").is_some() {
        let original_name = item.get("name").and_then(Value::as_str)?.to_string();
        let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));
        let description = item
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string);

        // 从 inputSchema 或 parameters 中提取参数定义
        // MCP 工具的 inputSchema 本身就是 JSON Schema，不需要访问 .json 字段
        let parameters = item
            .get("inputSchema")
            .cloned()
            .or_else(|| item.get("parameters").cloned());

        let mapping = if sanitized_name != original_name {
            Some((sanitized_name.clone(), original_name))
        } else {
            None
        };

        return Some((
            Tool {
                tool_type: "function".to_string(),
                function: ToolFunction {
                    name: sanitized_name,
                    description,
                    parameters,
                },
                cache_control: None,
            },
            mapping,
        ));
    }

    if item_type != "function" {
        return None;
    }

    let original_name = item.get("name").and_then(Value::as_str)?.to_string();
    let sanitized_name = shorten_tool_name(&sanitize_tool_name(&original_name));

    let mapping = if sanitized_name != original_name {
        Some((sanitized_name.clone(), original_name))
    } else {
        None
    };

    Some((
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: sanitized_name,
                description: item
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                parameters: item.get("parameters").cloned(),
            },
            cache_control: None,
        },
        mapping,
    ))
}

pub(super) fn normalize_tool_choice(
    tool_choice: &Option<Value>,
    tools: &Option<Vec<Tool>>,
) -> Result<Option<Value>, String> {
    let Some(choice) = tool_choice.as_ref() else {
        return Ok(None);
    };

    let choice_type = match choice {
        Value::String(raw) => raw.trim(),
        Value::Object(_) => choice
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .ok_or_else(|| "tool_choice.type 无效".to_string())?,
        _ => return Err("tool_choice 格式无效".to_string()),
    };

    match choice_type {
        "auto" => Ok(Some(json!({ "type": "auto" }))),
        "none" => Ok(Some(json!({ "type": "none" }))),
        "required" => {
            if tools.as_ref().is_none_or(|items| items.is_empty()) {
                return Err("tool_choice=required 时必须同时提供 tools".to_string());
            }
            Ok(Some(json!({ "type": "required" })))
        }
        "function" => {
            let name = choice
                .get("name")
                .or_else(|| choice.pointer("/function/name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "tool_choice.function.name 不能为空".to_string())?;

            let tool_exists = tools
                .as_ref()
                .map(|items| items.iter().any(|tool| tool.function.name == name))
                .unwrap_or(false);
            if !tool_exists {
                return Err(format!("tool_choice 指定的工具不存在: {name}"));
            }

            Ok(Some(json!({
                "type": "function",
                "name": name
            })))
        }
        other => Err(format!("暂不支持的 tool_choice.type: {other}")),
    }
}

pub(super) fn convert_tools(tools: &Option<Vec<Tool>>) -> Option<Vec<KiroTool>> {
    tools.as_ref().map(|items| {
        let mut result = Vec::new();

        // 插入所有工具定义
        for tool in items {
            result.push(KiroTool::ToolSpecification {
                tool_specification: KiroToolSpec {
                    name: tool.function.name.clone(),
                    description: tool_description(tool),
                    input_schema: KiroInputSchema {
                        json: tool_input_schema(tool),
                    },
                },
            });
        }

        // 注意：不要在 tools 数组末尾添加 cachePoint
        // Kiro API 会拒绝这种格式，导致 "Improperly formed request" 错误
        // Prompt Caching 应该通过其他方式触发（如消息的 cache_point metadata）

        result
    })
}

pub(super) fn tool_description(tool: &Tool) -> String {
    tool.function.description.clone().unwrap_or_default()
}

pub(super) fn tool_input_schema(tool: &Tool) -> Value {
    normalize_json_schema(
        tool.function
            .parameters
            .clone()
            .unwrap_or_else(|| json!({})),
    )
}

pub(super) fn normalize_json_schema(value: Value) -> Value {
    let mut schema = match value {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    // 清理 schema（删除 required: null 和空数组，递归清理嵌套结构）
    clean_schema(&mut schema);

    // 确保顶层有 type: "object"
    if !schema.contains_key("type") {
        schema.insert("type".to_string(), Value::String("object".to_string()));
    }

    Value::Object(schema)
}

/// 递归清理 JSON Schema，删除无效的 required 字段
/// Kiro API 会拒绝 required: null 或空的 required: []
pub(super) fn clean_schema(schema: &mut Map<String, Value>) {
    // 修复 required 字段：必须是非空数组或不存在
    if let Some(required) = schema.get("required") {
        let should_remove = match required {
            Value::Null => true,
            Value::Array(arr) if arr.is_empty() => true,
            _ => false,
        };
        if should_remove {
            schema.remove("required");
        }
    }

    // 递归清理 properties
    if let Some(Value::Object(properties)) = schema.get_mut("properties") {
        for value in properties.values_mut() {
            if let Value::Object(sub_schema) = value {
                clean_schema(sub_schema);
            }
        }
    }

    // 递归清理 items
    if let Some(Value::Object(items)) = schema.get_mut("items") {
        clean_schema(items);
    }

    // 递归清理 additionalProperties
    if let Some(Value::Object(additional)) = schema.get_mut("additionalProperties") {
        clean_schema(additional);
    }

    // 递归清理 allOf, oneOf, anyOf
    for key in &["allOf", "oneOf", "anyOf"] {
        if let Some(Value::Array(schemas)) = schema.get_mut(*key) {
            for item in schemas {
                if let Value::Object(sub_schema) = item {
                    clean_schema(sub_schema);
                }
            }
        }
    }
}

pub(super) fn process_tools_with_long_descriptions(
    tools: &Option<Vec<Tool>>,
) -> (Option<Vec<Tool>>, Option<String>) {
    let Some(tools) = tools else {
        return (None, None);
    };

    let mut processed = Vec::new();
    let mut long_docs = Vec::new();

    for tool in tools {
        let description = tool.function.description.clone().unwrap_or_default();
        if description.len() > TOOL_DESCRIPTION_MAX_LENGTH {
            long_docs.push(format!(
                "## Tool: {}\n\n{}",
                tool.function.name, description
            ));
            processed.push(Tool {
                tool_type: tool.tool_type.clone(),
                function: ToolFunction {
                    name: tool.function.name.clone(),
                    description: Some(format!(
                        "[Full documentation in system prompt under '## Tool: {}']",
                        tool.function.name
                    )),
                    parameters: tool.function.parameters.clone(),
                },
                cache_control: tool.cache_control.clone(),
            });
        } else {
            processed.push(tool.clone());
        }
    }

    let docs = if long_docs.is_empty() {
        None
    } else {
        Some(format!(
            "# Tool Documentation\n\n{}",
            long_docs.join("\n\n")
        ))
    };

    (Some(processed), docs)
}
