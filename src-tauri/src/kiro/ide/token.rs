// Kiro IDE 本地 Token / 客户端注册模型 + 读取

use serde::{Deserialize, Serialize};

// ===== Kiro IDE 本地 Token =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroLocalToken {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub auth_method: Option<String>,
    pub provider: Option<String>,
    // Social 专用
    pub profile_arn: Option<String>,
    // IdC 专用
    pub client_id_hash: Option<String>,
    pub region: Option<String>,
    // 注意：Kiro IDE 不在 kiro-auth-token.json 中存储 startUrl
    // startUrl 包含在 clientSecret JWT 的 initiateLoginUri 字段中
}

/// `IdC` 客户端注册信息 (从 {clientIdHash}.json 读取)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRegistration {
    pub client_id: String,
    pub client_secret: String,
    pub expires_at: Option<String>,
}

#[tauri::command]
pub async fn get_kiro_local_token() -> Option<KiroLocalToken> {
    tokio::task::spawn_blocking(|| {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()?;
        let path = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache")
            .join("kiro-auth-token.json");

        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    })
    .await
    .ok()
    .flatten()
}

/// 读取 `IdC` 客户端注册信息
pub async fn get_client_registration(client_id_hash: &str) -> Option<ClientRegistration> {
    // 安全检查：防止路径遍历攻击
    // 只允许字母、数字、下划线和连字符
    if !client_id_hash
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        log::warn!("[安全] 检测到非法的 client_id_hash: {}", client_id_hash);
        return None;
    }

    let hash = client_id_hash.to_string();
    tokio::task::spawn_blocking(move || {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()?;
        let path = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache")
            .join(format!("{hash}.json"));

        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    })
    .await
    .ok()
    .flatten()
}
