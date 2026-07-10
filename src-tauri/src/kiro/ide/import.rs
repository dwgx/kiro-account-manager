// 从 Kiro IDE 导入账号

use serde::{Deserialize, Serialize};

use super::token::{ClientRegistration, KiroLocalToken};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroAccountInfo {
    pub email: String,
    pub provider: String,
    pub auth_method: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    // Social 专用
    pub profile_arn: Option<String>,
    // IdC 专用
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub client_id_hash: Option<String>,
    pub region: Option<String>,
    // 注意：不需要 start_url 字段
    // startUrl 包含在 clientSecret JWT 的 initiateLoginUri 字段中
    // AWS SSO OIDC API 会自动从 JWT 中解析
}

/// 读取 Kiro IDE 中的所有账号
#[tauri::command]
pub async fn read_kiro_accounts() -> Result<Vec<KiroAccountInfo>, String> {
    tokio::task::spawn_blocking(|| {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "无法获取用户目录")?;

        let cache_dir = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache");

        if !cache_dir.exists() {
            return Err("未找到 Kiro IDE 缓存目录".to_string());
        }

        let mut accounts = Vec::new();

        // 读取主 token 文件
        let token_path = cache_dir.join("kiro-auth-token.json");
        if token_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&token_path) {
                if let Ok(token) = serde_json::from_str::<KiroLocalToken>(&content) {
                    let auth_method = token.auth_method.as_deref().unwrap_or("social");
                    let provider = token
                        .provider
                        .clone()
                        .unwrap_or_else(|| "Google".to_string());

                    let mut account = KiroAccountInfo {
                        email: String::new(), // 需要通过 API 获取
                        provider: provider.clone(),
                        auth_method: auth_method.to_string(),
                        access_token: token.access_token.clone(),
                        refresh_token: token.refresh_token.clone(),
                        expires_at: token.expires_at.clone(),
                        profile_arn: token.profile_arn.clone(),
                        client_id: None,
                        client_secret: None,
                        client_id_hash: token.client_id_hash.clone(),
                        region: token.region.clone(),
                    };

                    // 如果是 IdC 账号，读取 client registration
                    if auth_method == "IdC" {
                        if let Some(ref hash) = token.client_id_hash {
                            let client_path = cache_dir.join(format!("{hash}.json"));
                            if let Ok(client_content) = std::fs::read_to_string(&client_path) {
                                if let Ok(client_reg) =
                                    serde_json::from_str::<ClientRegistration>(&client_content)
                                {
                                    account.client_id = Some(client_reg.client_id);
                                    account.client_secret = Some(client_reg.client_secret);
                                }
                            }
                        }
                    }

                    accounts.push(account);
                }
            }
        }

        if accounts.is_empty() {
            return Err("未找到 Kiro IDE 账号，请先在 Kiro IDE 中登录".to_string());
        }

        Ok(accounts)
    })
    .await
    .map_err(|e| format!("读取失败: {e}"))?
}
