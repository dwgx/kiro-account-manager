use super::group_tag::{deserialize_tag_links, AccountTagLink};
use super::proxy::AccountProxyConfig;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelsCacheEntry {
    pub response: serde_json::Value,
    pub cached_at: i64,
}

// ============================================================
// 账号实体
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    /// email 字段（企业账号可能没有，用 `user_id` 代替）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    // 账号密码（可选）
    #[serde(default)]
    pub password: Option<String>,
    pub label: String,
    pub status: String,
    pub added_at: String,
    // 认证信息
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    // 账号信息
    pub provider: Option<String>,
    pub user_id: Option<String>,
    // 认证方式（IdC / social）
    #[serde(default)]
    pub auth_method: Option<String>,
    // IdC 专用
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: Option<String>,
    pub client_id_hash: Option<String>,
    pub sso_session_id: Option<String>,
    pub id_token: Option<String>,
    #[serde(default)]
    pub start_url: Option<String>, // Enterprise 的 Start URL
    // Social 专用
    #[serde(default)]
    pub profile_arn: Option<String>,
    // external_idp (微软/Azure AD) 专用：供将来独立刷新使用
    #[serde(default, alias = "token_endpoint")]
    pub token_endpoint: Option<String>,
    #[serde(default, alias = "issuer_url")]
    pub issuer_url: Option<String>,
    #[serde(default)]
    pub scopes: Option<String>, // 单词，camelCase==snake_case，天然两吃，无需 alias
    // 原始 usage API 响应
    pub usage_data: Option<serde_json::Value>,
    // 分组
    #[serde(default)]
    pub group_id: Option<String>,
    // 标签关联（带时间戳）
    #[serde(default, deserialize_with = "deserialize_tag_links")]
    pub tag_links: Vec<AccountTagLink>,
    // 绑定的机器码
    #[serde(default)]
    pub machine_id: Option<String>,
    #[serde(default)]
    pub available_models_cache: Option<AvailableModelsCacheEntry>,
    // 故障追踪（阶段一：失败计数和自动禁用）
    #[serde(default)]
    pub failure_count: u32,
    #[serde(default)]
    pub last_failure_at: Option<String>,
    #[serde(default)]
    pub disabled_reason: Option<String>,
    // 成功计数（用于 balanced 策略）
    #[serde(default)]
    pub success_count: u64,
    // 启用/禁用开关（禁用的账号网关会跳过）
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_config: Option<AccountProxyConfig>,
}

fn default_enabled() -> bool {
    true
}

impl Account {
    /// 创建普通账号（Google/GitHub/BuilderId）
    pub fn new(email: String, label: String) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email: Some(email),
            label,
            status: "active".to_string(),
            added_at: now.format("%Y/%m/%d %H:%M:%S").to_string(),
            access_token: None,
            refresh_token: None,
            expires_at: None,
            provider: None,
            user_id: None,
            auth_method: None,
            client_id: None,
            client_secret: None,
            region: None,
            client_id_hash: None,
            sso_session_id: None,
            id_token: None,
            start_url: None,
            profile_arn: None,
            token_endpoint: None,
            issuer_url: None,
            scopes: None,
            usage_data: None,
            group_id: None,
            tag_links: Vec::new(),
            machine_id: None,
            available_models_cache: None,
            password: None,
            failure_count: 0,
            last_failure_at: None,
            disabled_reason: None,
            success_count: 0,
            enabled: true,
            proxy_config: None,
        }
    }

    /// 创建 Enterprise 账号（没有 email，使用 `user_id`）
    pub fn new_enterprise(user_id: String, label: String) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email: None, // Enterprise 账号没有 email
            label,
            status: "active".to_string(),
            added_at: now.format("%Y/%m/%d %H:%M:%S").to_string(),
            access_token: None,
            refresh_token: None,
            expires_at: None,
            provider: Some("Enterprise".to_string()),
            user_id: Some(user_id),
            auth_method: Some("IdC".to_string()),
            client_id: None,
            client_secret: None,
            region: None,
            client_id_hash: None,
            sso_session_id: None,
            id_token: None,
            start_url: None,
            profile_arn: None,
            token_endpoint: None,
            issuer_url: None,
            scopes: None,
            usage_data: None,
            group_id: None,
            tag_links: Vec::new(),
            machine_id: None,
            available_models_cache: None,
            password: None,
            failure_count: 0,
            last_failure_at: None,
            disabled_reason: None,
            success_count: 0,
            enabled: true,
            proxy_config: None,
        }
    }

    /// 判断是否是 Enterprise 账号
    pub fn is_enterprise(&self) -> bool {
        self.provider.as_deref() == Some("Enterprise")
    }

    /// 获取显示用的标识（Enterprise 用 `user_id`，其他用 email）
    pub fn get_display_id(&self) -> String {
        if self.is_enterprise() {
            self.user_id
                .clone()
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            self.email.clone().unwrap_or_else(|| "Unknown".to_string())
        }
    }

    /// 判断账号是否可用（可正常参与切换/同步）
    pub fn is_available(&self) -> bool {
        !is_unavailable_status(self.status.as_str())
            && !crate::core::usage::is_usage_capped(self.usage_data.as_ref())
            && self.disabled_reason.is_none()
    }
}

fn is_unavailable_status(status: &str) -> bool {
    matches!(
        status,
        "banned" | "封禁" | "已封禁" | "invalid" | "失效" | "已失效" | "Token已失效"
    )
}

#[cfg(test)]
mod tests {
    use crate::core::account::Account;
    use crate::core::usage::is_usage_capped;

    #[test]
    fn account_is_not_available_when_monthly_usage_is_capped() {
        let mut account = Account::new("capped@example.com".to_string(), "capped".to_string());
        account.usage_data = Some(serde_json::json!({
            "overageConfiguration": {
                "overageStatus": "DISABLED"
            },
            "usageBreakdownList": [
                {
                    "currentUsage": 50,
                    "usageLimit": 50
                }
            ]
        }));

        assert!(is_usage_capped(account.usage_data.as_ref()));
        assert!(!account.is_available());
    }
}
