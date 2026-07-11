use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use super::config::{gateway_log_dir_raw, GatewayRequestStats};

pub(super) const REQUEST_LOG_FILE: &str = "gateway-request-log.jsonl";

#[cfg(test)]
thread_local! {
    static REQUEST_LOG_PATH_OVERRIDE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn request_log_path_override() -> Option<PathBuf> {
    REQUEST_LOG_PATH_OVERRIDE.with(|cell| cell.borrow().clone())
}

#[cfg(test)]
pub(super) fn set_request_log_path_override(path: Option<PathBuf>) {
    REQUEST_LOG_PATH_OVERRIDE.with(|cell| *cell.borrow_mut() = path);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRequestLogEntry {
    pub occurred_at: String,
    /// 全局唯一的请求 ID（UUID）
    pub request_id: String,
    pub request_index: u64,
    pub endpoint: String,
    pub client_ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub status_code: u16,
    pub outcome: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    /// Prompt Caching: 输入 tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i32>,
    /// Prompt Caching: 输出 tokens
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i32>,
    /// Prompt Caching: 缓存读取 tokens（节省 90% 成本）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i32>,
    /// Prompt Caching: 缓存写入 tokens（首次写入成本 +25%）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<i32>,
    /// 错误类型（如 invalid_request_error, authentication_error 等）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    /// 流式响应信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_info: Option<StreamInfo>,
    /// 请求摘要信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_summary: Option<RequestSummary>,
    /// 响应摘要信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_summary: Option<ResponseSummary>,
}

/// 流式响应信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    pub chunk_count: usize,
    /// 首字节时间（毫秒）
    pub first_chunk_ms: u64,
}

/// 请求摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSummary {
    pub message_count: usize,
    pub tool_count: usize,
    pub total_content_length: usize,
    pub has_images: bool,
}

/// 响应摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseSummary {
    pub content_length: usize,
    pub tool_calls_count: usize,
    pub stop_reason: Option<String>,
}
pub(super) fn request_log_path() -> Result<PathBuf, String> {
    #[cfg(test)]
    if let Some(path) = request_log_path_override() {
        return Ok(path);
    }
    Ok(gateway_log_dir_raw()?.join(REQUEST_LOG_FILE))
}

fn append_gateway_request_log_to_path(
    path: &Path,
    entry: &GatewayRequestLogEntry,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建请求日志目录失败: {e}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("打开请求日志失败: {e}"))?;
    let serialized =
        serde_json::to_string(entry).map_err(|e| format!("序列化请求日志失败: {e}"))?;
    writeln!(file, "{serialized}").map_err(|e| format!("写入请求日志失败: {e}"))
}

pub(super) fn get_gateway_request_logs_from_path(
    path: &Path,
    limit: Option<usize>,
) -> Result<Vec<GatewayRequestLogEntry>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path).map_err(|e| format!("读取请求日志失败: {e}"))?;
    let reader = BufReader::new(file);
    let max_items = limit.unwrap_or(100).clamp(1, 500);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<GatewayRequestLogEntry>(trimmed) {
            entries.push(entry);
        }
    }

    let start = entries.len().saturating_sub(max_items);
    let mut recent = entries.split_off(start);
    recent.reverse();
    Ok(recent)
}

pub(super) fn clear_gateway_request_logs_at_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path).map_err(|e| format!("清空请求日志失败: {e}"))
}

pub fn append_gateway_request_log(entry: &GatewayRequestLogEntry) -> Result<(), String> {
    let path = request_log_path()?;
    append_gateway_request_log_to_path(&path, entry)
}

pub(super) fn get_gateway_request_stats_from_path(
    path: &Path,
) -> Result<GatewayRequestStats, String> {
    if !path.exists() {
        return Ok(GatewayRequestStats {
            total: 0,
            success: 0,
            error: 0,
            streaming: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read_tokens: 0,
            total_cache_creation_tokens: 0,
            requests_with_cache: 0,
            max_duration_ms: 0,
            avg_duration_ms: 0,
        });
    }

    let file = fs::File::open(path).map_err(|e| format!("读取请求日志失败: {e}"))?;
    let reader = BufReader::new(file);

    let mut total = 0;
    let mut success = 0;
    let mut error = 0;
    let mut streaming = 0;
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cache_read_tokens: i64 = 0;
    let mut total_cache_creation_tokens: i64 = 0;
    let mut requests_with_cache = 0;
    let mut max_duration_ms = 0u64;
    let mut total_duration_ms = 0u64;

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<GatewayRequestLogEntry>(trimmed) {
            total += 1;

            if entry.status_code < 400 {
                success += 1;
            } else {
                error += 1;
            }

            if entry.stream {
                streaming += 1;
            }

            total_input_tokens += entry.input_tokens.unwrap_or(0) as i64;
            total_output_tokens += entry.output_tokens.unwrap_or(0) as i64;
            total_cache_read_tokens += entry.cache_read_input_tokens.unwrap_or(0) as i64;
            total_cache_creation_tokens += entry.cache_creation_input_tokens.unwrap_or(0) as i64;

            if entry.cache_read_input_tokens.unwrap_or(0) > 0
                || entry.cache_creation_input_tokens.unwrap_or(0) > 0
            {
                requests_with_cache += 1;
            }

            max_duration_ms = max_duration_ms.max(entry.duration_ms);
            total_duration_ms += entry.duration_ms;
        }
    }

    let avg_duration_ms = if total > 0 {
        total_duration_ms / total as u64
    } else {
        0
    };

    Ok(GatewayRequestStats {
        total,
        success,
        error,
        streaming,
        total_input_tokens,
        total_output_tokens,
        total_cache_read_tokens,
        total_cache_creation_tokens,
        requests_with_cache,
        max_duration_ms,
        avg_duration_ms,
    })
}
