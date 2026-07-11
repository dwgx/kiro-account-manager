// 超额开关（资格预检 + 过期刷新 + API 调用失败刷新重试 + 本地 usage_data 回写）

use super::apply_refreshed_account_tokens;
use crate::commands::common::{
    ensure_account_machine_id, find_account_by_id, is_auth_error_message, lock_store,
    refresh_token_by_provider, save_store, token_needs_refresh,
};
use crate::state::AppState;
use tauri::State;

/// 设置超额开关状态
#[tauri::command]
pub async fn set_overage_status(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    use crate::clients::kiro_client::KiroClient;
    use crate::core::usage::OverageCapability;

    // 1. 从 state 获取账号
    let mut account = find_account_by_id(&state, &id)?;
    let generated_machine_id = if account
        .machine_id
        .as_ref()
        .is_none_or(|machine_id| machine_id.trim().is_empty())
    {
        Some(ensure_account_machine_id(&mut account))
    } else {
        None
    };

    if let Some(machine_id) = generated_machine_id.as_ref() {
        let mut store = lock_store(&state.store, "store")?;
        if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
            if stored_account
                .machine_id
                .as_ref()
                .is_none_or(|stored_machine_id| stored_machine_id.trim().is_empty())
            {
                stored_account.machine_id = Some(machine_id.clone());
                save_store(&store)?;
            }
        }
    }

    // 资格预检：必须是 OVERAGE_CAPABLE 才能开关超额
    match OverageCapability::from_usage_data(account.usage_data.as_ref()) {
        OverageCapability::Capable => {}
        OverageCapability::Incapable => {
            return Err("此账号订阅级别不支持超额（仅 Pro / Pro+ 可开启）".to_string());
        }
        OverageCapability::Unknown => {
            return Err("无法判断账号超额资格，请先点击「检测」刷新账号信息".to_string());
        }
    }

    let access_token = account
        .access_token
        .as_ref()
        .ok_or("No access token")?
        .clone();

    // 2. 检查 token 是否过期，如果过期则刷新
    let final_access_token = if let Some(expires_at) = &account.expires_at {
        if token_needs_refresh(expires_at) {
            let refresh_result = refresh_token_by_provider(&account).await?;
            // 更新 store 中的 token
            let mut store = lock_store(&state.store, "store")?;
            if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                apply_refreshed_account_tokens(a, &refresh_result);
                save_store(&store)?;
            }
            refresh_result.access_token
        } else {
            access_token
        }
    } else {
        access_token
    };

    // 3. 解析 Kiro 调用上下文
    let ctx = crate::commands::common::resolve_kiro_call_context(&account, "us-east-1");

    // 4. 调用 API（失败后刷新重试）
    let overage_status = if enabled { "ENABLED" } else { "DISABLED" };
    let client = KiroClient::new()?;

    let result = client
        .set_user_preference(
            &final_access_token,
            &ctx.machine_id,
            &ctx.region,
            ctx.profile_arn.as_deref(),
            overage_status,
        )
        .await;

    // 如果首次调用失败且是认证错误，刷新 token 后重试
    if let Err(ref e) = result {
        if is_auth_error_message(e) {
            log::info!("[set_overage_status] Token 失效，尝试刷新后重试");
            let refresh_result = refresh_token_by_provider(&account).await?;

            // 更新 store 中的 token（在独立作用域内，确保锁在 await 之前释放）
            {
                let mut store = lock_store(&state.store, "store")?;
                if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
                    apply_refreshed_account_tokens(a, &refresh_result);
                    save_store(&store)?;
                }
            } // 锁在这里释放

            // 用新 token 重试
            client
                .set_user_preference(
                    &refresh_result.access_token,
                    &ctx.machine_id,
                    &ctx.region,
                    ctx.profile_arn.as_deref(),
                    overage_status,
                )
                .await?;
        } else {
            return Err(e.clone());
        }
    } else {
        result?;
    }

    // 5. 更新本地 usage_data 中的 overageConfiguration.overageStatus
    let mut store = lock_store(&state.store, "store")?;
    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == id) {
        if let Some(ref mut usage_data) = a.usage_data {
            if let Some(overage_config) = usage_data.get_mut("overageConfiguration") {
                if let Some(obj) = overage_config.as_object_mut() {
                    obj.insert(
                        "overageStatus".to_string(),
                        serde_json::Value::String(overage_status.to_string()),
                    );
                }
            } else {
                // 如果 overageConfiguration 不存在，创建它
                if let Some(obj) = usage_data.as_object_mut() {
                    obj.insert(
                        "overageConfiguration".to_string(),
                        serde_json::json!({
                            "overageStatus": overage_status
                        }),
                    );
                }
            }
        }
        save_store(&store)?;
    }

    Ok(())
}
