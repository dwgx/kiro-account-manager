use crate::core::account::Account;

/// 根据 `usage_result` 计算账号状态
pub fn calc_status(
    is_banned: bool,
    is_auth_error: bool,
    usage_data: Option<&serde_json::Value>,
) -> String {
    if is_banned {
        "banned".to_string()
    } else if is_auth_error {
        "invalid".to_string()
    } else if crate::core::usage::is_usage_capped(usage_data) {
        "capped".to_string()
    } else if crate::core::usage::is_in_overage(usage_data) {
        "overage".to_string()
    } else {
        "active".to_string()
    }
}

/// 更新账号状态并自动设置 enabled 字段
///
/// 规则：
/// - banned（封禁）→ enabled = false
/// - invalid（失效）→ enabled = false
/// - capped（封顶）→ enabled = false
/// - overage（超额）→ enabled 保持不变（账号还能用超额配额）
/// - active（正常）→ enabled 保持不变
pub fn update_account_status(
    account: &mut crate::core::account::Account,
    is_banned: bool,
    is_auth_error: bool,
) {
    account.status = calc_status(is_banned, is_auth_error, account.usage_data.as_ref());

    // 只有封禁、失效、封顶状态才自动禁用账号
    if matches!(account.status.as_str(), "banned" | "invalid" | "capped") {
        account.enabled = false;
    }
}

fn read_non_empty_string_field(
    value: &serde_json::Value,
    primary_path: &[&str],
    fallback_key: &str,
) -> Option<String> {
    let nested = primary_path
        .iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(|field| field.as_str())
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .map(std::string::ToString::to_string);

    nested.or_else(|| {
        value
            .get(fallback_key)
            .and_then(|field| field.as_str())
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .map(std::string::ToString::to_string)
    })
}

/// 从 `usage_data` 中提取 `email` 和 `user_id`
/// 兼容 `userInfo.email/userInfo.userId` 与顶层 `email/userId`
pub fn extract_user_info(usage_data: &serde_json::Value) -> (Option<String>, Option<String>) {
    let email = read_non_empty_string_field(usage_data, &["userInfo", "email"], "email");
    let user_id = read_non_empty_string_field(usage_data, &["userInfo", "userId"], "userId");
    (email, user_id)
}

/// 查找已存在的账号索引
/// 优先用 `user_id` 去重，其次用 `refresh_token`（BuilderId 可能没有 userId）
pub fn find_existing_account_idx(
    accounts: &[Account],
    _email: Option<&String>,
    _provider: &str,
    refresh_token: &str,
    user_id: Option<&String>,
) -> Option<usize> {
    if let Some(uid) = user_id {
        if let Some(idx) = accounts
            .iter()
            .position(|a| a.user_id.as_ref() == Some(uid))
        {
            return Some(idx);
        }
    }
    // 如果没有 userId，用 refreshToken 去重
    accounts
        .iter()
        .position(|a| a.refresh_token.as_ref() == Some(&refresh_token.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{extract_user_info, find_existing_account_idx};
    use crate::core::account::Account;

    #[test]
    fn extract_user_info_ignores_empty_email_and_reads_user_id() {
        let usage = serde_json::json!({
            "userInfo": {
                "email": "",
                "userId": "user-123"
            }
        });

        assert_eq!(
            extract_user_info(&usage),
            (None, Some("user-123".to_string()))
        );
    }

    #[test]
    fn extract_user_info_falls_back_to_top_level_fields() {
        let usage = serde_json::json!({
            "email": "top@example.com",
            "userId": "top-user-123"
        });

        assert_eq!(
            extract_user_info(&usage),
            (
                Some("top@example.com".to_string()),
                Some("top-user-123".to_string())
            )
        );
    }

    #[test]
    fn extract_user_info_prefers_nested_fields_and_trims_values() {
        let usage = serde_json::json!({
            "email": "fallback@example.com",
            "userId": "fallback-user",
            "userInfo": {
                "email": " nested@example.com ",
                "userId": " nested-user "
            }
        });

        assert_eq!(
            extract_user_info(&usage),
            (
                Some("nested@example.com".to_string()),
                Some("nested-user".to_string())
            )
        );
    }

    #[test]
    fn find_existing_account_idx_uses_user_id_only() {
        let mut first = Account::new("first@example.com".to_string(), "first".to_string());
        first.user_id = Some("user-1".to_string());

        let second = Account::new("second@example.com".to_string(), "second".to_string());
        let accounts = vec![first, second];

        let user_id = "user-1".to_string();
        let second_email = "second@example.com".to_string();

        assert_eq!(
            find_existing_account_idx(&accounts, Some(&second_email), "Google", "", Some(&user_id)),
            Some(0)
        );
        assert_eq!(
            find_existing_account_idx(&accounts, None, "Google", "", None),
            None
        );
        assert_eq!(
            find_existing_account_idx(&accounts, Some(&second_email), "Google", "", None),
            None
        );
    }
}
