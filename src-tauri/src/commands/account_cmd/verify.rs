// 凭据校验

use crate::auth::providers::{AuthProvider, IdcProvider, RefreshMetadata};
use crate::auth::refresh_token_desktop;
use crate::commands::common::{
    ensure_account_machine_id, get_usage_by_account, lock_store, save_store, update_account_status,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyAccountResponse {
    #[serde(rename = "usageData")]
    pub usage_data: serde_json::Value, // 直接返回原始数据，前端解析
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

/// `verify_account` 命令参数
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyAccountParams {
    #[allow(dead_code)]
    pub access_token: String,
    pub refresh_token: String,
    pub provider: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: Option<String>,
}

#[tauri::command]
pub async fn verify_account(
    state: State<'_, AppState>,
    params: VerifyAccountParams,
) -> Result<VerifyAccountResponse, String> {
    let VerifyAccountParams {
        access_token: _,
        refresh_token,
        provider,
        client_id,
        client_secret,
        region,
    } = params;

    let is_idc = provider == "BuilderId" || provider == "Enterprise";

    // 刷新 token
    let (new_access_token, new_refresh_token) = if is_idc {
        let (cid, csec, reg) = if client_id.is_some() && client_secret.is_some() {
            (client_id, client_secret, region)
        } else {
            let store = lock_store(&state.store, "store")?;
            store
                .accounts
                .iter()
                .find(|a| a.refresh_token.as_ref() == Some(&refresh_token))
                .map_or((None, None, None), |a| {
                    (
                        a.client_id.clone(),
                        a.client_secret.clone(),
                        a.region.clone(),
                    )
                })
        };

        let cid = cid.ok_or("IdC 账号缺少 client_id，请重新添加账号")?;
        let csec = csec.ok_or("IdC 账号缺少 client_secret，请重新添加账号")?;
        let metadata = RefreshMetadata {
            client_id: Some(cid),
            client_secret: Some(csec),
            region: reg.clone(),
            ..Default::default()
        };

        let idc_provider = IdcProvider::new(&provider, reg.as_deref().unwrap_or("us-east-1"), None);
        let auth = idc_provider.refresh_token(&refresh_token, metadata).await?;
        (auth.access_token, auth.refresh_token)
    } else {
        let auth = refresh_token_desktop(&refresh_token).await?;
        (auth.access_token, auth.refresh_token)
    };

    // 获取 usage_data（使用统一的 getUsageLimits 接口）
    let temp_account = {
        let store = lock_store(&state.store, "store")?;
        let account = store
            .accounts
            .iter()
            .find(|a| a.refresh_token.as_ref() == Some(&refresh_token))
            .ok_or("Account not found")?;

        let mut temp_account = account.clone();
        temp_account.access_token = Some(new_access_token.clone());
        ensure_account_machine_id(&mut temp_account);
        temp_account
    }; // MutexGuard 在这里被释放

    let usage_result = match get_usage_by_account(&temp_account, &new_access_token).await {
        Ok(result) => result,
        Err(e) => {
            log::warn!("Failed to get usage in verify_account: {}", e);
            // 即使 getUsageLimits 失败，也能更新账号
            crate::commands::common::UsageResult {
                usage_data: serde_json::json!({}),
                is_banned: false,
                is_auth_error: false,
            }
        }
    };
    let usage_data = usage_result.usage_data.clone();

    // 更新数据库（包括状态）
    {
        let mut store = lock_store(&state.store, "store")?;
        if let Some(account) = store
            .accounts
            .iter_mut()
            .find(|a| a.refresh_token.as_ref() == Some(&refresh_token))
        {
            // 更新 token
            account.access_token = Some(new_access_token.clone()); // ✅ 这里必须 clone，因为后面还要用
            account.refresh_token = Some(new_refresh_token.clone()); // ✅ 这里必须 clone，因为后面还要用
            if account
                .machine_id
                .as_ref()
                .is_none_or(|id| id.trim().is_empty())
            {
                account.machine_id = temp_account.machine_id.clone();
            }
            // 更新 usage_data 和状态（检测封禁）
            account.usage_data = Some(usage_result.usage_data);
            update_account_status(account, usage_result.is_banned, usage_result.is_auth_error);
            save_store(&store)?;
        }
    }

    Ok(VerifyAccountResponse {
        usage_data, // 直接返回，前端解析
        access_token: new_access_token,
        refresh_token: new_refresh_token,
    })
}
