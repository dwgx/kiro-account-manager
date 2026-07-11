use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use crate::clients::http_client::is_supported_kiro_region;

const CONFIG_DIR: &str = ".kiro-account-manager";
const CONFIG_FILE: &str = "gateway-config.json";
const LOGS_DIR: &str = "logs";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub client_api_keys: Vec<String>,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_account_mode")]
    pub account_mode: String,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub pool_account_ids: Vec<String>,
    #[serde(default = "default_strategy")]
    pub strategy: String,
    #[serde(default = "default_threshold")]
    pub threshold: i32,
    #[serde(default = "default_local_only")]
    pub local_only: bool,
    #[serde(default)]
    pub allowed_ips: Vec<String>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub model_mappings: Vec<ModelMappingRule>,
    /// 系统提示过滤：检测 Claude Code 系统提示并替换为精简版
    #[serde(default)]
    pub filter_claude_code: bool,
    /// 系统提示过滤：去掉 --- SYSTEM PROMPT --- 边界标记
    #[serde(default)]
    pub filter_strip_boundaries: bool,
    /// 系统提示过滤：去掉环境噪音行（git status、recent commits 等）
    #[serde(default)]
    pub filter_env_noise: bool,
    /// 自定义提示过滤规则
    #[serde(default)]
    pub prompt_filter_rules: Vec<PromptFilterRule>,
    /// 是否记录请求日志
    #[serde(default = "default_true_val")]
    pub log_requests: bool,
    /// 响应缓存：是否启用
    #[serde(default = "default_true_val")]
    pub response_cache_enabled: bool,
    /// 响应缓存：TTL（秒）
    #[serde(default = "default_cache_ttl")]
    pub response_cache_ttl: u64,
}

fn default_cache_ttl() -> u64 {
    180
}

/// 自定义提示过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFilterRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    /// regex | lines-containing
    pub rule_type: String,
    /// 匹配模式（正则表达式或子串）
    pub match_pattern: String,
    /// 替换内容（仅 regex 类型使用，空 = 删除匹配）
    #[serde(default)]
    pub replace: String,
}

/// 模型映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMappingRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    /// replace | alias | loadbalance
    #[serde(default = "default_mapping_type")]
    pub rule_type: String,
    pub source_model: String,
    pub target_models: Vec<String>,
    #[serde(default)]
    pub weights: Vec<u32>,
}

fn default_true_val() -> bool {
    true
}
fn default_mapping_type() -> String {
    "replace".to_string()
}
/// 根据模型映射规则解析实际模型名
pub fn resolve_model_mapping(config: &GatewayConfig, requested_model: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static ROUND_ROBIN: AtomicUsize = AtomicUsize::new(0);

    for rule in &config.model_mappings {
        if !rule.enabled {
            continue;
        }
        if rule.source_model != requested_model {
            continue;
        }
        if rule.target_models.is_empty() {
            continue;
        }

        match rule.rule_type.as_str() {
            "replace" | "alias" => {
                return rule.target_models[0].clone();
            }
            "loadbalance" => {
                if rule.weights.is_empty() || rule.weights.len() != rule.target_models.len() {
                    // 无权重或权重数量不匹配，简单轮询
                    let idx =
                        ROUND_ROBIN.fetch_add(1, Ordering::Relaxed) % rule.target_models.len();
                    return rule.target_models[idx].clone();
                }
                // 加权轮询
                let total_weight: u32 = rule.weights.iter().sum();
                if total_weight == 0 {
                    return rule.target_models[0].clone();
                }
                let tick = ROUND_ROBIN.fetch_add(1, Ordering::Relaxed) as u32 % total_weight;
                let mut cumulative = 0u32;
                for (i, &w) in rule.weights.iter().enumerate() {
                    cumulative += w;
                    if tick < cumulative {
                        return rule.target_models[i].clone();
                    }
                }
                return rule.target_models[0].clone();
            }
            _ => {}
        }
    }

    requested_model.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub running: bool,
    pub host: String,
    pub port: u16,
    pub request_count: u64,
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_config: Option<GatewayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRequestStats {
    pub total: usize,
    pub success: usize,
    pub error: usize,
    pub streaming: usize,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub requests_with_cache: usize,
    pub max_duration_ms: u64,
    pub avg_duration_ms: u64,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8765
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_account_mode() -> String {
    "pool".to_string()
}

fn default_strategy() -> String {
    "round_robin".to_string()
}

fn default_threshold() -> i32 {
    90
}

fn default_local_only() -> bool {
    true
}

fn default_log_level() -> String {
    "debug".to_string()
}
pub(super) fn build_bind_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    let normalized = host.trim();
    if normalized.is_empty() {
        return Err("监听地址不能为空".to_string());
    }

    if normalized.eq_ignore_ascii_case("localhost") {
        return Ok(SocketAddr::from(([127, 0, 0, 1], port)));
    }

    let bind_target = if normalized.contains(':') {
        format!("[{normalized}]:{port}")
    } else {
        format!("{normalized}:{port}")
    };

    bind_target
        .parse::<SocketAddr>()
        .map_err(|e| format!("监听地址无效: {e}"))
}
impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_host(),
            port: default_port(),
            access_token: None,
            client_api_keys: Vec::new(),
            region: default_region(),
            account_mode: default_account_mode(),
            account_id: None,
            group_id: None,
            pool_account_ids: Vec::new(),
            strategy: default_strategy(),
            threshold: default_threshold(),
            local_only: default_local_only(),
            allowed_ips: Vec::new(),
            log_level: default_log_level(),
            model_mappings: Vec::new(),
            filter_claude_code: false,
            filter_strip_boundaries: false,
            filter_env_noise: false,
            prompt_filter_rules: Vec::new(),
            log_requests: true,
            response_cache_enabled: true,
            response_cache_ttl: default_cache_ttl(),
        }
    }
}
pub(crate) fn effective_client_api_keys(config: &GatewayConfig) -> Vec<String> {
    let mut keys = Vec::new();

    if let Some(key) = config
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !value.starts_with("#disabled#"))
    // 过滤禁用的 Key
    {
        keys.push(key.to_string());
    }

    for key in config
        .client_api_keys
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .filter(|item| !item.starts_with("#disabled#"))
    // 过滤禁用的 Key
    {
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.to_string());
        }
    }

    keys
}

impl GatewayStatus {
    pub fn stopped(config: &GatewayConfig) -> Self {
        Self {
            running: false,
            host: config.host.clone(),
            port: config.port,
            request_count: 0,
            last_error: None,
            runtime_config: None,
        }
    }
}
pub(super) fn ensure_config_valid(config: &GatewayConfig) -> Result<(), String> {
    build_bind_addr(&config.host, config.port)?;
    if config.port == 0 {
        return Err("端口必须大于 0".to_string());
    }

    let region = config.region.trim();
    if region.is_empty() {
        return Err("region 不能为空".to_string());
    }
    if !is_supported_kiro_region(region) {
        return Err(format!("region 不受支持: {region}"));
    }
    match config.account_mode.as_str() {
        "single"
            if config
                .account_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            return Err("single 模式必须选择账号".to_string());
        }
        "group"
            if config
                .group_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            return Err("group 模式必须选择分组".to_string());
        }
        "pool" if config.pool_account_ids.is_empty() => {
            return Err("pool 模式必须至少选择一个账号".to_string());
        }
        "single" | "group" | "pool" => {}
        "local" => {
            return Err("2API不再支持 local 模式，请改用 single/group/pool 账号池模式".to_string());
        }
        _ => return Err("accountMode 必须是 single/group/pool".to_string()),
    }
    if !matches!(
        config.log_level.as_str(),
        "debug" | "info" | "warn" | "error"
    ) {
        return Err("logLevel 必须是 debug/info/warn/error".to_string());
    }
    if effective_client_api_keys(config).is_empty() {
        return Err("必须配置客户端 API Key".to_string());
    }
    if !config.local_only && config.allowed_ips.is_empty() {
        return Err("允许远程访问时必须至少配置一个白名单来源 IP".to_string());
    }
    for entry in &config.allowed_ips {
        if !is_valid_allowlist_entry(entry) {
            return Err(format!("白名单条目无效: {entry}"));
        }
    }
    Ok(())
}

fn is_valid_allowlist_entry(entry: &str) -> bool {
    let trimmed = entry.trim();
    !trimmed.is_empty()
        && (trimmed.parse::<IpAddr>().is_ok() || trimmed.parse::<ipnet::IpNet>().is_ok())
}

pub(super) fn normalize_config(config: &GatewayConfig) -> GatewayConfig {
    let mut normalized = config.clone();
    normalized.host = normalized.host.trim().to_string();
    normalized.access_token = normalized
        .access_token
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    normalized.region = normalized.region.trim().to_string();
    normalized.account_mode = normalized.account_mode.trim().to_string();
    normalized.strategy = normalized.strategy.trim().to_string();
    normalized.log_level = normalized.log_level.trim().to_ascii_lowercase();
    normalized.client_api_keys = effective_client_api_keys(&normalized);
    normalized.access_token = normalized.client_api_keys.first().cloned();
    normalized.allowed_ips = normalized
        .allowed_ips
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .fold(Vec::new(), |mut acc, item| {
            if !acc.contains(&item) {
                acc.push(item);
            }
            acc
        });
    normalized
}

fn config_path() -> Result<PathBuf, String> {
    Ok(ensure_gateway_data_dir()?.join(CONFIG_FILE))
}

fn gateway_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
        })
        .join(CONFIG_DIR)
}

fn ensure_gateway_data_dir() -> Result<PathBuf, String> {
    let dir = gateway_data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    Ok(dir)
}

pub(super) fn gateway_log_dir_raw() -> Result<PathBuf, String> {
    let dir = ensure_gateway_data_dir()?.join(LOGS_DIR);
    fs::create_dir_all(&dir).map_err(|e| format!("创建日志目录失败: {e}"))?;
    Ok(dir)
}

pub fn load_gateway_config() -> Result<GatewayConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(GatewayConfig::default());
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    let cfg = serde_json::from_str::<GatewayConfig>(&content)
        .map_err(|e| format!("解析配置失败: {e}"))?;
    Ok(normalize_config(&cfg))
}

pub fn get_gateway_config() -> Result<GatewayConfig, String> {
    load_gateway_config()
}

pub fn save_gateway_config(config: &GatewayConfig) -> Result<(), String> {
    let normalized = normalize_config(config);
    ensure_config_valid(&normalized)?;
    let path = config_path()?;
    let content =
        serde_json::to_string_pretty(&normalized).map_err(|e| format!("序列化配置失败: {e}"))?;
    fs::write(path, content).map_err(|e| format!("写入配置失败: {e}"))
}
