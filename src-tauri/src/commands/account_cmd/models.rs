// 列出账号可用模型（带缓存 + 401 刷新重试 + 封禁处理）

use super::apply_refreshed_account_tokens;
use crate::commands::account_models::{
    fetch_all_available_models, read_available_models_cache, write_available_models_cache,
    ListAvailableModelsResponse,
};
use crate::commands::common::{
    ensure_account_machine_id, find_account_by_id, is_auth_error_message, lock_store,
    refresh_token_by_provider, save_store,
};
use crate::state::AppState;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn list_available_models(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    id: String,
    force_refresh: Option<bool>,
) -> Result<ListAvailableModelsResponse, String> {
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

    if let Some(cached_response) =
        read_available_models_cache(&account, force_refresh.unwrap_or(false))
    {
        return Ok(cached_response);
    }

    let initial_access_token = account
        .access_token
        .clone()
        .ok_or("账号缺少 access_token，请先刷新 Token")?;

    match fetch_all_available_models(&account, &initial_access_token).await {
        Ok(result) => {
            let mut store = lock_store(&state.store, "store")?;
            if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                if stored_account
                    .profile_arn
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none()
                {
                    stored_account.profile_arn = result.resolved_profile_arn.clone();
                }
                write_available_models_cache(stored_account, &result.response)?;
                save_store(&store)?;
            }
            Ok(result.response)
        }
        Err(error) if is_auth_error_message(&error) => {
            let refresh = refresh_token_by_provider(&account).await?;
            apply_refreshed_account_tokens(&mut account, &refresh);

            {
                let mut store = lock_store(&state.store, "store")?;
                let stored_account = store
                    .accounts
                    .iter_mut()
                    .find(|item| item.id == id)
                    .ok_or("账号不存在")?;
                apply_refreshed_account_tokens(stored_account, &refresh);
                save_store(&store)?;
            }
            let result = fetch_all_available_models(&account, &refresh.access_token).await?;
            {
                let mut store = lock_store(&state.store, "store")?;
                if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                    if stored_account
                        .profile_arn
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none()
                    {
                        stored_account.profile_arn = result.resolved_profile_arn.clone();
                    }
                    write_available_models_cache(stored_account, &result.response)?;
                    save_store(&store)?;
                }
            }
            Ok(result.response)
        }
        Err(error) if error.starts_with("BANNED:") => {
            // 更新账号状态为封禁
            let mut store = lock_store(&state.store, "store")?;
            if let Some(stored_account) = store.accounts.iter_mut().find(|item| item.id == id) {
                stored_account.status = "banned".to_string();
                stored_account.enabled = false;
                save_store(&store)?;
                // 通知前端刷新账号列表
                let _ = app.emit("accounts-updated", ());
            }
            Err(error)
        }
        Err(error) => Err(error),
    }
}
