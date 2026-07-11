// 添加 IdC 账号（BuilderId / Enterprise）

use super::AddAccountResult;
use crate::auth::providers::{AuthProvider, IdcProvider, RefreshMetadata};
use crate::commands::common::{
    calc_expires_at, extract_user_info, find_existing_account_idx, generate_account_machine_id,
    get_usage_by_provider_with_machine_id, lock_store, save_store, update_account_status,
};
use crate::core::account::Account;
use crate::state::AppState;
use crate::utils::client_id_hash::{extract_start_url_from_client_secret, normalize_start_url};
use tauri::State;

/// 添加 IdC 账号（BuilderId 或 Enterprise）
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri IPC 命令签名需要显式参数，避免前后端调用契约破坏
pub async fn add_account_by_idc(
    state: State<'_, AppState>,
    provider: Option<String>,
    refresh_token: String,
    client_id: String,
    client_secret: String,
    region: Option<String>,
    machine_id: Option<String>,
    access_token: Option<String>,
    password: Option<String>,
    start_url: Option<String>,
    client_id_hash: Option<String>,
) -> Result<AddAccountResult, String> {
    // 从参数中获取 provider，默认为 BuilderId
    let provider_id = provider.unwrap_or_else(|| "BuilderId".to_string());

    // 验证 provider 是否合法
    if provider_id != "BuilderId" && provider_id != "Enterprise" {
        return Err(format!("不支持的 provider: {}", provider_id));
    }

    add_account_by_idc_internal(
        state,
        IdcAccountParams {
            refresh_token,
            client_id,
            client_secret,
            region,
            machine_id,
            access_token,
            password,
            provider_id,
            start_url,
            client_id_hash,
        },
    )
    .await
}

/// `IdC` 账号添加参数
struct IdcAccountParams {
    refresh_token: String,
    client_id: String,
    client_secret: String,
    region: Option<String>,
    machine_id: Option<String>,
    access_token: Option<String>,
    password: Option<String>,
    provider_id: String,
    start_url: Option<String>,
    client_id_hash: Option<String>,
}

/// 内部函数：添加 `IdC` 账号（BuilderId 或 Enterprise）
async fn add_account_by_idc_internal(
    state: State<'_, AppState>,
    params: IdcAccountParams,
) -> Result<AddAccountResult, String> {
    let is_enterprise = params.provider_id == "Enterprise";

    // 从 clientSecret JWT 中提取 startUrl（如果未提供）。
    // 两条来源都过 normalize_start_url 去尾斜杠：params.start_url 可能由前端带斜杠传入，
    // JWT 真相源虽已规范化，这里统一兜底，保证落进 account.start_url 的永远是无斜杠规范形。
    let start_url = if let Some(ref url) = params.start_url {
        Some(normalize_start_url(url))
    } else if is_enterprise {
        extract_start_url_from_client_secret(&params.client_secret)
    } else {
        None
    };

    // BuilderId 和 Enterprise 都使用默认 region（如果未提供）
    let region = params.region.unwrap_or_else(|| "us-east-1".to_string());

    // 获取 machine_id（企业账号多区域探测需要）
    let machine_id = params
        .machine_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(generate_account_machine_id);

    // 企业账号导入时强制刷新 token（导入时的 access_token 很可能已过期）
    let (
        final_access_token,
        final_refresh_token,
        usage_result,
        expires_at,
        id_token,
        sso_session_id,
    ) = if is_enterprise || params.access_token.is_none() {
        // 企业账号或没有 access_token 时，直接刷新
        let metadata = RefreshMetadata {
            client_id: Some(params.client_id.clone()),
            client_secret: Some(params.client_secret.clone()),
            region: Some(region.clone()),
            ..Default::default()
        };
        let idc_provider = IdcProvider::new(&params.provider_id, &region, start_url.clone());
        let auth_result = idc_provider
            .refresh_token(&params.refresh_token, metadata)
            .await?;

        // 使用用户选择的 region，不再进行探测
        let usage_result = match get_usage_by_provider_with_machine_id(
            &params.provider_id,
            &auth_result.access_token,
            &machine_id,
        )
        .await
        {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to get usage in add_account_by_idc: {}", e);
                // 即使 getUsageLimits 失败，也能保存账号
                crate::commands::common::UsageResult {
                    usage_data: serde_json::json!({}),
                    is_banned: false,
                    is_auth_error: false,
                }
            }
        };

        let expires_at = calc_expires_at(auth_result.expires_in);
        (
            auth_result.access_token,
            auth_result.refresh_token,
            usage_result,
            expires_at,
            auth_result.id_token,
            auth_result.sso_session_id,
        )
    } else if let Some(at) = params.access_token {
        // BuilderId 且有 access_token 时，先尝试使用
        // BuilderId 使用原有逻辑
        match get_usage_by_provider_with_machine_id(&params.provider_id, &at, &machine_id).await {
            Ok(result) if result.is_auth_error => {
                // 401 了，刷新 token
                let metadata = RefreshMetadata {
                    client_id: Some(params.client_id.clone()),
                    client_secret: Some(params.client_secret.clone()),
                    region: Some(region.clone()),
                    ..Default::default()
                };
                let idc_provider =
                    IdcProvider::new(&params.provider_id, &region, start_url.clone());
                let auth_result = idc_provider
                    .refresh_token(&params.refresh_token, metadata)
                    .await?;
                let new_usage = match get_usage_by_provider_with_machine_id(
                    &params.provider_id,
                    &auth_result.access_token,
                    &machine_id,
                )
                .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        log::warn!("Failed to get usage after token refresh: {}", e);
                        // 即使 getUsageLimits 失败，也能保存账号
                        crate::commands::common::UsageResult {
                            usage_data: serde_json::json!({}),
                            is_banned: false,
                            is_auth_error: false,
                        }
                    }
                };
                let expires_at = calc_expires_at(auth_result.expires_in);
                (
                    auth_result.access_token,
                    auth_result.refresh_token,
                    new_usage,
                    expires_at,
                    auth_result.id_token,
                    auth_result.sso_session_id,
                )
            }
            Ok(result) => {
                // access_token 有效，不需要刷新
                (
                    at,
                    params.refresh_token.clone(),
                    result,
                    String::new(),
                    None,
                    None,
                )
            }
            Err(e) => return Err(e),
        }
    } else {
        // 没有 access_token，直接刷新
        let metadata = RefreshMetadata {
            client_id: Some(params.client_id.clone()),
            client_secret: Some(params.client_secret.clone()),
            region: Some(region.clone()),
            ..Default::default()
        };
        let idc_provider = IdcProvider::new(&params.provider_id, &region, start_url.clone());
        let auth_result = idc_provider
            .refresh_token(&params.refresh_token, metadata)
            .await?;

        // 使用用户选择的 region，不再进行探测
        let usage_result = match get_usage_by_provider_with_machine_id(
            &params.provider_id,
            &auth_result.access_token,
            &machine_id,
        )
        .await
        {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to get usage in add_account_by_idc: {}", e);
                // 即使 getUsageLimits 失败，也能保存账号
                crate::commands::common::UsageResult {
                    usage_data: serde_json::json!({}),
                    is_banned: false,
                    is_auth_error: false,
                }
            }
        };

        let expires_at = calc_expires_at(auth_result.expires_in);
        (
            auth_result.access_token,
            auth_result.refresh_token,
            usage_result,
            expires_at,
            auth_result.id_token,
            auth_result.sso_session_id,
        )
    };

    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // ========== Enterprise 和 BuilderId 分开处理 ==========

    if is_enterprise {
        // Enterprise 账号：email 和 user_id 都可选（如果 API 返回为空）

        // 解析 client_id_hash：走统一裁决点，Enterprise 会在此硬校验（issue #119）
        let client_id_hash = Some(crate::commands::common::resolve_idc_client_id_hash(
            &params.provider_id,
            params.client_id_hash.as_deref(),
            start_url.as_deref(),
        )?);

        let mut store = lock_store(&state.store, "store")?;
        let existing_idx = find_existing_account_idx(
            &store.accounts,
            new_email.as_ref(),
            &params.provider_id,
            &final_refresh_token,
            user_id.as_ref(),
        );

        let is_new = existing_idx.is_none();

        let account = if let Some(idx) = existing_idx {
            // 更新已存在的账号
            let existing = &mut store.accounts[idx];
            existing.access_token = Some(final_access_token);
            existing.refresh_token = Some(final_refresh_token);
            existing.email = new_email; // 更新 email（可能是 None）
            existing.user_id = user_id.clone();
            existing.provider = Some(params.provider_id.clone());
            existing.auth_method = Some("IdC".to_string()); // 确保 authMethod 正确
            if !expires_at.is_empty() {
                existing.expires_at = Some(expires_at);
            }
            existing.client_id = Some(params.client_id.clone());
            existing.client_secret = Some(params.client_secret.clone());
            existing.region = Some(region.clone());
            existing.client_id_hash = client_id_hash.clone(); // 可能是 None
            existing.start_url = start_url.clone();
            if existing
                .machine_id
                .as_ref()
                .is_none_or(|id| id.trim().is_empty())
            {
                existing.machine_id = Some(machine_id.clone());
            }
            if id_token.is_some() {
                existing.id_token = id_token;
            }
            if sso_session_id.is_some() {
                existing.sso_session_id = sso_session_id;
            }
            existing.usage_data = Some(usage_result.usage_data);
            update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
            existing.clone()
        } else {
            // 创建新的 Enterprise 账号
            let mut account =
                Account::new_enterprise(user_id.clone().unwrap_or_default(), "Kiro Enterprise 账号".to_string());
            account.access_token = Some(final_access_token);
            account.refresh_token = Some(final_refresh_token);
            account.email = new_email; // 可能是 None
            if !expires_at.is_empty() {
                account.expires_at = Some(expires_at);
            }
            account.client_id = Some(params.client_id.clone());
            account.client_secret = Some(params.client_secret.clone());
            account.region = Some(region.clone());
            account.client_id_hash = client_id_hash; // 可能是 None
            account.start_url = start_url.clone();
            account.id_token = id_token;
            account.sso_session_id = sso_session_id;
            account.usage_data = Some(usage_result.usage_data);
            update_account_status(
                &mut account,
                usage_result.is_banned,
                usage_result.is_auth_error,
            );
            account.machine_id = Some(machine_id.clone());
            account.password.clone_from(&params.password);
            store.accounts.insert(0, account.clone());
            account
        };

        save_store(&store)?;
        Ok(AddAccountResult { account, is_new })
    } else {
        // BuilderId 账号：允许没有 userId/email，用 refreshToken 去重

        // 解析 client_id_hash：走统一裁决点（BuilderId 缺 startUrl 时用常量兜底）
        let client_id_hash = Some(crate::commands::common::resolve_idc_client_id_hash(
            &params.provider_id,
            params.client_id_hash.as_deref(),
            params.start_url.as_deref(),
        )?);

        let mut store = lock_store(&state.store, "store")?;
        let existing_idx = find_existing_account_idx(
            &store.accounts,
            new_email.as_ref(),
            &params.provider_id,
            &final_refresh_token,
            user_id.as_ref(),
        );

        let is_new = existing_idx.is_none();

        let account = if let Some(idx) = existing_idx {
            // 更新已存在的账号
            let existing = &mut store.accounts[idx];
            existing.access_token = Some(final_access_token);
            existing.refresh_token = Some(final_refresh_token);
            existing.provider = Some(params.provider_id.clone());
            existing.auth_method = Some("IdC".to_string()); // 确保 authMethod 正确
            existing.user_id = user_id;
            if !expires_at.is_empty() {
                existing.expires_at = Some(expires_at);
            }
            existing.client_id = Some(params.client_id.clone());
            existing.client_secret = Some(params.client_secret.clone());
            existing.region = Some(region.clone());
            existing.client_id_hash = client_id_hash.clone(); // 可能是 None
            if existing
                .machine_id
                .as_ref()
                .is_none_or(|id| id.trim().is_empty())
            {
                existing.machine_id = Some(machine_id.clone());
            }
            if id_token.is_some() {
                existing.id_token = id_token;
            }
            if sso_session_id.is_some() {
                existing.sso_session_id = sso_session_id;
            }
            existing.usage_data = Some(usage_result.usage_data);
            update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
            existing.clone()
        } else {
            // 创建新的 BuilderId 账号
            // 使用 user_id 或 email 作为标识
            let display_id = new_email
                .clone()
                .or_else(|| user_id.clone())
                .unwrap_or_else(|| "BuilderId 账号".to_string());

            let mut account = Account::new(display_id.clone(), "Kiro BuilderId 账号".to_string());
            account.access_token = Some(final_access_token);
            account.refresh_token = Some(final_refresh_token);
            account.provider = Some(params.provider_id.clone());
            account.auth_method = Some("IdC".to_string());
            account.email = new_email; // 可能是 None
            account.user_id = user_id; // 可能是 None
            if !expires_at.is_empty() {
                account.expires_at = Some(expires_at);
            }
            account.client_id = Some(params.client_id.clone());
            account.client_secret = Some(params.client_secret.clone());
            account.region = Some(region.clone());
            account.client_id_hash = client_id_hash; // 可能是 None
            account.id_token = id_token;
            account.sso_session_id = sso_session_id;
            account.usage_data = Some(usage_result.usage_data);
            update_account_status(
                &mut account,
                usage_result.is_banned,
                usage_result.is_auth_error,
            );
            account.machine_id = Some(machine_id);
            account.password = params.password;
            store.accounts.insert(0, account.clone());
            account
        };

        save_store(&store)?;
        Ok(AddAccountResult { account, is_new })
    }
}

