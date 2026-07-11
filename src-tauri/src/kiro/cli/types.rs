use serde::{Deserialize, Serialize};

/// Kiro CLI 账号数据
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KiroCliAccount {
    pub access_token: String,
    pub refresh_token: String,
    pub profile_arn: Option<String>,
    pub region: String,
    pub expires_at: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub auth_method: String, // "social" 或 "IdC"
    pub token_key: String,   // 记录来源键名
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    /// IdC/SSO token 自带的 start_url（真实 kiro-cli token 里就有，
    /// 用来区分 BuilderId 与 Enterprise，并还原正确的 clientIdHash）
    pub start_url: Option<String>,
}

/// Device Registration 数据（仅 AWS SSO OIDC）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceRegistration {
    pub client_id: String,
    pub client_secret: String,
    pub region: String,
}

/// CLI 数据库完整快照（用于读取当前状态）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KiroCliDbSnapshot {
    pub token_entries: Vec<KiroCliAuthEntry>,
    pub device_registration: Option<DeviceRegistration>,
    pub db_path: String,
}

/// CLI 认证条目（从 auth_kv 读取的原始记录）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KiroCliAuthEntry {
    pub key: String,
    pub value_json: String,
    pub parsed_token: Option<TokenData>,
}

/// Token 数据（解析后的结构）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<String>,
    pub region: String,
    pub start_url: Option<String>,
    pub oauth_flow: Option<String>,
    pub scopes: Option<Vec<String>>,
}

/// CLI 切号写入载荷（准备写入 DB 的目标记录）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KiroCliSwitchPayload {
    pub token_key: String,
    pub token_value: String,
    pub device_reg_key: String,
    pub device_reg_value: String,
}

/// 写入前的备份数据（用于回滚）。
///
/// 记录切号前**所有受影响 key 的完整快照**（3 个 token key + device_reg_key），而不仅是
/// 目标两键。切号会写目标 token/device_reg 并 DELETE 其余兄弟 token key；只备份目标两键的
/// 旧实现回滚时既不删新写入的键、也不恢复被删的兄弟键，跨类型切换（如 social→IdC）回滚后
/// DB 停在错误状态、原 token 永久丢失。用全量快照可精确还原到切号前。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KiroCliWriteBackup {
    /// 切号前所有受影响 key 的 (key, value) 快照。切号时存在的键才入表；
    /// 回滚时先删光受影响 key 集合，再把本表原样写回，从而精确还原。
    pub key_snapshot: Vec<(String, String)>,
    // 保留旧字段以兼容历史序列化数据（前端目前不读，仅防旧 backup 反序列化失败）
    #[serde(default)]
    pub old_token: Option<(String, String)>,
    #[serde(default)]
    pub old_device_reg: Option<(String, String)>,
    pub backup_time: String,
}
