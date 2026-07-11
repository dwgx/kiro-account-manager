// 添加 external_idp (微软 / Azure AD) 账号

use super::AddAccountResult;
use crate::commands::common::{
    extract_user_info, find_existing_account_idx, generate_account_machine_id,
    get_usage_by_account, lock_store, normalize_expires_at, save_store, update_account_status,
};
use crate::core::account::Account;
use crate::state::AppState;
use tauri::State;

/// 添加 external_idp (微软 / Azure AD) 账号（PKCE 公共客户端，通常无 client_secret）
///
/// external_idp 号的特征：有 clientId、可能无 clientSecret、带微软 tokenEndpoint/issuerUrl，
/// profileArn 必须原样保留（绝不套 IdC/social 默认占位）。当前后端无 M365 刷新链路，
/// 导入时用传入的 accessToken 取 usage；取不到不 hard-fail，账号仍落库。
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri IPC 命令签名需要显式参数，避免前后端调用契约破坏
pub async fn add_account_by_external_idp(
    state: State<'_, AppState>,
    refresh_token: String,
    client_id: String,
    profile_arn: String,
    client_secret: Option<String>,
    access_token: Option<String>,
    token_endpoint: Option<String>,
    issuer_url: Option<String>,
    scopes: Option<String>,
    region: Option<String>,
    machine_id: Option<String>,
    email: Option<String>,
    expires_at: Option<String>,
) -> Result<AddAccountResult, String> {
    // machine_id：账号自带（非空）→ 否则生成账号独立 ID
    let account_machine_id = machine_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(generate_account_machine_id);

    // region：缺省 us-east-1
    let region = region.unwrap_or_else(|| "us-east-1".to_string());

    // 取 usage（不刷新微软 token —— 无 M365 刷新路径）
    // 构造临时 Account 用于取 usage：provider=None + 真实 profile_arn，
    // resolve_kiro_call_context 会原样保留真实 arn，不套占位。
    let usage_result = match access_token.as_deref() {
        Some(at) if !at.trim().is_empty() => {
            let mut probe = Account::new(String::new(), String::new());
            probe.provider = None;
            probe.auth_method = Some("external_idp".to_string());
            probe.profile_arn = Some(profile_arn.clone());
            probe.machine_id = Some(account_machine_id.clone());
            probe.region = Some(region.clone());
            match get_usage_by_account(&probe, at).await {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("[external_idp] 取 usage 失败: {e}");
                    crate::commands::common::UsageResult {
                        usage_data: serde_json::json!({}),
                        is_banned: false,
                        is_auth_error: false,
                    }
                }
            }
        }
        // 无 access_token 不 hard-fail，账号仍落库（usage 置空）
        _ => crate::commands::common::UsageResult {
            usage_data: serde_json::json!({}),
            is_banned: false,
            is_auth_error: false,
        },
    };

    // 封禁账号直接报错（与 social / idc 一致）
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // email：入参 > usage 提取 email > userId > refreshToken 前缀标识
    let final_email = email
        .filter(|e| !e.trim().is_empty())
        .or(new_email)
        .or(user_id.clone())
        .unwrap_or_else(|| {
            format!(
                "external_idp_{}",
                &refresh_token[..refresh_token.len().min(8)]
            )
        });

    // expires_at 归一（可选；解析失败返回 None，不写脏值）
    let final_expires_at = expires_at.as_deref().and_then(normalize_expires_at);

    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        Some(&final_email),
        "external_idp",
        &refresh_token,
        user_id.as_ref(),
    );

    let is_new = existing_idx.is_none();

    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        existing.access_token = access_token.clone();
        existing.refresh_token = Some(refresh_token.clone());
        existing.provider = None; // ★ 不设 provider（保证 profile_arn 原样透传）
        existing.auth_method = Some("external_idp".to_string());
        existing.client_id = Some(client_id.clone());
        existing.client_secret = client_secret.clone();
        existing.profile_arn = Some(profile_arn.clone()); // ★ 原样，绝不套占位
        existing.token_endpoint = token_endpoint.clone();
        existing.issuer_url = issuer_url.clone();
        existing.scopes = scopes.clone();
        existing.region = Some(region.clone());
        existing.email = Some(final_email.clone());
        existing.user_id = user_id.clone();
        existing.usage_data = Some(usage_result.usage_data);
        if let Some(exp) = final_expires_at {
            existing.expires_at = Some(exp);
        }
        if existing
            .machine_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
        {
            existing.machine_id = Some(account_machine_id.clone());
        }
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone()
    } else {
        let mut account = Account::new(final_email.clone(), "Kiro external_idp 账号".to_string());
        account.access_token = access_token.clone();
        account.refresh_token = Some(refresh_token.clone());
        account.provider = None; // ★ 不设 provider
        account.auth_method = Some("external_idp".to_string());
        account.client_id = Some(client_id.clone());
        account.client_secret = client_secret.clone();
        account.profile_arn = Some(profile_arn.clone()); // ★ 原样，绝不套占位
        account.token_endpoint = token_endpoint.clone();
        account.issuer_url = issuer_url.clone();
        account.scopes = scopes.clone();
        account.region = Some(region.clone());
        account.user_id = user_id.clone();
        account.usage_data = Some(usage_result.usage_data);
        if let Some(exp) = final_expires_at {
            account.expires_at = Some(exp);
        }
        account.machine_id = Some(account_machine_id.clone());
        update_account_status(
            &mut account,
            usage_result.is_banned,
            usage_result.is_auth_error,
        );
        store.accounts.insert(0, account.clone());
        account
    };

    // ★ 不写 state.auth.user/access_token —— 导入不改登录态（对齐 idc）
    save_store(&store)?;
    drop(store);

    Ok(AddAccountResult { account, is_new })
}
