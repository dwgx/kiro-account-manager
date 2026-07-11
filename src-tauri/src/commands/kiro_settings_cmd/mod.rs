// Kiro IDE 设置命令 (读写 Kiro IDE 的 settings.json)

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递参数
#![allow(clippy::too_many_lines)] // 设置命令文件包含多个函数

mod io;
mod json_util;
mod model;
mod read;
mod write;

pub use model::KiroSettings; // 保持 commands::kiro_settings_cmd::KiroSettings 可达

use io::run_kiro_blocking;
use read::get_kiro_settings_inner;
use write::{
    set_kiro_agent_autonomy_inner, set_kiro_codebase_indexing_inner, set_kiro_debug_logs_inner,
    set_kiro_generic_inner, set_kiro_model_inner, set_kiro_notification_inner, set_kiro_proxy_inner,
    set_kiro_tab_autocomplete_inner, set_kiro_trusted_commands_inner, set_kiro_usage_summary_inner,
};

#[tauri::command]
pub async fn get_kiro_settings() -> Result<KiroSettings, String> {
    run_kiro_blocking(get_kiro_settings_inner).await
}

#[tauri::command]
pub async fn set_kiro_proxy(proxy: String) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_proxy_inner(proxy)).await
}

#[tauri::command]
pub async fn set_kiro_model(model: String) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_model_inner(model)).await
}

#[tauri::command]
pub async fn set_kiro_codebase_indexing(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_codebase_indexing_inner(enabled)).await
}

#[tauri::command]
pub async fn set_kiro_trusted_commands(
    mode: String,
    custom_commands: Option<String>,
) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_trusted_commands_inner(mode, custom_commands)).await
}

#[tauri::command]
pub async fn set_kiro_agent_autonomy(autonomy: String) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_agent_autonomy_inner(autonomy)).await
}

#[tauri::command]
pub async fn set_kiro_tab_autocomplete(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_tab_autocomplete_inner(enabled)).await
}

#[tauri::command]
pub async fn set_kiro_usage_summary(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_usage_summary_inner(enabled)).await
}

#[tauri::command]
pub async fn set_kiro_debug_logs(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_debug_logs_inner(enabled)).await
}

#[tauri::command]
pub async fn set_kiro_notification(key: String, enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || set_kiro_notification_inner(key, enabled)).await
}

/// 设置 trustedTools（字符串数组）
#[tauri::command]
pub async fn set_kiro_trusted_tools(tools: Vec<String>) -> Result<(), String> {
    run_kiro_blocking(move || {
        set_kiro_generic_inner(
            "kiroAgent.trustedTools".to_string(),
            serde_json::json!(tools),
        )
    })
    .await
}

/// 设置 referenceTracker
#[tauri::command]
pub async fn set_kiro_reference_tracker(enabled: bool) -> Result<(), String> {
    run_kiro_blocking(move || {
        set_kiro_generic_inner(
            "kiroAgent.codeReferences.referenceTracker".to_string(),
            serde_json::json!(enabled),
        )
    })
    .await
}

/// 设置 configureMCP（"Enabled" / "Disabled"）
#[tauri::command]
pub async fn set_kiro_configure_mcp(mode: String) -> Result<(), String> {
    run_kiro_blocking(move || {
        set_kiro_generic_inner(
            "kiroAgent.configureMCP".to_string(),
            serde_json::json!(mode),
        )
    })
    .await
}

/// 设置遥测选项（通用 bool，key 由前端传入）
#[tauri::command]
pub async fn set_kiro_telemetry(key: String, enabled: bool) -> Result<(), String> {
    // 白名单校验，防止任意 key 写入
    let allowed = [
        "telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement",
        "telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics",
        "telemetry.editStats.enabled",
        "telemetry.feedback.enabled",
    ];
    if !allowed.contains(&key.as_str()) {
        return Err(format!("不允许的遥测 key: {key}"));
    }
    run_kiro_blocking(move || set_kiro_generic_inner(key, serde_json::json!(enabled))).await
}
