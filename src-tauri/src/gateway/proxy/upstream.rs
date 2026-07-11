//! 网关请求编排:向 Kiro 上游发起 generateAssistantResponse + 请求头装配(纯搬迁)。
use super::*;

pub(super) async fn call_generate_assistant_response<T: serde::Serialize + ?Sized>(
    upstream: &UpstreamCredentials,
    upstream_payload: &T,
    request_index: usize,
) -> Result<reqwest::Response, UpstreamRequestError> {
    let upstream_url = build_generate_assistant_response_url(&upstream.region);

    // 追加最新请求到日志文件
    if let Ok(payload_json) = serde_json::to_string(upstream_payload) {
        let log_dir = dirs::data_dir()
            .unwrap_or_default()
            .join(".kiro-account-manager")
            .join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let body_end = safe_truncate(&payload_json, 50000);
        let entry = format!(
            "[{}] kind=kiro_request idx={} upstream=generateAssistantResponse bytes={} truncated={} body={}\n",
            chrono::Local::now().format("%H:%M:%S"),
            request_index,
            payload_json.len(),
            body_end < payload_json.len(),
            &payload_json[..body_end]
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("kiro-request.log"))
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    }

    const MAX_RETRIES: u32 = 5;
    let mut attempt = 0;

    loop {
        attempt += 1;

        let upstream_resp = add_kiro_upstream_headers(
            upstream.http.post(&upstream_url),
            upstream,
            "application/vnd.amazon.eventstream",
            true,
            true,
            false,
        )
        .json(upstream_payload)
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                "api_error",
                sanitize_error(&format!("上游请求失败: {error}")),
                None,
            )
        })?;

        let status = upstream_resp.status();

        if status.is_success() {
            return Ok(upstream_resp);
        }

        let body = upstream_resp.text().await.unwrap_or_default();

        // 追加错误响应到日志文件
        {
            let log_dir = dirs::data_dir()
                .unwrap_or_default()
                .join(".kiro-account-manager")
                .join("logs");
            let body_end = safe_truncate(&body, 50000);
            let entry = format!(
                "[{}] kind=kiro_response idx={} upstream=generateAssistantResponse status={} bytes={} truncated={} body={}\n",
                chrono::Local::now().format("%H:%M:%S"),
                request_index,
                status.as_u16(),
                body.len(),
                body_end < body.len(),
                &body[..body_end]
            );
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_dir.join("kiro-request.log"))
                .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
        }

        // 既保留原始响应体透传，也必须先识别上游语义：
        // 403 bearer token invalid/expired 需要触发账号 token 刷新后重试。
        let (mapped_status, error_type, message) = map_upstream_error(status, &body);

        // 402 配额不足错误不重试，直接返回让外层切换账号
        if mapped_status == StatusCode::PAYMENT_REQUIRED {
            log::warn!("[网关] 上游 402 配额不足，type={}，交给外层切换账号", error_type);
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 429 限流错误不重试，直接返回让外层切换账号
        if mapped_status == StatusCode::TOO_MANY_REQUESTS {
            log::warn!("[网关] 上游 429 限流，type={}，交给外层切换账号", error_type);
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 401 认证错误不在 HTTP 层重试；交给外层刷新当前账号 token 或切换账号。
        if mapped_status == StatusCode::UNAUTHORIZED {
            log::warn!("[网关] 上游 401 认证错误，type={}，交给外层处理", error_type);
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 403 认证错误不在 HTTP 层重试；交给外层刷新当前账号 token 或切换账号。
        if mapped_status == StatusCode::FORBIDDEN {
            log::warn!("[网关] 上游 403 错误，type={}，交给外层处理", error_type);
            return Err((mapped_status, error_type, message, Some(body)));
        }

        // 5xx 服务器错误才重试
        let should_retry = attempt < MAX_RETRIES && mapped_status.is_server_error();

        if should_retry {
            let backoff_ms = 1000 * 2u64.pow(attempt - 1);
            log::warn!(
                "上游请求失败 (状态: {}, 类型: {}, 尝试: {}/{}), {}ms 后重试",
                mapped_status,
                error_type,
                attempt,
                MAX_RETRIES,
                backoff_ms
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
            continue;
        }

        // 其他错误也直接返回原始响应（不提取 message，直接透传 JSON）
        return Err((mapped_status, error_type, message, Some(body)));
    }
}

pub(super) fn add_kiro_upstream_headers(
    builder: reqwest::RequestBuilder,
    upstream: &UpstreamCredentials,
    accept: &str,
    include_opt_out: bool,
    include_agent_mode: bool,
    include_profile_arn_header: bool,
) -> reqwest::RequestBuilder {
    let invocation_id = uuid::Uuid::new_v4().to_string();
    let x_amz_user_agent = build_kiro_x_amz_user_agent(&upstream.machine_id);

    let mut builder = builder
        .header("Authorization", format!("Bearer {}", upstream.access_token))
        .header("Content-Type", "application/json")
        .header("Accept", accept)
        .header("host", build_kiro_runtime_host(&upstream.region))
        .header(header::USER_AGENT, upstream.user_agent.clone())
        .header("x-amz-user-agent", x_amz_user_agent)
        .header("amz-sdk-invocation-id", invocation_id)
        .header("amz-sdk-request", "attempt=1; max=3");

    if include_opt_out && upstream.send_opt_out {
        builder = builder.header("x-amzn-codewhisperer-optout", "true");
    }
    if include_agent_mode {
        builder = builder.header("x-amzn-kiro-agent-mode", DEFAULT_AGENT_MODE);
    }
    if include_profile_arn_header {
        if let Some(profile_arn) = upstream
            .profile_arn
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            builder = builder.header("x-amzn-kiro-profile-arn", profile_arn);
        }
    }
    if should_add_redirect_for_internal(upstream.provider.as_deref()) {
        builder = builder.header("redirect-for-internal", "true");
    }

    builder
}
