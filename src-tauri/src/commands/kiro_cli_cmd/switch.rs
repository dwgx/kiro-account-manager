#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数

use super::shared::{expand_home_dir, lock_account_store};
use crate::commands::common::{ensure_account_machine_id, get_usage_by_account};
use crate::core::account::Account;
use crate::state::AppState;
use crate::utils::client_id_hash::normalize_start_url;
use tauri::{Emitter, State};

// ============================================================
// CLI 2.0 切号功能
// ============================================================

/// 检测 CLI 2.0 安装状态
#[tauri::command]
pub fn check_cli_installation() -> crate::kiro::cli::CliInstallationInfo {
    crate::kiro::cli::check_cli_installation()
}

/// 读取 CLI 数据库快照（前端展示用）
#[tauri::command]
pub fn read_cli_db_snapshot(
    db_path: String,
) -> Result<crate::kiro::cli::KiroCliDbSnapshot, String> {
    let expanded_path = expand_home_dir(&db_path)?;
    crate::kiro::cli::read_cli_db_snapshot(&expanded_path)
}

/// 切号到 CLI 账号
#[tauri::command]
pub async fn switch_to_cli_account(
    account_id: String,
    db_path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<crate::kiro::cli::KiroCliWriteBackup, String> {
    let expanded_path = expand_home_dir(&db_path)?;

    // 1. 从 store 读取账号数据
    let account = {
        let store = lock_account_store(&state.store)?;
        store
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| format!("账号不存在: {account_id}"))?
    };

    // 2. 切号前刷新 token（确保写入的是有效 token）
    let refreshed_account = if account.refresh_token.is_some() {
        log::info!("[CLI Switch] 切号前刷新 token...");
        match crate::commands::common::refresh_token_by_provider(&account).await {
            Ok(refresh_result) => {
                log::info!("[CLI Switch] Token 刷新成功");
                // 更新 store 中的 token
                {
                    let mut store = lock_account_store(&state.store)?;
                    if let Some(a) = store.accounts.iter_mut().find(|a| a.id == account_id) {
                        crate::commands::common::apply_refreshed_account_tokens(a, &refresh_result);
                        let _ = crate::commands::common::save_store(&store);
                    }
                }
                // 构造刷新后的账号对象
                let mut updated = account.clone();
                updated.access_token = Some(refresh_result.access_token);
                updated.refresh_token = refresh_result.refresh_token;
                // 根据 expires_in 计算 expires_at
                let expires_at =
                    chrono::Utc::now() + chrono::Duration::seconds(refresh_result.expires_in);
                updated.expires_at = Some(expires_at.to_rfc3339());
                updated
            }
            Err(e) => {
                log::warn!("[CLI Switch] Token 刷新失败: {}, 使用现有 token", e);
                account
            }
        }
    } else {
        account
    };

    let mut refreshed_account = refreshed_account;
    let generated_machine_id = if refreshed_account
        .machine_id
        .as_ref()
        .is_none_or(|id| id.trim().is_empty())
    {
        Some(ensure_account_machine_id(&mut refreshed_account))
    } else {
        None
    };
    if let Some(machine_id) = generated_machine_id {
        let mut store = lock_account_store(&state.store)?;
        if let Some(a) = store.accounts.iter_mut().find(|a| a.id == account_id) {
            if a.machine_id.as_ref().is_none_or(|id| id.trim().is_empty()) {
                a.machine_id = Some(machine_id);
                let _ = crate::commands::common::save_store(&store);
            }
        }
    }

    // 3. 切号后立即获取配额检测封禁状态
    if refreshed_account.provider.is_none() {
        return Err("账号缺少 provider 字段".to_string());
    }
    let access_token = refreshed_account
        .access_token
        .as_ref()
        .ok_or("账号缺少 access_token")?;

    log::info!("[CLI Switch] 切号后检测账号状态...");
    match get_usage_by_account(&refreshed_account, access_token).await {
        Ok(usage_result) => {
            // 更新账号状态（包括封禁检测）
            let mut store = lock_account_store(&state.store)?;
            if let Some(a) = store.accounts.iter_mut().find(|a| a.id == account_id) {
                a.usage_data = Some(usage_result.usage_data);
                crate::commands::common::update_account_status(
                    a,
                    usage_result.is_banned,
                    usage_result.is_auth_error,
                );
                let _ = crate::commands::common::save_store(&store);

                // 通知前端刷新账号列表
                let _ = app.emit("accounts-updated", ());

                if usage_result.is_banned {
                    log::warn!("[CLI Switch] 检测到账号已封禁");
                    return Err("账号已被封禁，无法切换到 CLI".to_string());
                }
            }
        }
        Err(e) => {
            log::warn!("[CLI Switch] 获取配额失败: {}, 继续切号", e);
            // 获取配额失败不阻止切号，但记录警告
        }
    }

    // 4. 构造切号载荷
    let payload = build_switch_payload(&refreshed_account)?;

    // 5. 执行切号写入（包括清除旧 key）
    crate::kiro::cli::switch_cli_account(&expanded_path, &payload)
}

/// 回滚切号操作
#[tauri::command]
pub fn rollback_cli_switch(
    db_path: String,
    backup: crate::kiro::cli::KiroCliWriteBackup,
) -> Result<(), String> {
    let expanded_path = expand_home_dir(&db_path)?;
    crate::kiro::cli::rollback_cli_switch(&expanded_path, &backup)
}

/// 退出 CLI 2.0 当前登录态（清空所有 token key，切号的逆操作）。
///
/// 数据库不存在时视为本来就没登录，幂等返回 0。返回实际清掉的 token key 数量。
#[tauri::command]
pub fn logout_cli_account(db_path: String) -> Result<usize, String> {
    let expanded_path = expand_home_dir(&db_path)?;
    // 数据库文件不存在 = 没装/没登录 CLI，幂等返回 0，不报错
    if !std::path::Path::new(&expanded_path).exists() {
        return Ok(0);
    }
    crate::kiro::cli::logout_cli_account(&expanded_path)
}
/// 构造切号载荷（从 Account 转换为 CLI 2.0 格式）
fn build_switch_payload(
    account: &Account,
) -> Result<crate::kiro::cli::KiroCliSwitchPayload, String> {
    // 判断账号类型
    let provider = account.provider.as_ref().ok_or("账号缺少 provider 字段")?;
    let (token_key, device_reg_key, auth_method) = match provider.as_str() {
        // Enterprise 也是 IdC/SSO，写入 odic key（真实 kiro-cli 实测样本即来自 SSO 登录）
        "BuilderId" | "Enterprise" => (
            "kirocli:odic:token",
            "kirocli:odic:device-registration",
            "IdC",
        ),
        // "Unknown" 只可能来自 social 路径（determine_provider 的 IdC 分支只会返回
        // BuilderId/Enterprise，绝不返回 Unknown），即 profile_arn 未能细分 google/github
        // 的 social 账号。social 切号只需 social token + profile_arn，不真正区分子类型，
        // 因此按 social 处理即可，否则这类账号导入成功却切不回 CLI（M4）。
        "Google" | "Github" | "Unknown" => (
            "kirocli:social:token",
            "kirocli:social:device-registration",
            "social",
        ),
        _ => return Err(format!("不支持的 provider: {}", provider)),
    };

    // Social 默认 profile_arn（与 Electron 版本一致；IdC token 不带 profile_arn）
    const SOCIAL_PROFILE_ARN: &str =
        "arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK";
    // CLI 默认 Builder ID start_url（Enterprise 用账号自带的 d-xxx start_url）。
    // 复用 common 的共享常量，避免字面量重复、与 clientIdHash 常量保持同源。
    const DEFAULT_START_URL: &str = crate::commands::common::KIRO_BUILDER_ID_START_URL;

    let default_region = "us-east-1".to_string();
    let region = account.region.as_ref().unwrap_or(&default_region);

    // expires_at：真实 kiro-cli 用 RFC3339 + 'Z'（而非 chrono 默认的 +00:00 偏移）
    let token_expires_at = (chrono::Utc::now() + chrono::Duration::hours(1))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);

    // 公共字段（IdC 与 social 的 token 都包含 oauth_flow / scopes，与实测一致）
    let mut token_data = serde_json::json!({
        "access_token": account.access_token,
        "refresh_token": account.refresh_token,
        "expires_at": token_expires_at,
        "region": region,
        "oauth_flow": "Pkce",
        "scopes": [
            "codewhisperer:completions",
            "codewhisperer:analysis",
            "codewhisperer:conversations"
        ],
    });

    if auth_method == "IdC" {
        // IdC/SSO token：带 start_url，且 **不带** profile_arn（与真实 kiro-cli 一致）
        // 写出边界统一 normalize_start_url 去尾斜杠：本次修复前存的老账号 start_url
        // 可能带斜杠，真实 kiro-cli token 里存的是无斜杠版本，这里兜底规范化。
        let start_url = match provider.as_str() {
            // BuilderId 的 startUrl 固定，缺省直接兜底到默认值
            "BuilderId" => account
                .start_url
                .as_deref()
                .map(normalize_start_url)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_START_URL.to_string()),
            // Enterprise 必须用账号自带的 d-xxx 域名，绝不能回退到 BuilderId 默认值。
            // 缺失或等于 BuilderId 默认值都视为脏数据，直接拒绝（issue #119 根因）。
            "Enterprise" => {
                let url = account
                    .start_url
                    .as_deref()
                    .map(normalize_start_url)
                    .filter(|s| !s.is_empty())
                    .ok_or(
                        "Enterprise 账号必须提供 start_url（企业自己的 d-xxx 域名），\
                         不能为空",
                    )?;
                if crate::commands::common::is_builder_id_start_url(&url) {
                    return Err("Enterprise 账号的 start_url 不能是 BuilderId 默认值\
                         （https://view.awsapps.com/start），请填入企业自己的 d-xxx 域名"
                        .to_string());
                }
                url
            }
            _ => return Err(format!("不支持的 IdC provider: {provider}")),
        };
        token_data["start_url"] = serde_json::json!(start_url);
    } else {
        // Social token：带 start_url 与 profile_arn
        token_data["start_url"] = serde_json::json!(DEFAULT_START_URL);
        let profile_arn = account
            .profile_arn
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(SOCIAL_PROFILE_ARN);
        token_data["profile_arn"] = serde_json::json!(profile_arn);
    }

    let token_value =
        serde_json::to_string(&token_data).map_err(|e| format!("序列化 token 失败: {e}"))?;

    // device-registration：真实 CLI 含 client_secret_expires_at / oauth_flow / scopes。
    // 本地未持久化 client_secret 过期时间，按 90 天兜底（AWS SSO client_secret 默认有效期，
    // 该字段仅用于 CLI 本地判断，实际有效性由 AWS 服务端校验）。
    let secret_expires_at = (chrono::Utc::now() + chrono::Duration::days(90))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let empty = String::new();
    let device_reg_data = serde_json::json!({
        "client_id": account.client_id.as_ref().unwrap_or(&empty),
        "client_secret": account.client_secret.as_ref().unwrap_or(&empty),
        "client_secret_expires_at": secret_expires_at,
        "region": region,
        "oauth_flow": "Pkce",
        "scopes": [
            "codewhisperer:completions",
            "codewhisperer:analysis",
            "codewhisperer:conversations"
        ],
    });

    let device_reg_value = serde_json::to_string(&device_reg_data)
        .map_err(|e| format!("序列化 device registration 失败: {e}"))?;

    Ok(crate::kiro::cli::KiroCliSwitchPayload {
        token_key: token_key.to_string(),
        token_value,
        device_reg_key: device_reg_key.to_string(),
        device_reg_value,
    })
}

#[cfg(test)]
mod tests {
    use super::build_switch_payload;
    use crate::core::account::Account;

    #[test]
    fn build_switch_payload_treats_unknown_provider_as_social() {
        // M4：provider "Unknown"（social 未细分 google/github）应按 social 切号，不报错。
        let mut acc = Account::new("u@example.com".into(), "label".into());
        acc.provider = Some("Unknown".into());
        let payload = build_switch_payload(&acc).expect("Unknown 应按 social 切号");
        assert_eq!(payload.token_key, "kirocli:social:token");
    }
}
