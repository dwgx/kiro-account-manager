#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数

use super::mapping::{
    build_account_identity, create_account_label, determine_provider, find_existing_account,
    resolve_effective_start_url,
};
use super::shared::{expand_home_dir, lock_account_store};
use crate::commands::common::{
    extract_user_info, generate_account_machine_id, get_usage_by_provider_with_machine_id,
};
use crate::core::account::Account;
use crate::kiro::cli::read_kiro_cli_accounts;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct KiroCliImportResult {
    pub success: bool,
    pub is_new: bool,
    pub account: Option<Account>,
    pub error: Option<String>,
}

/// 获取 kiro-cli 默认数据库路径
#[tauri::command]
pub fn get_kiro_cli_default_path() -> Result<String, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "无法获取用户主目录".to_string())?;

    let mut candidates = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.push(
            std::path::PathBuf::from(&home)
                .join("Library")
                .join("Application Support")
                .join("kiro-cli")
                .join("data.sqlite3"),
        );
    } else if cfg!(target_os = "windows") {
        // Kiro CLI 2.0 原生支持 Windows
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidates.push(
                std::path::PathBuf::from(local_app_data)
                    .join("Kiro-Cli")
                    .join("data.sqlite3"),
            );
        }
    } else {
        if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
            candidates.push(
                std::path::PathBuf::from(xdg_data_home)
                    .join("kiro-cli")
                    .join("data.sqlite3"),
            );
        }
        candidates.push(
            std::path::PathBuf::from(&home)
                .join(".local")
                .join("share")
                .join("kiro-cli")
                .join("data.sqlite3"),
        );
    }

    for path in candidates {
        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    // 文件不存在，返回空字符串（前端会显示占位符）
    Ok(String::new())
}
/// 从 kiro-cli 数据库导入账号
#[tauri::command]
pub async fn import_from_kiro_cli(
    db_path: String,
    state: State<'_, AppState>,
) -> Result<KiroCliImportResult, String> {
    eprintln!("[Kiro CLI Import] 开始导入，数据库路径: {db_path}");

    // 展开 ~ 为用户主目录
    let expanded_path = expand_home_dir(&db_path)?;
    eprintln!("[Kiro CLI Import] 展开后的路径: {expanded_path}");

    // 1. 读取 kiro-cli 数据库
    let cli_accounts = read_kiro_cli_accounts(&expanded_path)?;

    if cli_accounts.is_empty() {
        return Err("数据库中没有账号数据".to_string());
    }

    if cli_accounts.len() > 1 {
        return Err("数据库中有多个账号，请联系开发者".to_string());
    }

    let cli_account = &cli_accounts[0];
    let auth_method = &cli_account.auth_method;
    let token_key = &cli_account.token_key;
    eprintln!("[Kiro CLI Import] 读取到账号: auth_method={auth_method}, token_key={token_key}");

    // 2. 调用统一的 getUsageLimits API 获取配额
    // 先解析有效 start_url（含 JWT 兜底），provider 判定必须基于它，否则顶层不带
    // start_url 的 Enterprise token 会被误判成 BuilderId，绕过 #119 硬校验。
    let effective_start_url = resolve_effective_start_url(cli_account);
    let provider = determine_provider(cli_account, effective_start_url.as_deref());
    let account_machine_id = {
        let store = lock_account_store(&state.store)?;
        store
            .accounts
            .iter()
            .find(|account| account.refresh_token.as_ref() == Some(&cli_account.refresh_token))
            .and_then(|account| {
                account
                    .machine_id
                    .clone()
                    .filter(|id| !id.trim().is_empty())
            })
            .unwrap_or_else(generate_account_machine_id)
    };
    let usage_result = get_usage_by_provider_with_machine_id(
        &provider,
        &cli_account.access_token,
        &account_machine_id,
    )
    .await;

    let (email, user_id, usage_data, is_banned, is_auth_error) = match usage_result {
        Ok(result) => {
            let (email, user_id) = extract_user_info(&result.usage_data);
            // 用 log:: 而非 eprintln!，让身份提取结果落进 app.log，便于事后排查
            // （Enterprise 配额 API 常返回 userInfo:null，email/userId 均为 None 属正常）。
            log::info!(
                "[Kiro CLI Import] 身份提取: provider={provider}, email={email:?}, user_id={user_id:?}"
            );
            (
                email,
                user_id,
                Some(result.usage_data),
                result.is_banned,
                result.is_auth_error,
            )
        }
        Err(e) => {
            eprintln!("[Kiro CLI Import] 获取配额失败: {e}");
            return Ok(KiroCliImportResult {
                success: false,
                is_new: false,
                account: None,
                error: Some(format!("获取账号信息失败: {e}")),
            });
        }
    };

    // 3. 检查账号是否已存在
    let mut store = lock_account_store(&state.store)?;
    let existing_index = find_existing_account(
        &store.accounts,
        user_id.as_ref(),
        email.as_ref(),
        &provider,
        &cli_account.refresh_token,
    );
    let is_new = existing_index.is_none();

    // 4. 创建或更新 Account
    let existing_account = existing_index.and_then(|idx| store.accounts.get(idx));
    let label = create_account_label(is_new, &cli_account.token_key, existing_account);

    let mut account = build_account_identity(email.as_deref(), user_id.as_deref(), &provider, label);

    // 5. 填充字段
    account.access_token = Some(cli_account.access_token.clone());
    account.refresh_token = Some(cli_account.refresh_token.clone());
    // kiro-cli DB 里的 expires_at 是 RFC3339（UTC），内部过期判断只认 %Y/%m/%d %H:%M:%S。
    // 原样拷贝会让导入账号永远被判为已过期、每次访问强制刷新，这里统一规范化。
    // 无法解析时留空，交由 token 刷新流程按需重建，而非写入必然被判过期的脏格式。
    account.expires_at = cli_account
        .expires_at
        .as_deref()
        .and_then(crate::commands::common::normalize_expires_at);
    account.provider = Some(provider.clone());
    account.user_id = user_id;
    account.region = Some(cli_account.region.clone());
    account.usage_data = usage_data;

    // 更新账号状态（包括封禁检测）
    crate::commands::common::update_account_status(&mut account, is_banned, is_auth_error);

    // 6. 根据认证类型填充字段
    if cli_account.auth_method == "social" {
        account.auth_method = Some("social".to_string());
        account.profile_arn.clone_from(&cli_account.profile_arn);
    } else {
        account.auth_method = Some("IdC".to_string());
        account.client_id.clone_from(&cli_account.client_id);
        account.client_secret.clone_from(&cli_account.client_secret);

        // start_url：复用 provider 判定时已解析好的有效值（token 顶层 → clientSecret JWT
        // 兜底，已 normalize 去尾斜杠）。切回 IDE 时靠它算正确的 clientIdHash —— Enterprise
        // 必须是自己的 d-xxx 域名。与 provider 判定同源，避免两处解析漂移（issue #119）。
        let start_url = effective_start_url.clone();

        // clientIdHash：走统一裁决点，Enterprise 在此硬校验（issue #119）。导入数据被污染
        // （Enterprise 缺 start_url / 落到 BuilderId 默认值）时直接返回结构化失败，不写坏数据。
        let client_id_hash = match crate::commands::common::resolve_idc_client_id_hash(
            &provider,
            None,
            start_url.as_deref(),
        ) {
            Ok(hash) => hash,
            Err(e) => {
                return Ok(KiroCliImportResult {
                    success: false,
                    is_new: false,
                    account: None,
                    error: Some(format!("解析 clientIdHash 失败: {e}")),
                });
            }
        };
        account.client_id_hash = Some(client_id_hash);
        account.start_url = start_url;
    }

    // 7. 生成或保留 machine_id
    if let Some(idx) = existing_index {
        // 更新现有账号，保留 machine_id；历史空值则回填本次导入使用的账号级 ID
        account
            .machine_id
            .clone_from(&store.accounts[idx].machine_id);
        if account
            .machine_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
        {
            account.machine_id = Some(account_machine_id);
        }
        account.id.clone_from(&store.accounts[idx].id);
        store.accounts[idx] = account.clone();
    } else {
        // 新账号，保存本次 usage 检测使用的账号级 machine_id
        account.machine_id = Some(account_machine_id);
        store.accounts.push(account.clone());
    }

    store.save_to_file();
    drop(store);

    let email = &account.email;
    let user_id = &account.user_id;
    eprintln!("[Kiro CLI Import] 导入成功: is_new={is_new}, email={email:?}, user_id={user_id:?}");

    Ok(KiroCliImportResult {
        success: true,
        is_new,
        account: Some(account),
        error: None,
    })
}
