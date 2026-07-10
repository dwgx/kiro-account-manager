// ===== 时间常量（参考 Kiro IDE 源码）=====

/// Token 提前刷新时间（10分钟）
/// 在 token 过期前 10 分钟开始尝试刷新，避免真正过期
/// 参考 Kiro IDE: REFRESH_BEFORE_EXPIRY_SECONDS = 10 * 60
pub const AUTH_TOKEN_REFRESH_BEFORE_EXPIRY_SECONDS: i64 = 10 * 60;

/// Token 过期判断的容错时间（3分钟）
/// 判断 token 是否过期时，提前 3 分钟视为过期，防止时钟偏差
/// 参考 Kiro IDE: AUTH_TOKEN_INVALIDATION_OFFSET_SECONDS = 3 * 60
pub const AUTH_TOKEN_INVALIDATION_OFFSET_SECONDS: i64 = 3 * 60;

/// Client Registration 过期容错时间（15分钟）
/// IdC 账号的 clientSecret 过期检查，提前 15 分钟视为过期
/// 参考 Kiro IDE: CLIENT_REG_INVALIDATION_OFFSET_SECONDS = 15 * 60
#[allow(dead_code)] // 预留给 IdC 账号的 client registration 过期检查
pub const CLIENT_REG_INVALIDATION_OFFSET_SECONDS: i64 = 15 * 60;

/// 后台刷新检查间隔（60秒）
/// 参考 Kiro IDE: REFRESH_LOOP_INTERVAL_SECONDS = 60
pub const REFRESH_LOOP_INTERVAL_SECONDS: u64 = 60;

// ===== Token 过期检查函数 =====

/// 检查 token 是否即将过期（需要刷新）
///
/// 在 token 过期前 10 分钟返回 true，用于触发提前刷新
pub fn is_token_expiring_soon(expires_at: &str) -> bool {
    is_token_expired_within_seconds(expires_at, AUTH_TOKEN_REFRESH_BEFORE_EXPIRY_SECONDS)
}

/// 检查 token 是否已过期（带容错时间）
///
/// 在 token 过期前 3 分钟返回 true，用于判断 token 是否真正不可用
pub fn is_token_expired(expires_at: &str) -> bool {
    is_token_expired_within_seconds(expires_at, AUTH_TOKEN_INVALIDATION_OFFSET_SECONDS)
}

/// 检查 token 是否需要刷新（即将过期或已过期）
///
/// 等价于 `is_token_expiring_soon(expires_at)`，因为 10 分钟阈值已经包含了 3 分钟的"已过期"判定。
/// 单独提供这个函数是为了让调用点的语义更清晰。
pub fn token_needs_refresh(expires_at: &str) -> bool {
    is_token_expiring_soon(expires_at)
}

/// 检查 token 是否在指定秒数内过期
fn is_token_expired_within_seconds(expires_at: &str, seconds: i64) -> bool {
    match chrono::NaiveDateTime::parse_from_str(expires_at, "%Y/%m/%d %H:%M:%S") {
        Ok(expires) => {
            let now = chrono::Local::now().naive_local();
            let threshold = now + chrono::Duration::seconds(seconds);
            expires < threshold
        }
        Err(_) => true, // 解析失败视为已过期
    }
}

/// 检查 client registration 是否即将过期
#[allow(dead_code)] // 预留给 IdC 账号的 client registration 过期检查
pub fn is_client_registration_expiring(expires_at: &str) -> bool {
    match chrono::NaiveDateTime::parse_from_str(expires_at, "%Y/%m/%d %H:%M:%S") {
        Ok(expires) => {
            let now = chrono::Local::now().naive_local();
            let threshold = now + chrono::Duration::seconds(CLIENT_REG_INVALIDATION_OFFSET_SECONDS);
            expires < threshold
        }
        Err(_) => true,
    }
}

pub fn calc_expires_at(expires_in: i64) -> String {
    let now = chrono::Local::now();
    let expires_at = now + chrono::Duration::seconds(expires_in);
    expires_at.format("%Y/%m/%d %H:%M:%S").to_string()
}

/// 把外部来源的 expires_at 规范化成内部统一格式 `%Y/%m/%d %H:%M:%S`（本地时区）。
///
/// 内部所有过期判断（`is_token_expired_within_seconds`）只认这个格式，解析失败一律
/// 当作"已过期"。但 kiro-cli 数据库里的 token `expires_at` 是 RFC3339（带 'Z'，UTC），
/// 直接原样存进 `account.expires_at` 会让导入账号永远被判为已过期、每次访问都强制刷新。
///
/// 这里统一入口：
/// - 先按内部格式解析（已规范化的值原样返回，幂等）；
/// - 再按 RFC3339 解析，转成本地时区后重新格式化；
/// - 两者都失败返回 None（交由调用方决定回退策略，不再把脏格式写进 store）。
pub fn normalize_expires_at(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // 已经是内部格式：原样返回，保证幂等（避免重复导入时二次转换）
    if chrono::NaiveDateTime::parse_from_str(trimmed, "%Y/%m/%d %H:%M:%S").is_ok() {
        return Some(trimmed.to_string());
    }
    // RFC3339（kiro-cli / IDE token 的真实格式，带时区）→ 转本地时区后重新格式化
    chrono::DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%Y/%m/%d %H:%M:%S")
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::{is_client_registration_expiring, is_token_expired, is_token_expiring_soon};

    #[test]
    fn test_is_token_expiring_soon() {
        // 测试即将过期的 token（9分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(9))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            is_token_expiring_soon(&expires_at),
            "Token expiring in 9 minutes should return true"
        );

        // 测试还有效的 token（11分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(11))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            !is_token_expiring_soon(&expires_at),
            "Token expiring in 11 minutes should return false"
        );

        // 测试已过期的 token
        let expires_at = (chrono::Local::now() - chrono::Duration::minutes(5))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            is_token_expiring_soon(&expires_at),
            "Expired token should return true"
        );

        // 测试无效的时间格式
        assert!(
            is_token_expiring_soon("invalid-date"),
            "Invalid date should return true"
        );
    }

    #[test]
    fn test_is_token_expired() {
        // 测试已过期的 token（2分钟前）
        let expires_at = (chrono::Local::now() - chrono::Duration::minutes(2))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            is_token_expired(&expires_at),
            "Token expired 2 minutes ago should return true"
        );

        // 测试即将过期的 token（2分钟后，在3分钟容错范围内）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(2))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            is_token_expired(&expires_at),
            "Token expiring in 2 minutes should return true (within 3min threshold)"
        );

        // 测试还有效的 token（5分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(5))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            !is_token_expired(&expires_at),
            "Token expiring in 5 minutes should return false"
        );

        // 测试无效的时间格式
        assert!(
            is_token_expired("invalid-date"),
            "Invalid date should return true"
        );
    }

    #[test]
    fn test_is_client_registration_expiring() {
        // 测试即将过期的 client registration（10分钟后，在15分钟容错范围内）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(10))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            is_client_registration_expiring(&expires_at),
            "Client reg expiring in 10 minutes should return true"
        );

        // 测试还有效的 client registration（20分钟后）
        let expires_at = (chrono::Local::now() + chrono::Duration::minutes(20))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            !is_client_registration_expiring(&expires_at),
            "Client reg expiring in 20 minutes should return false"
        );

        // 测试已过期的 client registration
        let expires_at = (chrono::Local::now() - chrono::Duration::days(1))
            .format("%Y/%m/%d %H:%M:%S")
            .to_string();
        assert!(
            is_client_registration_expiring(&expires_at),
            "Expired client reg should return true"
        );
    }
}
