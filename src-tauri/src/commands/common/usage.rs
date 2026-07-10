use super::kiro_context::resolve_kiro_call_context;

/// Usage 获取结果
pub struct UsageResult {
    pub usage_data: serde_json::Value,
    pub is_banned: bool,
    pub is_auth_error: bool,
}

async fn get_usage_by_account_inner(
    account: &crate::core::account::Account,
    access_token: &str,
    use_account_proxy: bool,
) -> Result<UsageResult, String> {
    use crate::clients::http_client::build_http_client_with_timeout_for_account;
    use crate::clients::kiro_client::KiroClient;

    let ctx = resolve_kiro_call_context(account, "us-east-1");

    let client = if use_account_proxy {
        KiroClient::from_client(build_http_client_with_timeout_for_account(account, 30, 10)?)
    } else {
        KiroClient::new()?
    };
    let usage_call = client
        .get_usage_limits(
            access_token,
            &ctx.machine_id,
            &ctx.region,
            ctx.profile_arn.as_deref(),
            account.auth_method.as_deref(),
            account.provider.as_deref(),
        )
        .await;

    // 如果 getUsageLimits 成功，额外调用 ListAvailableModels 检测封禁
    // 因为某些封禁状态下 getUsageLimits 会正常返回，但 ListAvailableModels 会返回 403
    let mut result = parse_usage_result(usage_call)?;

    if !result.is_banned && ctx.profile_arn.is_some() {
        match client
            .list_available_models(
                access_token,
                &ctx.machine_id,
                &ctx.region,
                ctx.profile_arn.as_deref(),
            )
            .await
        {
            Err(e) if e.starts_with("BANNED:") => {
                log::warn!(
                    "[get_usage_by_account] ListAvailableModels 检测到封禁: {}",
                    e
                );
                result.is_banned = true;
            }
            _ => {
                // 其他情况忽略（AUTH_ERROR 或成功都不影响 usage 结果）
            }
        }
    }

    Ok(result)
}

/// 统一使用 getUsageLimits 接口获取 usage 数据（支持所有账号类型）
pub async fn get_usage_by_account(
    account: &crate::core::account::Account,
    access_token: &str,
) -> Result<UsageResult, String> {
    get_usage_by_account_inner(account, access_token, false).await
}

pub async fn get_usage_by_provider_with_machine_id(
    provider: &str,
    access_token: &str,
    machine_id: &str,
) -> Result<UsageResult, String> {
    // 为了兼容旧调用，创建一个临时账号对象
    let mut temp_account = crate::core::account::Account::new(String::new(), String::new());
    temp_account.provider = Some(provider.to_string());
    temp_account.machine_id = Some(machine_id.to_string());

    // 根据 provider 设置 auth_method（profile_arn 由 get_usage_by_account 统一处理）
    if provider == "BuilderId" || provider == "Enterprise" {
        temp_account.auth_method = Some("IdC".to_string());
    } else {
        temp_account.auth_method = Some("social".to_string());
    }

    get_usage_by_account(&temp_account, access_token).await
}

/// 为企业账号获取 usage 数据
pub async fn get_enterprise_usage(
    access_token: &str,
    machine_id: &str,
) -> Result<UsageResult, String> {
    use crate::clients::kiro_client::KiroClient;

    let client = KiroClient::new()?;
    let result = client
        .get_enterprise_usage_limits(access_token, machine_id)
        .await;

    match result {
        Ok(usage_data) => Ok(UsageResult {
            usage_data,
            is_banned: false,
            is_auth_error: false,
        }),
        Err(e) if e.starts_with("BANNED:") => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: true,
            is_auth_error: false,
        }),
        Err(e) if is_auth_error_message(&e) => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: false,
            is_auth_error: true,
        }),
        Err(e) => Err(e),
    }
}

/// 解析 usage 结果，提取封禁状态和认证错误
fn parse_usage_result(result: Result<serde_json::Value, String>) -> Result<UsageResult, String> {
    match result {
        Ok(usage_data) => Ok(UsageResult {
            usage_data, // 直接使用 JSON Value
            is_banned: false,
            is_auth_error: false,
        }),
        Err(e) if e.starts_with("BANNED:") => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: true,
            is_auth_error: false,
        }),
        // 401 或认证相关错误（包括 403 + token invalid）
        Err(e) if is_auth_error_message(&e) => Ok(UsageResult {
            usage_data: serde_json::Value::Null,
            is_banned: false,
            is_auth_error: true,
        }),
        // 其他错误直接抛出
        Err(e) => Err(e),
    }
}

pub fn is_auth_error_message(error: &str) -> bool {
    let lower = error.to_lowercase();
    error.starts_with("AUTH_ERROR:")
        || error.contains("401")
        || error.contains("Unauthorized")
        || lower.contains("expired")
        || lower.contains("invalid")
}

#[cfg(test)]
mod tests {
    use super::parse_usage_result;

    #[test]
    fn parse_usage_result_maps_banned_and_auth_errors_without_failing() {
        let banned = parse_usage_result(Err("BANNED: blocked".to_string())).unwrap();
        assert!(banned.is_banned);
        assert!(!banned.is_auth_error);
        assert_eq!(banned.usage_data, serde_json::Value::Null);

        let auth_error = parse_usage_result(Err("AUTH_ERROR: token expired".to_string())).unwrap();
        assert!(!auth_error.is_banned);
        assert!(auth_error.is_auth_error);
        assert_eq!(auth_error.usage_data, serde_json::Value::Null);
    }
}
