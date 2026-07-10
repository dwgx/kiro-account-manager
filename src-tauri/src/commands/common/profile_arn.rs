// ===== Profile ARN 常量与 provider 映射 =====

/// BuilderId / Enterprise（IdC）账号的默认 profileArn
pub const KIRO_BUILDER_ID_PROFILE_ARN: &str =
    "arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX";

/// Social（Github / Google）账号的默认 profileArn
pub const KIRO_SOCIAL_PROFILE_ARN: &str =
    "arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK";

pub fn resolve_default_profile_arn(provider: Option<&str>) -> &'static str {
    match provider {
        Some("Github") | Some("Google") => KIRO_SOCIAL_PROFILE_ARN,
        _ => KIRO_BUILDER_ID_PROFILE_ARN,
    }
}

/// 统一的 profileArn 解析逻辑（用于 ListAvailableModels 等 API 调用）
///
/// BuilderId 账号本地常见为 `profileArn=null`，但真实 IDE 抓包会带固定
/// BuilderId profileArn；不带时上游会返回 `Invalid profileArn`。
/// 因此这里对空 profileArn 降级到默认值（根据 provider）。
///
/// ## 降级策略
/// - Enterprise: 保持 None（Enterprise 不需要 profileArn）
/// - 其他 provider: 账号 profileArn → 默认 profileArn（根据 provider）
pub fn resolve_profile_arn_with_fallback(
    account_profile_arn: Option<&str>,
    provider: Option<&str>,
) -> Option<String> {
    // 过滤空白字符串
    let account_profile_arn = account_profile_arn
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match provider {
        Some("Enterprise") => None,
        provider => account_profile_arn
            .map(String::from)
            .or_else(|| Some(resolve_default_profile_arn(provider).to_string())),
    }
}

/// 统一解析带“优先候选”的 profileArn。
///
/// 用于 token refresh 之后的调用：上游刷新结果返回的 profileArn 优先，其次账号保存值，
/// 最后按 provider 降级到默认 profileArn；Enterprise 始终返回 None。
pub fn resolve_profile_arn_from_candidates(
    preferred_profile_arn: Option<&str>,
    account_profile_arn: Option<&str>,
    provider: Option<&str>,
) -> Option<String> {
    let preferred_profile_arn = preferred_profile_arn
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let account_profile_arn = account_profile_arn
        .map(str::trim)
        .filter(|value| !value.is_empty());

    resolve_profile_arn_with_fallback(preferred_profile_arn.or(account_profile_arn), provider)
}

#[cfg(test)]
mod tests {
    use super::{resolve_profile_arn_from_candidates, resolve_profile_arn_with_fallback};

    #[test]
    fn resolve_profile_arn_with_fallback_prefers_trimmed_account_value() {
        let resolved = resolve_profile_arn_with_fallback(
            Some("  arn:aws:codewhisperer:us-west-2:123456789012:profile/CUSTOM  "),
            Some("BuilderId"),
        );

        assert_eq!(
            resolved.as_deref(),
            Some("arn:aws:codewhisperer:us-west-2:123456789012:profile/CUSTOM")
        );
    }

    #[test]
    fn resolve_profile_arn_with_fallback_uses_provider_defaults() {
        assert_eq!(
            resolve_profile_arn_with_fallback(None, Some("BuilderId")).as_deref(),
            Some(super::KIRO_BUILDER_ID_PROFILE_ARN)
        );
        assert_eq!(
            resolve_profile_arn_with_fallback(Some("   "), Some("Google")).as_deref(),
            Some(super::KIRO_SOCIAL_PROFILE_ARN)
        );
        assert_eq!(
            resolve_profile_arn_with_fallback(None, Some("Github")).as_deref(),
            Some(super::KIRO_SOCIAL_PROFILE_ARN)
        );
    }

    #[test]
    fn resolve_profile_arn_with_fallback_omits_enterprise_profile_arn() {
        assert_eq!(
            resolve_profile_arn_with_fallback(
                Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/IGNORED"),
                Some("Enterprise"),
            ),
            None
        );
    }

    #[test]
    fn resolve_profile_arn_from_candidates_prefers_refresh_then_account_then_default() {
        assert_eq!(
            resolve_profile_arn_from_candidates(
                Some(" arn:aws:codewhisperer:us-west-2:123456789012:profile/REFRESHED "),
                Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/ACCOUNT"),
                Some("BuilderId"),
            )
            .as_deref(),
            Some("arn:aws:codewhisperer:us-west-2:123456789012:profile/REFRESHED")
        );
        assert_eq!(
            resolve_profile_arn_from_candidates(
                Some("   "),
                Some(" arn:aws:codewhisperer:us-east-1:123456789012:profile/ACCOUNT "),
                Some("BuilderId"),
            )
            .as_deref(),
            Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/ACCOUNT")
        );
        assert_eq!(
            resolve_profile_arn_from_candidates(None, Some("   "), Some("Google")).as_deref(),
            Some(super::KIRO_SOCIAL_PROFILE_ARN)
        );
    }

    #[test]
    fn resolve_profile_arn_from_candidates_omits_enterprise_even_with_candidates() {
        assert_eq!(
            resolve_profile_arn_from_candidates(
                Some("arn:aws:codewhisperer:us-west-2:123456789012:profile/REFRESHED"),
                Some("arn:aws:codewhisperer:us-east-1:123456789012:profile/ACCOUNT"),
                Some("Enterprise"),
            ),
            None
        );
    }
}
