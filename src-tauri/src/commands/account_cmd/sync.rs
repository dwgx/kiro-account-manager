// 同步配额 / 刷新 token / 配额查询

use crate::commands::account_models::clear_available_models_cache;
use crate::commands::common::{
    account_machine_id_or_new, calc_expires_at, ensure_account_machine_id, find_account_by_id,
    get_enterprise_usage, get_usage_by_account, get_usage_by_provider_with_machine_id,
    is_auth_error_message, lock_store, refresh_token_by_provider, save_store,
    update_account_status, RefreshResult,
};
use crate::core::account::Account;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SyncAccountResult {
    pub account: Account,
    pub warning: Option<String>,
}

#[tauri::command]
pub async fn sync_account(
    state: State<'_, AppState>,
    id: String,
) -> Result<SyncAccountResult, String> {
    let mut account = find_account_by_id(&state, &id)?;

    let provider_str = account.provider.as_deref().unwrap_or("Google");
    let access_token = account.access_token.clone().ok_or("No access token")?;
    let is_enterprise = provider_str == "Enterprise";

    // 如果账号缺少 machine_id，自动生成账号独立 ID（所有账号都需要）
    if account
        .machine_id
        .as_ref()
        .is_none_or(|id| id.trim().is_empty())
    {
        ensure_account_machine_id(&mut account);
        log::info!(
            "Generated account-scoped machine_id for account: {}",
            account.id
        );
    }

    // 先尝试用现有 token 获取配额
    let mut usage_result = if is_enterprise {
        let machine_id = account
            .machine_id
            .as_ref()
            .ok_or("Enterprise account missing machine_id")?;
        get_enterprise_usage(&access_token, machine_id).await
    } else {
        get_usage_by_account(&account, &access_token).await
    };

    let mut refresh_result: Option<RefreshResult> = None;

    // 如果是认证错误，刷新 token 后重试
    let needs_refresh = match &usage_result {
        Ok(r) => r.is_auth_error,
        Err(_) => false,
    };

    if needs_refresh {
        match refresh_token_by_provider(&account).await {
            Ok(refreshed) => {
                usage_result = if is_enterprise {
                    let machine_id = account
                        .machine_id
                        .as_ref()
                        .ok_or("Enterprise account missing machine_id")?;
                    get_enterprise_usage(&refreshed.access_token, machine_id).await
                } else {
                    get_usage_by_account(&account, &refreshed.access_token).await
                };
                refresh_result = Some(refreshed);
            }
            Err(e) => {
                if e.starts_with("BANNED:") || is_auth_error_message(&e) {
                    let mut store = lock_store(&state.store, "store")?;
                    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                        a.status = if e.starts_with("BANNED:") {
                            "banned".to_string()
                        } else {
                            "invalid".to_string()
                        };
                        a.enabled = false;
                        save_store(&store)?;
                    }
                }
                return Err(e);
            }
        }
    }

    // 获取配额失败时容错处理：只更新 token，不更新 usageData
    let (usage, warning) = match usage_result {
        Ok(u) => (Some(u), None),
        Err(e) => {
            // 获取配额失败，不打印日志，直接返回错误信息
            (None, Some(format!("获取配额失败: {e}")))
        }
    };

    let mut store = lock_store(&state.store, "store")?;
    let result = if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        // 如果生成了新的 machine_id，保存它（所有账号都需要）
        if account.machine_id.is_some()
            && a.machine_id.as_ref().is_none_or(|id| id.trim().is_empty())
        {
            a.machine_id = account.machine_id.clone();
            log::info!("Saved account-scoped machine_id for account: {}", a.id);
        }

        // 如果刷新了 token，更新 token 相关字段
        if let Some(ref result) = refresh_result {
            clear_available_models_cache(a);

            let email_display = a
                .email
                .as_deref()
                .or(a.user_id.as_deref())
                .unwrap_or("Unknown");

            // 刷新 Token 成功，更新账号信息
            a.access_token = Some(result.access_token.clone());
            if let Some(ref refresh_token) = result.refresh_token {
                a.refresh_token = Some(refresh_token.clone());
            }
            a.profile_arn = result.profile_arn.clone();
            a.id_token = result.id_token.clone();
            a.sso_session_id = result.sso_session_id.clone();
            a.expires_at = Some(calc_expires_at(result.expires_in));

            log::info!(
                "Token refreshed successfully for account: {}",
                email_display
            );
        }

        // 只有成功获取配额时才更新 usage_data 和 status
        if let Some(usage_data) = usage {
            // 直接移动所有权，避免 clone
            a.usage_data = Some(usage_data.usage_data);
            update_account_status(a, usage_data.is_banned, usage_data.is_auth_error);

            // 从 usage_data 中提取并更新 email 和 user_id
            if let Some(user_info) = a.usage_data.as_ref().and_then(|d| d.get("userInfo")) {
                if let Some(email) = user_info.get("email").and_then(|v| v.as_str()) {
                    if !email.is_empty() {
                        a.email = Some(email.to_string());
                    }
                }
                if let Some(user_id) = user_info.get("userId").and_then(|v| v.as_str()) {
                    a.user_id = Some(user_id.to_string());
                }
            }
        } else if refresh_result.is_some() {
            // 获取配额失败，但 token 刷新成功了，说明 token 是有效的
            // 将状态设置为 active（避免显示为失效状态）
            if !matches!(a.status.as_str(), "banned" | "封禁" | "已封禁") {
                a.status = "active".to_string();
            }
        }

        // 克隆结果（这个必须 clone，因为要返回给前端）
        Some(a.clone())
    } else {
        None
    };

    // 保存文件
    save_store(&store)?;

    match result {
        Some(account) => Ok(SyncAccountResult { account, warning }),
        None => Err("Account not found after update".to_string()),
    }
}

/// 只获取配额，不刷新 token（用于手动刷新配额）
/// 如果 token 无效（401/403），直接返回错误，不会自动刷新 token
#[tauri::command]
pub async fn get_usage_limits(
    state: State<'_, AppState>,
    id: String,
) -> Result<SyncAccountResult, String> {
    let mut account = find_account_by_id(&state, &id)?;

    let provider_str = account.provider.as_deref().unwrap_or("Google");
    let access_token = account.access_token.clone().ok_or("No access token")?;
    let is_enterprise = provider_str == "Enterprise";

    // 如果账号缺少 machine_id，自动生成账号独立 ID（所有账号都需要）
    if account
        .machine_id
        .as_ref()
        .is_none_or(|id| id.trim().is_empty())
    {
        ensure_account_machine_id(&mut account);
        log::info!(
            "Generated account-scoped machine_id for account: {}",
            account.id
        );
    }

    // 直接获取配额，不自动刷新 token
    let usage_result = if is_enterprise {
        let machine_id = account
            .machine_id
            .as_ref()
            .ok_or("Enterprise account missing machine_id")?;
        log::info!("[Enterprise] Fetching usage for account {} with machine_id: {}", account.id, machine_id);
        get_enterprise_usage(&access_token, machine_id).await
    } else {
        get_usage_by_account(&account, &access_token).await
    };

    // 如果是认证错误，直接返回错误，不刷新 token
    let usage = match usage_result {
        Ok(u) => {
            if u.is_auth_error {
                return Err("Token 已失效，请刷新 token".to_string());
            }
            u
        }
        Err(e) => {
            return Err(format!("获取配额失败: {e}"));
        }
    };

    let mut store = lock_store(&state.store, "store")?;
    let result = if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        // 如果生成了新的 machine_id，保存它（所有账号都需要）
        if account.machine_id.is_some()
            && a.machine_id.as_ref().is_none_or(|id| id.trim().is_empty())
        {
            a.machine_id = account.machine_id.clone();
            log::info!("Saved account-scoped machine_id for account: {}", a.id);
        }

        // 更新 usage_data 和 status
        a.usage_data = Some(usage.usage_data);
        update_account_status(a, usage.is_banned, usage.is_auth_error);

        // 从 usage_data 中提取并更新 email 和 user_id
        if let Some(user_info) = a.usage_data.as_ref().and_then(|d| d.get("userInfo")) {
            if let Some(email) = user_info.get("email").and_then(|v| v.as_str()) {
                if !email.is_empty() {
                    a.email = Some(email.to_string());
                }
            }
            if let Some(user_id) = user_info.get("userId").and_then(|v| v.as_str()) {
                a.user_id = Some(user_id.to_string());
            }
        }

        // 克隆结果（这个必须 clone，因为要返回给前端）
        Some(a.clone())
    } else {
        None
    };

    // 保存文件
    save_store(&store)?;

    match result {
        Some(account) => Ok(SyncAccountResult {
            account,
            warning: None,
        }),
        None => Err("Account not found after update".to_string()),
    }
}

/// 只刷新 token，不获取 usage（启动时快速刷新用）
/// 如果 token 还有 5 分钟以上有效期，跳过刷新直接返回
#[tauri::command]
pub async fn refresh_token(
    state: State<'_, AppState>,
    id: String,
) -> Result<Account, String> {
    let mut account = find_account_by_id(&state, &id)?;
    let generated_machine_id = if account
        .machine_id
        .as_ref()
        .is_none_or(|id| id.trim().is_empty())
    {
        Some(ensure_account_machine_id(&mut account))
    } else {
        None
    };
    if let Some(ref machine_id) = generated_machine_id {
        let mut store = lock_store(&state.store, "store")?;
        if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
            if a.machine_id.as_ref().is_none_or(|id| id.trim().is_empty()) {
                a.machine_id = Some(machine_id.clone());
                save_store(&store)?;
            }
        }
    }

    // 检查 token 是否还有 5 分钟以上有效期
    if let Some(expires_at) = &account.expires_at {
        if let Ok(exp) = chrono::NaiveDateTime::parse_from_str(expires_at, "%Y/%m/%d %H:%M:%S") {
            let now = chrono::Local::now().naive_local();
            let remaining = exp.signed_duration_since(now);
            if remaining.num_minutes() >= 5 {
                return Ok(account);
            }
        }
    }

    let refresh_result = match refresh_token_by_provider(&account).await {
        Ok(result) => result,
        Err(e) => {
            if e.starts_with("BANNED:") || is_auth_error_message(&e) {
                let mut store = lock_store(&state.store, "store")?;
                if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                    a.status = if e.starts_with("BANNED:") {
                        "banned".to_string()
                    } else {
                        "invalid".to_string()
                    };
                    a.enabled = false;
                    save_store(&store)?;
                }
            }
            return Err(e);
        }
    };

    let mut store = lock_store(&state.store, "store")?;
    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        clear_available_models_cache(a);
        if a.machine_id.as_ref().is_none_or(|id| id.trim().is_empty()) {
            a.machine_id = generated_machine_id;
        }
        // 直接移动所有权，避免 clone
        a.access_token = Some(refresh_result.access_token);
        a.refresh_token = refresh_result.refresh_token;
        a.expires_at = Some(calc_expires_at(refresh_result.expires_in));
        if matches!(
            a.status.as_str(),
            "invalid" | "失效" | "已失效" | "Token已失效"
        ) {
            a.status = "active".to_string();
        }
        let result = a.clone();
        save_store(&store)?;
        return Ok(result);
    }
    Err("Account not found after update".to_string())
}

/// 获取账号配额信息（不刷新 token，不更新数据库）
#[tauri::command]
pub async fn get_account_usage(
    access_token: String,
    provider: Option<String>,
    machine_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let provider_str = provider.as_deref().unwrap_or("Google");
    let account_machine_id = account_machine_id_or_new(&machine_id);
    let usage_result =
        get_usage_by_provider_with_machine_id(provider_str, &access_token, &account_machine_id)
            .await?;
    Ok(usage_result.usage_data)
}
