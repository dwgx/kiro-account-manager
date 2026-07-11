// 账号 CRUD / 导入导出 / 远程删除 / 筛选查询

use crate::commands::account_models::clear_available_models_cache;
use crate::commands::common::{
    account_machine_id_or_new, ensure_account_machine_id, find_account_by_id, lock_store,
    save_store,
};
use crate::core::account::{Account, AccountProxyConfig};
use crate::state::AppState;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountParams {
    pub id: String,
    pub label: Option<String>,
    pub status: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub machine_id: Option<String>,
    pub added_at: Option<String>,
    pub expires_at: Option<String>,
    pub enabled: Option<bool>,
    pub proxy_config: Option<AccountProxyConfig>,
}

#[tauri::command]
pub fn get_accounts(state: State<AppState>) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(mut store) => {
            // 每次获取前重新从文件加载，确保数据最新
            store.reload();
            store.get_all()
        }
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}

#[tauri::command]
pub fn delete_account(state: State<AppState>, id: &str) -> bool {
    match lock_store(&state.store, "store") {
        Ok(mut store) => store.delete(id).unwrap_or_else(|err| {
            eprintln!("[account_cmd] {err}");
            false
        }),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            false
        }
    }
}

#[tauri::command]
pub fn delete_accounts(state: State<AppState>, ids: Vec<String>) -> usize {
    match lock_store(&state.store, "store") {
        Ok(mut store) => store.delete_many(&ids).unwrap_or_else(|err| {
            eprintln!("[account_cmd] {err}");
            0
        }),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            0
        }
    }
}

#[tauri::command]
pub fn import_accounts(state: State<AppState>, json: &str) -> Result<usize, String> {
    let mut store = lock_store(&state.store, "store")?;
    store.import_from_json(json)
}

#[tauri::command]
pub fn export_accounts(state: State<AppState>, ids: Option<Vec<String>>) -> String {
    let store = match lock_store(&state.store, "store") {
        Ok(store) => store,
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            return "[]".to_string();
        }
    };

    // 修复账号数据
    let fix_account = |mut account: Account| -> Account {
        // 1. 修复 provider 为 null
        if account.provider.is_none() && account.auth_method.as_deref() == Some("IdC") {
            // IdC 账号但 provider 为 null，根据 start_url 或 client_secret 判断
            if let Some(ref start_url) = account.start_url {
                if start_url.contains("awsapps.com") {
                    account.provider = Some("Enterprise".to_string());
                } else {
                    account.provider = Some("BuilderId".to_string());
                }
            } else if let Some(ref client_secret) = account.client_secret {
                if client_secret.contains("initiateLoginUri") {
                    account.provider = Some("Enterprise".to_string());
                } else {
                    account.provider = Some("BuilderId".to_string());
                }
            } else {
                // 默认 BuilderId
                account.provider = Some("BuilderId".to_string());
            }
        } else if account.provider.is_none() && account.auth_method.as_deref() == Some("social") {
            // Social 账号但 provider 为 null，根据邮箱判断
            if let Some(ref email) = account.email {
                if email.contains("gmail") {
                    account.provider = Some("Google".to_string());
                } else if email.contains("github") {
                    account.provider = Some("Github".to_string());
                } else {
                    account.provider = Some("Google".to_string());
                }
            } else {
                account.provider = Some("Google".to_string());
            }
        }

        // 2. 修复 authMethod 为 null
        if account.auth_method.is_none() {
            if account.client_id.is_some() && account.client_secret.is_some() {
                account.auth_method = Some("IdC".to_string());
            } else {
                account.auth_method = Some("social".to_string());
            }
        }

        account
    };

    match ids {
        Some(id_list) if !id_list.is_empty() => {
            // 导出选中的账号
            let selected: Vec<Account> = store
                .accounts
                .iter()
                .filter(|a| id_list.contains(&a.id))
                .cloned()
                .map(fix_account)
                .collect();
            serde_json::to_string_pretty(&selected).unwrap_or_else(|_| "[]".to_string())
        }
        _ => {
            // 没有选中任何账号，返回空数组
            "[]".to_string()
        }
    }
}

/// 更新账号信息（支持修改 label、token、SSO Client ID/Secret、machineId）
#[tauri::command]
pub fn update_account(
    state: State<AppState>,
    params: UpdateAccountParams,
) -> Result<Account, String> {
    let mut store = lock_store(&state.store, "store")?;

    // 先找到索引，避免借用冲突
    let idx = store.accounts.iter().position(|a| a.id == params.id);

    if let Some(idx) = idx {
        if let Some(l) = params.label {
            store.accounts[idx].label = l;
        }
        if let Some(status) = params.status {
            store.accounts[idx].status = status;
        }
        if let Some(at) = params.access_token {
            store.accounts[idx].access_token = Some(at);
        }
        if let Some(rt) = params.refresh_token {
            store.accounts[idx].refresh_token = Some(rt);
        }
        // BuilderId SSO 字段
        if let Some(cid) = params.client_id {
            store.accounts[idx].client_id = Some(cid);
        }
        if let Some(csec) = params.client_secret {
            store.accounts[idx].client_secret = Some(csec);
        }
        // 机器码
        if let Some(mid) = params.machine_id {
            store.accounts[idx].machine_id = Some(mid);
        }
        if let Some(added_at) = params.added_at {
            let trimmed = added_at.trim();
            if !trimmed.is_empty() {
                store.accounts[idx].added_at = trimmed.to_string();
            }
        }
        if let Some(expires_at) = params.expires_at {
            let trimmed = expires_at.trim();
            store.accounts[idx].expires_at = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        // 启用/禁用
        if let Some(enabled) = params.enabled {
            store.accounts[idx].enabled = enabled;
        }
        if let Some(proxy_config) = params.proxy_config {
            let has_proxy_values = !proxy_config.host.trim().is_empty()
                || proxy_config.port > 0
                || proxy_config
                    .username
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                || proxy_config
                    .password
                    .as_deref()
                    .is_some_and(|value| !value.is_empty());
            let next_proxy_config = if proxy_config.enabled || has_proxy_values {
                Some(proxy_config)
            } else {
                None
            };
            if store.accounts[idx].proxy_config != next_proxy_config {
                store.accounts[idx].proxy_config = next_proxy_config;
                clear_available_models_cache(&mut store.accounts[idx]);
            }
        }
        let result = store.accounts[idx].clone();
        save_store(&store)?;
        Ok(result)
    } else {
        Err("账号不存在".to_string())
    }
}

/// 从 AWS 服务端删除账号（注销账号）
/// 仅支持 Google、Github，不支持 `BuilderId` 和 `Enterprise`
#[tauri::command]
pub async fn delete_account_remote(
    state: State<'_, AppState>,
    id: String,
    delete_local: bool,
) -> Result<String, String> {
    use crate::auth::delete_account_desktop;

    // 获取账号信息
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

    // 检查 provider
    let provider = account.provider.as_deref().unwrap_or("Google");
    if provider == "Enterprise" {
        return Err("Enterprise 账号不支持远程删除".to_string());
    }
    if provider == "BuilderId" {
        return Err("BuilderId 账号不支持远程删除".to_string());
    }

    let access_token = account
        .access_token
        .as_ref()
        .ok_or("账号缺少 access_token，请先刷新")?;

    // Google/Github 账号使用 Desktop API
    let machine_id = account_machine_id_or_new(&account.machine_id);
    delete_account_desktop(access_token, &machine_id).await?;

    // 如果需要同时删除本地记录
    if delete_local {
        let mut store = lock_store(&state.store, "store")?;
        store.delete(&id)?;
    }

    Ok(format!("账号 {} 已从服务端删除", account.get_display_id()))
}

// ============================================================
// 筛选查询命令
// ============================================================

/// 获取可用账号列表（用于自动换号）
#[tauri::command]
pub fn get_available_accounts(state: State<AppState>) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(store) => store
            .get_available_accounts()
            .into_iter()
            .cloned()
            .collect(),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}

/// 按分组筛选账号
#[tauri::command]
pub fn get_accounts_by_group(state: State<AppState>, group_id: String) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(store) => store
            .get_accounts_by_group(&group_id)
            .into_iter()
            .cloned()
            .collect(),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}

/// 按标签筛选账号
#[tauri::command]
pub fn get_accounts_by_tag(state: State<AppState>, tag_id: String) -> Vec<Account> {
    match lock_store(&state.store, "store") {
        Ok(store) => store
            .get_accounts_by_tag(&tag_id)
            .into_iter()
            .cloned()
            .collect(),
        Err(err) => {
            eprintln!("[account_cmd] {err}");
            Vec::new()
        }
    }
}
