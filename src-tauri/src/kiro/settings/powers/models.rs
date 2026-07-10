use serde::{Deserialize, Serialize};

/// installed.json 中的已安装 Power 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPowerEntry {
    pub name: String,
    #[serde(default)]
    pub registry_id: String,
    #[serde(default)]
    pub auto_installed: bool,
}

/// installed.json 文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPowersFile {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub installed_powers: Vec<InstalledPowerEntry>,
    #[serde(default)]
    pub dismissed_auto_installs: Vec<DismissedEntry>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DismissedEntry {
    pub name: String,
    #[serde(default)]
    pub registry_id: String,
}

impl Default for InstalledPowersFile {
    fn default() -> Self {
        Self {
            version: default_version(),
            installed_powers: vec![],
            dismissed_auto_installs: vec![],
        }
    }
}

/// POWER.md frontmatter
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PowerFrontMatter {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub display_name: String,
}

/// 前端展示用的 Power 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub keywords: Vec<String>,
    pub registry_id: String,
    pub auto_installed: bool,
    /// POWER.md 完整内容
    pub power_md: String,
    /// mcp.json 中定义的 MCP 服务器名列表
    pub mcp_servers: Vec<String>,
    /// steering 目录下的 .md 文件列表
    pub steering_files: Vec<String>,
    /// 目录总大小
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryInfo {
    pub id: String,
    pub name: String,
    pub registry_type: String,
    pub power_count: usize,
}

/// 推荐 Power 条目（来自远程 registry）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedPower {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub repository_url: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub repository_clone_url: String,
    #[serde(default)]
    pub path_in_repo: String,
    #[serde(default)]
    pub repository_branch: String,
    /// 前端用: 是否已安装
    #[serde(default)]
    pub installed: bool,
}

/// 远程推荐 registry 响应
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedRegistryResponse {
    #[serde(default, rename = "schemaVersion")]
    pub _schema_version: String,
    #[serde(default)]
    pub powers: Vec<RecommendedPower>,
}
