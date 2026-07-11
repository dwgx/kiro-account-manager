// setter 写入 IDE + 回写 app-settings

use super::io::{get_kiro_settings_path, load_kiro_settings_json, write_kiro_settings_json};

const DEFAULT_SAFE_TRUSTED_COMMANDS: &[&str] = &[
    "npm run *",
    "npm test *",
    "pnpm run *",
    "pnpm test *",
    "yarn run *",
    "yarn test *",
    "bun run *",
    "bun test *",
    "cargo check *",
    "cargo test *",
    "cargo build *",
    "cargo clippy *",
    "cargo fmt *",
    "git status",
    "git diff *",
    "git log *",
    "git show *",
    "git branch *",
    "git rev-parse *",
    "cat *",
    "ls *",
    "dir *",
    "pwd",
];

pub(super) fn set_kiro_proxy_inner(proxy: String) -> Result<(), String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    let mut settings = load_kiro_settings_json(&path)?;

    if let Some(obj) = settings.as_object_mut() {
        if proxy.is_empty() {
            // 清除代理时，必须把 proxySupport 设为 off，否则 Kiro 会尝试连接系统代理
            obj.remove("http.proxy");
            obj.insert(
                "http.proxySupport".to_string(),
                serde_json::Value::String("off".to_string()),
            );
        } else {
            // 设置代理时，proxySupport 必须为 on，同时提供代理地址
            obj.insert("http.proxy".to_string(), serde_json::Value::String(proxy));
            obj.insert(
                "http.proxyStrictSSL".to_string(),
                serde_json::Value::Bool(false),
            );
            obj.insert(
                "http.proxySupport".to_string(),
                serde_json::Value::String("on".to_string()),
            );
        }
    }

    write_kiro_settings_json(&path, &settings)
}

pub(super) fn set_kiro_model_inner(model: String) -> Result<(), String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    let mut settings = load_kiro_settings_json(&path)?;

    if let Some(obj) = settings.as_object_mut() {
        obj.insert(
            "kiroAgent.modelSelection".to_string(),
            serde_json::Value::String(model),
        );
    }

    write_kiro_settings_json(&path, &settings)
}

pub(super) fn set_kiro_codebase_indexing_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.enableCodebaseIndexing".to_string(),
        serde_json::json!(enabled),
    )
}

pub(super) fn set_kiro_trusted_commands_inner(
    mode: String,
    custom_commands: Option<String>,
) -> Result<(), String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    let mut settings = load_kiro_settings_json(&path)?;

    if let Some(obj) = settings.as_object_mut() {
        let commands = match mode.as_str() {
            "all" => serde_json::json!(["*"]),
            "common" => {
                // 如果有自定义命令，解析它；否则使用默认列表
                if let Some(ref custom) = custom_commands {
                    if custom.trim().is_empty() {
                        serde_json::json!(DEFAULT_SAFE_TRUSTED_COMMANDS)
                    } else {
                        let cmds: Vec<&str> = custom
                            .lines()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .collect();
                        if cmds.contains(&"*") {
                            return Err("common 模式不允许使用 *，如需全部信任请切换到“全部信任”"
                                .to_string());
                        }
                        serde_json::json!(cmds)
                    }
                } else {
                    serde_json::json!(DEFAULT_SAFE_TRUSTED_COMMANDS)
                }
            }
            "none" => serde_json::json!([]),
            _ => return Err(format!("不支持的 trusted commands 模式: {mode}")),
        };
        obj.insert("kiroAgent.trustedCommands".to_string(), commands);
    }

    write_kiro_settings_json(&path, &settings)
}

// 设置 Agent 自主模式
pub(super) fn set_kiro_agent_autonomy_inner(autonomy: String) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.agentAutonomy".to_string(),
        serde_json::json!(autonomy),
    )
}

// 设置 Tab 自动补全
pub(super) fn set_kiro_tab_autocomplete_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.enableTabAutocomplete".to_string(),
        serde_json::json!(enabled),
    )
}

// 设置使用统计
pub(super) fn set_kiro_usage_summary_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.usageSummary".to_string(),
        serde_json::json!(enabled),
    )
}

// 设置调试日志
pub(super) fn set_kiro_debug_logs_inner(enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(
        "kiroAgent.enableDebugLogs".to_string(),
        serde_json::json!(enabled),
    )
}

// 设置通知选项
pub(super) fn set_kiro_notification_inner(key: String, enabled: bool) -> Result<(), String> {
    set_kiro_generic_inner(key, serde_json::json!(enabled))
}

// ===== 通用设置写入 =====

/// 通用写入 Kiro IDE settings.json（支持 bool / string / string[] 类型）
/// 同时同步到 app-settings.json
pub(super) fn set_kiro_generic_inner(key: String, value: serde_json::Value) -> Result<(), String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    let mut settings = load_kiro_settings_json(&path)?;

    if let Some(obj) = settings.as_object_mut() {
        obj.insert(key.clone(), value.clone());
    }

    write_kiro_settings_json(&path, &settings)?;

    // 同步到 app-settings.json
    sync_to_app_settings(&key, &value);

    Ok(())
}

/// 将 IDE 设置变更同步到 app-settings.json
fn sync_to_app_settings(key: &str, value: &serde_json::Value) {
    let mut app = crate::commands::app_settings_cmd::get_app_settings_inner().unwrap_or_default();
    match key {
        "kiroAgent.trustedTools" => {
            app.trusted_tools = value.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });
        }
        "kiroAgent.codeReferences.referenceTracker" => {
            app.reference_tracker = value.as_bool();
        }
        "kiroAgent.configureMCP" => {
            app.configure_mcp = value.as_str().map(String::from);
        }
        "telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement" => {
            app.telemetry_content_collection = value.as_bool();
        }
        "telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics" => {
            app.telemetry_usage_analytics = value.as_bool();
        }
        "telemetry.editStats.enabled" => {
            app.telemetry_edit_stats = value.as_bool();
        }
        "telemetry.feedback.enabled" => {
            app.telemetry_feedback = value.as_bool();
        }
        "kiroAgent.enableCodebaseIndexing" => {
            app.enable_codebase_indexing = value.as_bool();
        }
        "kiroAgent.enableTabAutocomplete" => {
            app.enable_tab_autocomplete = value.as_bool();
        }
        "kiroAgent.usageSummary" => {
            app.usage_summary = value.as_bool();
        }
        "kiroAgent.enableDebugLogs" => {
            app.enable_debug_logs = value.as_bool();
        }
        "kiroAgent.notifications.agent.actionRequired" => {
            app.notify_action_required = value.as_bool();
        }
        "kiroAgent.notifications.agent.failure" => {
            app.notify_failure = value.as_bool();
        }
        "kiroAgent.notifications.agent.success" => {
            app.notify_success = value.as_bool();
        }
        "kiroAgent.notifications.billing" => {
            app.notify_billing = value.as_bool();
        }
        _ => return, // 不需要同步的 key
    }
    let _ = crate::commands::app_settings_cmd::save_settings_to_file(&app);
}
