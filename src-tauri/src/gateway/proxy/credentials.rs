//! 网关请求编排:上游账号选择/刷新/凭据构造/失败追踪/可用模型探测(纯搬迁)。
use super::*;

pub(super) const MAX_FAILURES_PER_ACCOUNT: u32 = 3;

/// 获取账号可用模型列表
///
/// 调用 Kiro Management API 的 ListAvailableModels 接口获取账号权限内的模型
pub(super) async fn get_available_models_for_upstream(
    upstream: &UpstreamCredentials,
) -> Result<Vec<String>, String> {
    let client = KiroClient::from_client(upstream.http.clone());
    let (machine_id, profile_arn) = get_available_models_call_context(upstream);

    let response = client
        .list_available_models(
            &upstream.access_token,
            machine_id,
            &upstream.region,
            profile_arn,
        )
        .await?;

    // 解析返回的模型列表
    let models = response
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or("Invalid response: missing models array")?
        .iter()
        .filter_map(|m| {
            m.get("modelId")
                .and_then(|id| id.as_str())
                .map(String::from)
        })
        .collect();

    Ok(models)
}

pub(super) fn get_available_models_call_context(upstream: &UpstreamCredentials) -> (&str, Option<&str>) {
    (
        upstream.machine_id.as_str(),
        upstream.available_models_profile_arn.as_deref(),
    )
}

pub(super) async fn resolve_upstream_credentials(
    config: &GatewayConfig,
    state: &RouterState,
) -> Result<UpstreamCredentials, String> {
    match config.account_mode.as_str() {
        "single" | "group" | "pool" => resolve_managed_account_credentials(config, state).await,
        "local" => Err("2API不再支持 local 模式，请改用 single/group/pool 账号池模式".to_string()),
        _ => Err("accountMode 必须是 single/group/pool".to_string()),
    }
}

pub(super) async fn resolve_managed_account_credentials(
    config: &GatewayConfig,
    state: &RouterState,
) -> Result<UpstreamCredentials, String> {
    let mut store = AccountStore::new();
    store.reload();

    // 自愈机制：检查是否所有账号都因 "TooManyFailures" 被禁用
    let all_disabled_by_failures = match config.account_mode.as_str() {
        "single" => store
            .accounts
            .iter()
            .filter(|account| config.account_id.as_deref() == Some(account.id.as_str()))
            .all(|account| account.disabled_reason.as_deref() == Some("TooManyFailures")),
        "group" => {
            let group_accounts: Vec<_> = store
                .accounts
                .iter()
                .filter(|account| config.group_id.as_deref() == account.group_id.as_deref())
                .collect();

            !group_accounts.is_empty()
                && group_accounts
                    .iter()
                    .all(|account| account.disabled_reason.as_deref() == Some("TooManyFailures"))
        }
        "pool" => {
            let pool_accounts: Vec<_> = store
                .accounts
                .iter()
                .filter(|account| config.pool_account_ids.contains(&account.id))
                .collect();

            !pool_accounts.is_empty()
                && pool_accounts
                    .iter()
                    .all(|account| account.disabled_reason.as_deref() == Some("TooManyFailures"))
        }
        _ => false,
    };

    if all_disabled_by_failures {
        for account in store.accounts.iter_mut() {
            if account.disabled_reason.as_deref() == Some("TooManyFailures") {
                account.failure_count = 0;
                account.status = "active".to_string();
                account.disabled_reason = None;
            }
        }
        let _ = store.save_to_file();
    }

    let accounts = match config.account_mode.as_str() {
        "single" => store
            .accounts
            .iter()
            .filter(|account| config.account_id.as_deref() == Some(account.id.as_str()))
            .cloned()
            .collect::<Vec<_>>(),
        "group" => store
            .accounts
            .iter()
            .filter(|account| {
                config.group_id.as_deref() == account.group_id.as_deref()
                    && account.is_available()
                    && account.enabled
            })
            .cloned()
            .collect::<Vec<_>>(),
        "pool" => store
            .accounts
            .iter()
            .filter(|account| {
                config.pool_account_ids.contains(&account.id)
                    && account.is_available()
                    && account.enabled
            })
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    if accounts.is_empty() {
        return Err("__402__未找到符合2API配置的可用账号".to_string());
    }

    // 使用 LoadBalancer 选择账号
    let selected_account = state.load_balancer.select_account(&accounts).await;

    let Some(account) = selected_account else {
        return Err("__402__LoadBalancer 未能选择可用账号".to_string());
    };

    // 增加连接计数
    state.load_balancer.increment_connections(&account.id).await;
    let request_start = Instant::now();

    // 检查 token 是否真正过期（不再提前刷新，避免和定时器/IDE 冲突导致 429）
    // 定时器会提前 10 分钟刷新，网关只在 token 真正过期时才刷新
    let need_refresh = match &account.expires_at {
        Some(expires_at) => is_token_expired(expires_at),
        None => true, // 没有过期时间，强制刷新
    };

    // 如果 token 没过期且有 access_token，直接使用
    if !need_refresh {
        if let Some(access_token) = &account.access_token {
            if !access_token.is_empty() {
                // token 未过期，不需要 refresh，释放连接计数
                state.load_balancer.decrement_connections(&account.id).await;
                let ctx = crate::commands::common::resolve_kiro_call_context(
                    &account,
                    &state.config.region,
                );
                let available_models_profile_arn = ctx.profile_arn.clone();
                let http = match build_streaming_http_client_for_account(&account) {
                    Ok(http) => http,
                    Err(error) => {
                        state.load_balancer.decrement_connections(&account.id).await;
                        return Err(format!(
                            "创建账号 {} 的2API HTTP 客户端失败: {}",
                            account.label,
                            sanitize_error(&error)
                        ));
                    }
                };
                return Ok(UpstreamCredentials {
                    account_id: account.id.clone(),
                    access_token: access_token.clone(),
                    machine_id: ctx.machine_id.clone(),
                    profile_arn: ctx.profile_arn,
                    available_models_profile_arn,
                    provider: account.provider.clone(),
                    region: ctx.region,
                    source_label: format_managed_upstream_source(&state.config, &account),
                    user_agent: build_kiro_custom_user_agent(&ctx.machine_id),
                    auth_method: account.auth_method.clone(),
                    send_opt_out: should_send_codewhisperer_optout(),
                    http,
                });
            }
        }
    }

    match refresh_token_by_provider_with_account_proxy(&account).await {
        Ok(refresh) => {
            let usage_result = get_usage_by_account(&account, &refresh.access_token).await;
            let mut usage_data = None;
            let mut is_banned = false;
            let mut is_auth_error = false;

            if let Ok(usage) = usage_result {
                usage_data = Some(usage.usage_data);
                is_banned = usage.is_banned;
                is_auth_error = usage.is_auth_error;
            }

            // 失败追踪：如果账号被封禁或认证失败，累加失败计数
            let should_increment_failure = is_banned || is_auth_error;

            persist_account_refresh(
                &account,
                &refresh,
                usage_data.clone(),
                is_banned,
                is_auth_error,
                should_increment_failure,
            );

            // 减少连接计数
            state.load_balancer.decrement_connections(&account.id).await;

            if is_banned || is_auth_error {
                // 记录失败
                state.load_balancer.record_failure(&account.id).await;
                return Err(format!("账号 {} 已不可用", account.label));
            }

            if let Some(usage_data) = &usage_data {
                if usage_exceeds_threshold(usage_data, config.threshold) {
                    // 配额超阈值，直接禁用账号
                    state.load_balancer.record_failure(&account.id).await;
                    disable_account_by_id(&account.id, "配额已满");
                    return Err(format!("账号 {} 配额已满，已自动禁用", account.label));
                } else {
                    // 配额已恢复，检查是否需要自动启用账号
                    // 仅当账号因配额满被自动禁用时才自动启用
                    if !account.enabled && account.disabled_reason.as_deref() == Some("配额已满")
                    {
                        enable_account_by_id(&account.id);
                    }
                }
            }

            // 记录成功
            let response_time_ms = request_start.elapsed().as_millis() as u64;
            state
                .load_balancer
                .record_success(&account.id, response_time_ms)
                .await;

            build_upstream_credentials_from_refresh(config, &account, refresh)
        }
        Err(error) => {
            // 减少连接计数
            state.load_balancer.decrement_connections(&account.id).await;
            // 记录失败
            state.load_balancer.record_failure(&account.id).await;

            Err(format!(
                "刷新账号 {} 失败: {}",
                account.label,
                sanitize_error(&error)
            ))
        }
    }
}

pub(super) async fn force_refresh_upstream_credentials(
    config: &GatewayConfig,
    state: &RouterState,
    upstream: &UpstreamCredentials,
) -> Result<UpstreamCredentials, String> {
    let mut store = AccountStore::new();
    store.reload();

    let account = store
        .accounts
        .iter()
        .find(|candidate| candidate.id == upstream.account_id)
        .cloned()
        .ok_or_else(|| format!("账号 {} 不存在，无法刷新 Token", upstream.source_label))?;

    let refresh = refresh_token_by_provider_with_account_proxy(&account)
        .await
        .map_err(|error| {
            format!(
                "刷新账号 {} 失败: {}",
                account.label,
                sanitize_error(&error)
            )
        })?;

    let usage_result = get_usage_by_account(&account, &refresh.access_token).await;
    let mut usage_data = None;
    let mut is_banned = false;
    let mut is_auth_error = false;

    if let Ok(usage) = usage_result {
        usage_data = Some(usage.usage_data);
        is_banned = usage.is_banned;
        is_auth_error = usage.is_auth_error;
    }

    persist_account_refresh(
        &account,
        &refresh,
        usage_data.clone(),
        is_banned,
        is_auth_error,
        is_banned || is_auth_error,
    );

    if is_banned || is_auth_error {
        state.load_balancer.record_failure(&account.id).await;
        return Err(format!("账号 {} 刷新后仍不可用", account.label));
    }

    if let Some(usage_data) = &usage_data {
        if usage_exceeds_threshold(usage_data, config.threshold) {
            state.load_balancer.record_failure(&account.id).await;
            disable_account_by_id(&account.id, "配额已满");
            return Err(format!("账号 {} 配额已满，已自动禁用", account.label));
        }
    }

    build_upstream_credentials_from_refresh(config, &account, refresh)
}

pub(super) fn build_upstream_credentials_from_refresh(
    config: &GatewayConfig,
    account: &Account,
    refresh: RefreshResult,
) -> Result<UpstreamCredentials, String> {
    let machine_id = account_machine_id_or_new(&account.machine_id);
    let profile_arn = resolve_profile_arn_from_candidates(
        refresh.profile_arn.as_deref(),
        account.profile_arn.as_deref(),
        account.provider.as_deref(),
    );
    let region = resolve_kiro_upstream_region(
        profile_arn.as_deref(),
        account.region.as_deref(),
        &config.region,
    );

    let http = build_streaming_http_client_for_account(account).map_err(|error| {
        format!(
            "创建账号 {} 的2API HTTP 客户端失败: {}",
            account.label,
            sanitize_error(&error)
        )
    })?;

    Ok(UpstreamCredentials {
        account_id: account.id.clone(),
        access_token: refresh.access_token,
        machine_id: machine_id.clone(),
        profile_arn: profile_arn.clone(),
        available_models_profile_arn: profile_arn,
        provider: account.provider.clone(),
        region,
        source_label: format_managed_upstream_source(config, account),
        user_agent: build_kiro_custom_user_agent(&machine_id),
        auth_method: account.auth_method.clone(),
        send_opt_out: should_send_codewhisperer_optout(),
        http,
    })
}
/// 根据账号 provider 返回默认的 profileArn
/// BuilderId 账号和 Social 账号（Github/Google）使用不同的 profileArn

pub(super) fn format_managed_upstream_source(config: &GatewayConfig, account: &Account) -> String {
    let account_label = account
        .email
        .as_deref()
        .or(account.user_id.as_deref())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim())
        .unwrap_or("unknown");

    match config.account_mode.as_str() {
        "single" => format!("single:{account_label}"),
        "group" => format!(
            "group:{}:{account_label}",
            config.group_id.as_deref().unwrap_or("unknown")
        ),
        "pool" => format!("pool:{account_label}"),
        _ => account_label.to_string(),
    }
}

/// 禁用指定账号（配额满时自动调用）
pub(super) fn disable_account_by_id(account_id: &str, reason: &str) {
    let mut store = AccountStore::new();
    if let Some(account) = store.accounts.iter_mut().find(|a| a.id == account_id) {
        account.enabled = false;
        account.disabled_reason = Some(reason.to_string());
        store.save_to_file();
        log::info!("[网关] 账号 {} 已自动禁用: {}", account_id, reason);
    }
}

/// 启用指定账号（配额恢复时自动调用）
pub(super) fn enable_account_by_id(account_id: &str) {
    let mut store = AccountStore::new();
    if let Some(account) = store.accounts.iter_mut().find(|a| a.id == account_id) {
        account.enabled = true;
        account.disabled_reason = None;
        store.save_to_file();
        log::info!("[网关] 账号 {} 配额已恢复，已自动启用", account_id);
    }
}

pub(super) fn persist_account_refresh(
    account: &Account,
    refresh: &RefreshResult,
    usage_data: Option<Value>,
    is_banned: bool,
    is_auth_error: bool,
    should_increment_failure: bool,
) {
    let mut store = AccountStore::new();
    if let Some(target) = store
        .accounts
        .iter_mut()
        .find(|candidate| candidate.id == account.id)
    {
        // 应用 token 字段更新（Option 字段仅在新值存在时覆盖，避免清空已有值）
        crate::commands::common::apply_refreshed_account_tokens(target, &refresh);
        if let Some(data) = usage_data {
            target.usage_data = Some(data);
        }
        update_account_status(target, is_banned, is_auth_error);

        // 失败追踪逻辑
        if should_increment_failure {
            target.failure_count += 1;
            target.last_failure_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

            // 如果失败次数达到阈值，自动禁用账号
            if target.failure_count >= MAX_FAILURES_PER_ACCOUNT {
                target.status = "disabled".to_string();
                target.disabled_reason = Some("TooManyFailures".to_string());
                log::warn!(
                    "[Gateway] 账号 {} 失败次数达到 {}，自动禁用",
                    target.label,
                    MAX_FAILURES_PER_ACCOUNT
                );
            }
        } else {
            // 请求成功，重置失败计数并累加成功计数
            target.failure_count = 0;
            target.success_count += 1;
            target.last_failure_at = None;

            // 如果之前因为失败过多被禁用，现在恢复
            if target.disabled_reason.as_deref() == Some("TooManyFailures") {
                target.disabled_reason = None;
                if target.status == "disabled" {
                    target.status = "active".to_string();
                }
            }
        }

        let _ = store.save_to_file();
    }
}

pub(super) fn usage_exceeds_threshold(usage_data: &Value, threshold: i32) -> bool {
    crate::core::usage::usage_exceeds_threshold(Some(usage_data), f64::from(threshold))
}

pub(super) fn extract_account_id_from_upstream(upstream: &UpstreamCredentials) -> String {
    upstream.account_id.clone()
}
