//! 网关请求编排:客户端鉴权/请求归一化/IP 白名单(纯搬迁)。
use super::*;

pub(super) fn ip_matches_allowlist(ip: IpAddr, allowlist: &[String]) -> bool {
    allowlist.iter().any(|entry| {
        let entry = entry.trim();
        entry
            .parse::<IpAddr>()
            .map(|allowed| allowed == ip)
            .unwrap_or(false)
            || entry
                .parse::<ipnet::IpNet>()
                .map(|network| network.contains(&ip))
                .unwrap_or(false)
    })
}

pub(super) fn normalize_request(format: ResponseFormat, payload: &Value) -> Result<NormalizedRequest, String> {
    match format {
        ResponseFormat::Anthropic => {
            let request: AnthropicMessagesRequest = serde_json::from_value(payload.clone())
                .map_err(|error| format!("Anthropic 请求解析失败: {error}"))?;
            Ok(normalize_anthropic_request(&request))
        }
        ResponseFormat::Responses => normalize_openai_responses_request(payload),
        ResponseFormat::OpenAI => {
            let request: OpenAIChatRequest = serde_json::from_value(payload.clone())
                .map_err(|error| format!("OpenAI 请求解析失败: {error}"))?;
            Ok(crate::gateway::converter::normalize_openai_chat_request(
                &request,
            ))
        }
    }
}

pub(super) fn verify_client_auth(headers: &HeaderMap, config: &GatewayConfig) -> Result<(), String> {
    let expected_keys = effective_client_api_keys(config);
    if expected_keys.is_empty() {
        return Err("客户端 API Key 未配置".to_string());
    }

    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());

    if expected_keys.iter().any(|expected| {
        authorization == Some(expected.as_str()) || api_key == Some(expected.as_str())
    }) {
        Ok(())
    } else {
        Err("客户端 API Key 无效".to_string())
    }
}
