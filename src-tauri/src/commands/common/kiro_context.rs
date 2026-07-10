use super::machine_id::account_machine_id_or_new;
use super::profile_arn::resolve_profile_arn_with_fallback;
use crate::core::account::Account;

// ===== 调用 Kiro Management API 时的上下文解析 =====

/// 调用 Kiro Management API 需要的三件套：machine_id / region / profile_arn
///
/// 解析规则：
/// - machine_id：账号自带（非空）→ 否则生成账号独立 ID
/// - profile_arn：Enterprise → None；其他账号自带 → 否则 provider 默认 ARN
/// - region：profile_arn 解析出来的 region 优先 → 账号 region → fallback
pub struct KiroCallContext {
    pub machine_id: String,
    pub region: String,
    pub profile_arn: Option<String>,
}

/// 从账号 machine_id 解析出有效的 machine_id，空值时生成账号独立 ID
pub fn resolve_machine_id(account_machine_id: Option<String>) -> String {
    account_machine_id_or_new(&account_machine_id)
}

pub fn resolve_kiro_call_context(account: &Account, fallback_region: &str) -> KiroCallContext {
    use crate::clients::http_client::resolve_kiro_upstream_region;

    let machine_id = resolve_machine_id(account.machine_id.clone());

    let profile_arn = resolve_profile_arn_with_fallback(
        account.profile_arn.as_deref(),
        account.provider.as_deref(),
    );

    let region = resolve_kiro_upstream_region(
        profile_arn.as_deref(),
        account.region.as_deref(),
        fallback_region,
    );

    KiroCallContext {
        machine_id,
        region,
        profile_arn,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_kiro_call_context;
    use crate::core::account::Account;

    #[test]
    fn resolve_kiro_call_context_uses_shared_profile_arn_fallback() {
        let mut account = Account::new("builder@example.com".to_string(), "builder".to_string());
        account.provider = Some("BuilderId".to_string());
        account.machine_id = Some("machine-123".to_string());
        account.profile_arn = None;

        let ctx = resolve_kiro_call_context(&account, "us-east-1");

        assert_eq!(ctx.machine_id, "machine-123");
        assert_eq!(
            ctx.profile_arn.as_deref(),
            Some(crate::commands::common::KIRO_BUILDER_ID_PROFILE_ARN)
        );
    }
}
