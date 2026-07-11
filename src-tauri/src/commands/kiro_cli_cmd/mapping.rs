use crate::core::account::Account;
use crate::utils::client_id_hash::{extract_start_url_from_client_secret, normalize_start_url};

/// 解析 IdC 账号的「有效 start_url」：优先用 token 顶层自带的，缺失则回退到
/// clientSecret JWT 里的 `initiateLoginUri`（与添加路径同源）。统一规范化去尾斜杠。
///
/// 真实 kiro-cli 的 Enterprise IdC token 顶层**常常不带** start_url —— 真值藏在
/// device-registration 的 client_secret JWT 里。所以 provider 判定必须用这个兜底后的
/// 值，否则 Enterprise 会被误判成 BuilderId（issue #119）。
pub(super) fn resolve_effective_start_url(
    cli_account: &crate::kiro::cli::KiroCliAccount,
) -> Option<String> {
    cli_account
        .start_url
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            cli_account
                .client_secret
                .as_deref()
                .and_then(extract_start_url_from_client_secret)
        })
        .map(|s| normalize_start_url(&s))
}

/// 从 CLI 账号判断 provider
///
/// IdC 账号靠**有效 start_url**（token 顶层或 clientSecret JWT 兜底后的值）区分
/// BuilderId 与 Enterprise（与前端 JSON 导入同源逻辑）：start_url 缺失或就是 BuilderId
/// 默认值（view.awsapps.com/start）→ BuilderId，否则是企业自己的 d-xxx 域名 →
/// Enterprise。这样导入的 Enterprise 账号才能保留正确的 provider，切回 IDE 时算出
/// 正确的 clientIdHash（issue #119）。
///
/// `effective_start_url` 由 `resolve_effective_start_url` 预先解析好传入——必须在此
/// 之前完成 JWT 兜底，否则顶层不带 start_url 的 Enterprise token 会被误判为 BuilderId。
pub(super) fn determine_provider(
    cli_account: &crate::kiro::cli::KiroCliAccount,
    effective_start_url: Option<&str>,
) -> String {
    if cli_account.auth_method == "social" {
        // Social Login，通过 profile_arn 判断
        if let Some(ref arn) = cli_account.profile_arn {
            if arn.contains("google") {
                return "Google".to_string();
            } else if arn.contains("github") {
                return "Github".to_string();
            }
        }
        return "Unknown".to_string();
    }

    // IdC：用有效 start_url 区分 BuilderId / Enterprise
    match effective_start_url.map(str::trim) {
        Some(url) if !url.is_empty() && !crate::commands::common::is_builder_id_start_url(url) => {
            "Enterprise".to_string()
        }
        _ => "BuilderId".to_string(),
    }
}

/// 检查账号是否已存在。
///
/// 复用 `common::find_existing_account_idx`：user_id 优先，缺失时回退 refresh_token。
/// 以前只按 user_id 匹配、user_id 为 None 直接返回 None，导致 userId 为空的账号
/// （部分 BuilderId/Enterprise，API 只回 email 或都不回）每次导入都被当新账号，
/// 重复堆积（M5）。
pub(super) fn find_existing_account(
    accounts: &[Account],
    user_id: Option<&String>,
    email: Option<&String>,
    provider: &str,
    refresh_token: &str,
) -> Option<usize> {
    crate::commands::common::find_existing_account_idx(
        accounts,
        email,
        provider,
        refresh_token,
        user_id,
    )
}

/// 创建账号标签
pub(super) fn create_account_label(
    is_new: bool,
    token_key: &str,
    existing_account: Option<&Account>,
) -> String {
    if is_new {
        format!("从 kiro-cli 导入 ({token_key})")
    } else {
        existing_account.map_or_else(
            || format!("从 kiro-cli 导入 ({token_key})"),
            |a| a.label.clone(),
        )
    }
}

/// 用可拿到的身份信息构造账号，email/userId 缺失也不阻断导入。
///
/// email/userId 只能从配额 API 响应（getUsageLimits）里取，而部分账号——尤其
/// Enterprise IdC——的响应根本不带 `userInfo`。旧逻辑在两者都缺失时直接返回
/// “无法获取账号标识”硬失败，使这些账号永远无法从 kiro-cli 导入。
///
/// 身份字段主要用于**展示**；去重另有 refresh_token 兜底（`find_existing_account_idx`），
/// 不依赖它们。因此缺失时与在线登录（`auth_cmd::resolve_idc_login_email`）对齐、回退到
/// 占位标识照常建账号：
/// - 有 email → 普通账号；
/// - 无 email 有 userId → Enterprise 账号（email=None，用 userId 标识）；
/// - 两者都无 → Enterprise 建 email=None 的账号（靠 user_id/refresh_token 识别），
///   其余 provider 用 `"{provider}_account"` 占位 email。
pub(super) fn build_account_identity(
    email: Option<&str>,
    user_id: Option<&str>,
    provider: &str,
    label: String,
) -> Account {
    if let Some(e) = email {
        Account::new(e.to_string(), label)
    } else if let Some(uid) = user_id {
        Account::new_enterprise(uid.to_string(), label)
    } else {
        let mut acc = Account::new(format!("{provider}_account"), label);
        if provider == "Enterprise" {
            acc.email = None;
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::{build_account_identity, determine_provider, resolve_effective_start_url};
    use crate::kiro::cli::KiroCliAccount;

    fn idc_account(start_url: Option<&str>, client_secret: Option<&str>) -> KiroCliAccount {
        KiroCliAccount {
            access_token: "a".into(),
            refresh_token: "r".into(),
            profile_arn: None,
            region: "us-east-1".into(),
            expires_at: None,
            scopes: Some(vec!["sso:account:access".into()]),
            auth_method: "IdC".into(),
            token_key: "kirocli:odic:token".into(),
            client_id: None,
            client_secret: client_secret.map(String::from),
            start_url: start_url.map(String::from),
        }
    }

    #[test]
    fn build_account_identity_falls_back_when_no_email_or_user_id() {
        // 回归：配额 API 不返回 userInfo（email/userId 均缺失）时，导入不应失败，
        // 而是按 provider 回退到占位标识建账号。

        // Enterprise：无 email/userId → email=None，靠 refresh_token/后续刷新识别
        let ent = build_account_identity(None, None, "Enterprise", "L".into());
        assert_eq!(ent.email, None, "Enterprise 缺身份时 email 应为 None");
        assert_eq!(ent.user_id, None);

        // 其余 provider：无 email/userId → 占位 email "{provider}_account"
        let bid = build_account_identity(None, None, "BuilderId", "L".into());
        assert_eq!(bid.email.as_deref(), Some("BuilderId_account"));

        // 有 email → 普通账号
        let with_email = build_account_identity(Some("u@e.com"), None, "Google", "L".into());
        assert_eq!(with_email.email.as_deref(), Some("u@e.com"));

        // 无 email 有 userId → Enterprise 账号，用 userId 标识
        let with_uid = build_account_identity(None, Some("uid-1"), "Enterprise", "L".into());
        assert_eq!(with_uid.email, None);
        assert_eq!(with_uid.user_id.as_deref(), Some("uid-1"));
    }

    #[test]
    fn determine_provider_uses_jwt_fallback_start_url_for_enterprise() {
        // H2：token 顶层不带 start_url，但 effective start_url（JWT 兜底后）是企业域名，
        // 应判为 Enterprise 而非误判 BuilderId。
        let acc = idc_account(None, None);
        assert_eq!(
            determine_provider(&acc, Some("https://d-90660ceab3.awsapps.com/start")),
            "Enterprise",
            "有效 start_url 为企业域名时应判 Enterprise"
        );
        // 无任何 start_url 时才回落 BuilderId
        assert_eq!(determine_provider(&acc, None), "BuilderId");
    }

    #[test]
    fn resolve_effective_start_url_prefers_top_level_then_normalizes() {
        // 顶层带 start_url（含尾斜杠）→ 规范化去斜杠
        let acc = idc_account(Some("https://d-x.awsapps.com/start/"), None);
        assert_eq!(
            resolve_effective_start_url(&acc).as_deref(),
            Some("https://d-x.awsapps.com/start")
        );
        // 顶层空白 → 无 client_secret 兜底 → None
        let empty = idc_account(Some("   "), None);
        assert_eq!(resolve_effective_start_url(&empty), None);
    }
}
