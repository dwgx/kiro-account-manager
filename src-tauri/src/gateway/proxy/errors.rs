//! 网关请求编排:错误映射/构造/透传(纯搬迁)。
use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct GatewayErrorDetails<'a> {
    pub(super) status: StatusCode,
    pub(super) error_type: &'a str,
    pub(super) message: &'a str,
    pub(super) response_body: Option<&'a str>,
}

pub(super) fn build_gateway_error_body(
    format: ResponseFormat,
    status: StatusCode,
    error_type: &str,
    message: &str,
) -> Value {
    match format {
        ResponseFormat::Anthropic => json!({
            "type": "error",
            "error": {
                "type": error_type,
                "message": message
            }
        }),
        ResponseFormat::Responses => json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": status.as_u16()
            }
        }),
        ResponseFormat::OpenAI => json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": status.as_u16()
            }
        }),
    }
}

pub(super) async fn gateway_error_with_log(
    state: &RouterState,
    format: ResponseFormat,
    context: &RequestLogContext<'_>,
    error: GatewayErrorDetails<'_>,
) -> Response {
    // 如果有 response_body，尝试从中提取 message 用于 last_error
    let error_message = if error.message.is_empty() {
        error
            .response_body
            .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
            .and_then(|json| {
                json.pointer("/message")
                    .or_else(|| json.pointer("/error/message"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "上游错误".to_string())
    } else {
        error.message.to_string()
    };

    *state.last_error.lock().await = Some(error_message.clone());

    // 尝试从错误响应体中提取token信息
    let (input_tokens, output_tokens, cache_read, cache_creation) = error
        .response_body
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .and_then(|json| {
            let usage = json.get("usage")?;
            Some((
                usage
                    .get("input_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                usage
                    .get("output_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                usage
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                usage
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
            ))
        })
        .unwrap_or((None, None, None, None));

    // 日志中记录的响应体：优先使用原始响应，否则构造
    let logged_response_body = error.response_body.map(str::to_string).or_else(|| {
        Some(serialize_logged_value(&build_gateway_error_body(
            format,
            error.status,
            error.error_type,
            error.message,
        )))
    });

    write_request_log(
        context,
        error.status,
        "error",
        if error.message.is_empty() {
            None
        } else {
            Some(error.message)
        },
        Some(error.error_type),
        logged_response_body.as_deref(),
        input_tokens,
        output_tokens,
        cache_read,
        cache_creation,
        state,
    );
    gateway_error_response(
        format,
        error.status,
        error.error_type,
        error.message,
        error.response_body,
    )
}

pub(super) fn gateway_error_response(
    format: ResponseFormat,
    status: StatusCode,
    error_type: &str,
    message: &str,
    response_body: Option<&str>,
) -> Response {
    // 如果有原始响应体且是有效的 JSON，直接透传
    if let Some(body_str) = response_body {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(body_str) {
            return (status, Json(json_value)).into_response();
        }
    }

    // 否则使用构造的错误响应
    let body = build_gateway_error_body(format, status, error_type, message);
    (status, Json(body)).into_response()
}

pub(super) fn map_upstream_error(status: StatusCode, body: &str) -> (StatusCode, &'static str, String) {
    let sanitized = sanitize_error(&extract_error_message(body));
    let explicit_error_type = extract_error_type(body);
    let text = body.to_lowercase();

    // 检测封禁错误（403 + TEMPORARILY_SUSPENDED 或 AccessDeniedException + TemporarilySuspended）
    let is_banned = status == StatusCode::FORBIDDEN
        && (body.contains("TEMPORARILY_SUSPENDED")
            || (body.contains("AccessDeniedException") && body.contains("TemporarilySuspended")));

    // 检测token失效错误（403 + bearer token invalid/expired）
    let is_token_invalid = status == StatusCode::FORBIDDEN
        && (text.contains("bearer token") || text.contains("bearer_token"))
        && (text.contains("invalid") || text.contains("expired"));

    let mapped_status = if status == StatusCode::BAD_GATEWAY || status == StatusCode::OK {
        if explicit_error_type == Some("authentication_error") {
            StatusCode::UNAUTHORIZED
        } else if explicit_error_type == Some("permission_error") {
            StatusCode::FORBIDDEN
        } else if explicit_error_type == Some("rate_limit_error") {
            StatusCode::TOO_MANY_REQUESTS
        } else if explicit_error_type == Some("invalid_request_error") {
            StatusCode::BAD_REQUEST
        } else if text.contains("throttlingexception")
            || text.contains("servicequotaexceededexception")
        {
            StatusCode::TOO_MANY_REQUESTS
        } else if text.contains("accessdeniedexception") {
            StatusCode::FORBIDDEN
        } else if text.contains("validationexception") {
            StatusCode::BAD_REQUEST
        } else if text.contains("serviceunavailableexception") {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::BAD_GATEWAY
        }
    } else {
        status
    };
    // 根据检测结果返回特殊的error_type和message
    let (error_type, message) = if is_banned {
        // 对于封禁错误，返回以 BANNED: 开头的消息，以便前端可以识别
        ("account_banned_error", format!("BANNED: {}", sanitized))
    } else if is_token_invalid {
        ("token_expired_error", sanitized)
    } else {
        let error_type = explicit_error_type.unwrap_or(match mapped_status {
            StatusCode::UNAUTHORIZED => "authentication_error",
            StatusCode::FORBIDDEN => "permission_error",
            StatusCode::PAYMENT_REQUIRED => "insufficient_quota",
            StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
            StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND | StatusCode::CONFLICT => {
                "invalid_request_error"
            }
            _ => "api_error",
        });
        (error_type, sanitized)
    };

    (mapped_status, error_type, message)
}

pub(super) fn extract_error_type(body: &str) -> Option<&'static str> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    let raw = value
        .pointer("/error/type")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/type").and_then(Value::as_str))?;

    match raw {
        "authentication_error" => Some("authentication_error"),
        "permission_error" => Some("permission_error"),
        "insufficient_quota" => Some("insufficient_quota"),
        "rate_limit_error" => Some("rate_limit_error"),
        "invalid_request_error" => Some("invalid_request_error"),
        "api_error" => Some("api_error"),
        _ => None,
    }
}

pub(super) fn detect_upstream_error_body(body: &str) -> Option<(StatusCode, &'static str, String)> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    let value = serde_json::from_str::<Value>(trimmed).ok()?;
    let object = value.as_object()?;
    let has_error_container = object.get("error").is_some();
    let has_error_metadata = object.get("__type").and_then(Value::as_str).is_some()
        || object.get("errorCode").and_then(Value::as_str).is_some()
        || object.get("Message").and_then(Value::as_str).is_some();
    let has_message_only_error = object.get("message").and_then(Value::as_str).is_some()
        && object.get("content").is_none()
        && object.get("output").is_none()
        && object.get("choices").is_none()
        && object.get("results").is_none();

    if has_error_container || has_error_metadata || has_message_only_error {
        Some(map_upstream_error(StatusCode::OK, trimmed))
    } else {
        None
    }
}

pub(super) fn extract_error_message(body: &str) -> String {
    if body.trim().is_empty() {
        return "上游返回空错误响应".to_string();
    }
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        for pointer in [
            "/message",
            "/Message",
            "/error/message",
            "/reason",
            "/__type",
            "/errorCode",
        ] {
            if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
                return text.to_string();
            }
        }
    }
    body.to_string()
}

pub(super) fn sanitize_error(message: &str) -> String {
    let mut sanitized = message.to_string();
    for pattern in [
        r"Bearer\s+[A-Za-z0-9._\-]+",
        r#""accessToken"\s*:\s*"[^"]+""#,
        r#""refreshToken"\s*:\s*"[^"]+""#,
        r#""clientSecret"\s*:\s*"[^"]+""#,
        r#"sk-[A-Za-z0-9]+"#,
    ] {
        if let Ok(regex) = Regex::new(pattern) {
            sanitized = regex.replace_all(&sanitized, "[REDACTED]").to_string();
        }
    }
    sanitized
}
