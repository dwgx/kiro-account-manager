// 读取合成 (IDE json → KiroSettings, 并以 app-settings 为准强制回写 IDE)

use super::io::{get_kiro_settings_path, write_kiro_settings_json};
use super::json_util::{
    get_optional_string_array, get_string_value, upsert_bool_if_changed, upsert_json_if_changed,
    upsert_string_if_changed,
};
use super::model::KiroSettings;

fn classify_trusted_commands(commands: &[String]) -> String {
    if commands.iter().any(|item| item == "*") {
        "all".to_string()
    } else if commands.is_empty() {
        "none".to_string()
    } else {
        "common".to_string()
    }
}

fn format_custom_trusted_commands(commands: &[String]) -> Option<String> {
    if commands.iter().any(|item| item == "*") || commands.is_empty() {
        None
    } else {
        Some(commands.join("\n"))
    }
}

fn resolve_trusted_tools(app_tools: Option<Vec<String>>, json: &serde_json::Value) -> Vec<String> {
    app_tools.unwrap_or_else(|| {
        get_optional_string_array(json, "kiroAgent.trustedTools").unwrap_or_default()
    })
}

fn resolve_configure_mcp(app_value: Option<String>, json: &serde_json::Value) -> String {
    app_value.unwrap_or_else(|| {
        get_string_value(json, "kiroAgent.configureMCP").unwrap_or_else(|| "Enabled".to_string())
    })
}

fn sync_optional_trusted_tools_if_changed(
    json: &mut serde_json::Value,
    app_tools: Option<Vec<String>>,
) -> bool {
    let Some(app_tools) = app_tools else {
        return false;
    };

    if app_tools == get_optional_string_array(json, "kiroAgent.trustedTools").unwrap_or_default() {
        return false;
    }

    upsert_json_if_changed(json, "kiroAgent.trustedTools", serde_json::json!(app_tools))
}

fn sync_optional_configure_mcp_if_changed(
    json: &mut serde_json::Value,
    app_value: Option<String>,
) -> bool {
    let Some(app_value) = app_value else {
        return false;
    };

    if app_value
        == get_string_value(json, "kiroAgent.configureMCP").unwrap_or_else(|| "Enabled".to_string())
    {
        return false;
    }

    upsert_string_if_changed(json, "kiroAgent.configureMCP", &app_value)
}

pub(super) fn get_kiro_settings_inner() -> Result<KiroSettings, String> {
    let path = get_kiro_settings_path().ok_or("无法获取 Kiro 设置路径")?;

    if !path.exists() {
        return Ok(KiroSettings::default());
    }

    // 读取 app-settings.json（首次启动会使用默认值）
    let app_settings = crate::commands::app_settings_cmd::get_app_settings_inner().unwrap_or_default();

    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取设置文件失败: {e}"))?;

    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析设置文件失败: {e}"))?;

    let mut ide_modified = false;

    // 核心逻辑：以 app-settings.json 为准，强制同步到 Kiro IDE
    // 首次启动时，app_settings 使用默认值，会将默认值写入 IDE

    // enableCodebaseIndexing
    let codebase_indexing = app_settings.enable_codebase_indexing.unwrap_or(true);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "kiroAgent.enableCodebaseIndexing",
        codebase_indexing,
    );

    // enableTabAutocomplete
    let tab_autocomplete = app_settings.enable_tab_autocomplete.unwrap_or(true);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "kiroAgent.enableTabAutocomplete",
        tab_autocomplete,
    );

    // usageSummary
    let usage_summary = app_settings.usage_summary.unwrap_or(true);
    ide_modified |= upsert_bool_if_changed(&mut json, "kiroAgent.usageSummary", usage_summary);

    // enableDebugLogs
    let debug_logs = app_settings.enable_debug_logs.unwrap_or(false);
    ide_modified |= upsert_bool_if_changed(&mut json, "kiroAgent.enableDebugLogs", debug_logs);

    // notifyActionRequired
    let notify_action = app_settings.notify_action_required.unwrap_or(true);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "kiroAgent.notifications.agent.actionRequired",
        notify_action,
    );

    // notifyFailure
    let notify_failure = app_settings.notify_failure.unwrap_or(true);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "kiroAgent.notifications.agent.failure",
        notify_failure,
    );

    // notifySuccess
    let notify_success = app_settings.notify_success.unwrap_or(true);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "kiroAgent.notifications.agent.success",
        notify_success,
    );

    // notifyBilling
    let notify_billing = app_settings.notify_billing.unwrap_or(true);
    ide_modified |=
        upsert_bool_if_changed(&mut json, "kiroAgent.notifications.billing", notify_billing);

    // trustedTools
    ide_modified |=
        sync_optional_trusted_tools_if_changed(&mut json, app_settings.trusted_tools.clone());

    // referenceTracker
    let reference_tracker = app_settings.reference_tracker.unwrap_or(false);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "kiroAgent.codeReferences.referenceTracker",
        reference_tracker,
    );

    // configureMCP
    ide_modified |=
        sync_optional_configure_mcp_if_changed(&mut json, app_settings.configure_mcp.clone());

    // telemetry: contentCollectionForServiceImprovement
    let tele_content = app_settings.telemetry_content_collection.unwrap_or(false);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "telemetry.dataSharingAndPromptLogging.contentCollectionForServiceImprovement",
        tele_content,
    );

    // telemetry: usageAnalyticsAndPerformanceMetrics
    let tele_usage = app_settings.telemetry_usage_analytics.unwrap_or(false);
    ide_modified |= upsert_bool_if_changed(
        &mut json,
        "telemetry.dataSharingAndPromptLogging.usageAnalyticsAndPerformanceMetrics",
        tele_usage,
    );

    // telemetry: editStats
    let tele_edit = app_settings.telemetry_edit_stats.unwrap_or(false);
    ide_modified |= upsert_bool_if_changed(&mut json, "telemetry.editStats.enabled", tele_edit);

    // telemetry: feedback
    let tele_feedback = app_settings.telemetry_feedback.unwrap_or(false);
    ide_modified |= upsert_bool_if_changed(&mut json, "telemetry.feedback.enabled", tele_feedback);

    // 如果有修改，写入 Kiro IDE settings.json
    if ide_modified {
        write_kiro_settings_json(&path, &json)?;
    }

    let trusted_commands = get_optional_string_array(&json, "kiroAgent.trustedCommands");

    Ok(KiroSettings {
        http_proxy: get_string_value(&json, "http.proxy"),
        model_selection: get_string_value(&json, "kiroAgent.modelSelection"),
        enable_codebase_indexing: codebase_indexing,
        trusted_commands_mode: trusted_commands
            .as_ref()
            .map(|commands| classify_trusted_commands(commands)),
        custom_trusted_commands: trusted_commands
            .as_ref()
            .and_then(|commands| format_custom_trusted_commands(commands)),
        agent_autonomy: get_string_value(&json, "kiroAgent.agentAutonomy"),
        enable_tab_autocomplete: tab_autocomplete,
        usage_summary,
        enable_debug_logs: debug_logs,
        notify_action_required: notify_action,
        notify_failure,
        notify_success,
        notify_billing,
        trusted_tools: resolve_trusted_tools(app_settings.trusted_tools, &json),
        reference_tracker,
        configure_mcp: resolve_configure_mcp(app_settings.configure_mcp, &json),
        telemetry_content_collection: tele_content,
        telemetry_usage_analytics: tele_usage,
        telemetry_edit_stats: tele_edit,
        telemetry_feedback: tele_feedback,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        classify_trusted_commands, format_custom_trusted_commands, resolve_configure_mcp,
        resolve_trusted_tools, sync_optional_configure_mcp_if_changed,
        sync_optional_trusted_tools_if_changed,
    };

    #[test]
    fn trusted_command_helpers_preserve_existing_mode_rules() {
        assert_eq!(classify_trusted_commands(&[]), "none");
        assert_eq!(classify_trusted_commands(&["*".to_string()]), "all");
        assert_eq!(
            classify_trusted_commands(&["git status".to_string(), "cargo test *".to_string()]),
            "common"
        );

        assert_eq!(format_custom_trusted_commands(&[]), None);
        assert_eq!(format_custom_trusted_commands(&["*".to_string()]), None);
        assert_eq!(
            format_custom_trusted_commands(&["git status".to_string(), "cargo test *".to_string()]),
            Some("git status\ncargo test *".to_string())
        );
    }

    #[test]
    fn resolve_trusted_tools_prefers_app_settings_then_json_then_empty() {
        let json = serde_json::json!({
            "kiroAgent.trustedTools": ["json-tool"]
        });

        assert_eq!(
            resolve_trusted_tools(Some(vec!["app-tool".to_string()]), &json),
            vec!["app-tool".to_string()]
        );
        assert_eq!(
            resolve_trusted_tools(None, &json),
            vec!["json-tool".to_string()]
        );
        assert_eq!(
            resolve_trusted_tools(None, &serde_json::json!({})),
            Vec::<String>::new()
        );
    }

    #[test]
    fn resolve_configure_mcp_prefers_app_settings_then_json_then_enabled() {
        let json = serde_json::json!({
            "kiroAgent.configureMCP": "Disabled"
        });

        assert_eq!(
            resolve_configure_mcp(Some("Enabled".to_string()), &json),
            "Enabled".to_string()
        );
        assert_eq!(
            resolve_configure_mcp(None, &json),
            "Disabled".to_string()
        );
        assert_eq!(
            resolve_configure_mcp(None, &serde_json::json!({})),
            "Enabled".to_string()
        );
    }

    #[test]
    fn sync_optional_trusted_tools_if_changed_only_updates_for_explicit_app_values() {
        let mut unchanged = serde_json::json!({
            "kiroAgent.trustedTools": ["json-tool"]
        });
        let mut changed = unchanged.clone();
        let mut missing = serde_json::json!({});

        assert!(!sync_optional_trusted_tools_if_changed(
            &mut unchanged,
            None
        ));
        assert_eq!(
            unchanged.get("kiroAgent.trustedTools"),
            Some(&serde_json::json!(["json-tool"]))
        );

        assert!(sync_optional_trusted_tools_if_changed(
            &mut changed,
            Some(vec!["app-tool".to_string()])
        ));
        assert_eq!(
            changed.get("kiroAgent.trustedTools"),
            Some(&serde_json::json!(["app-tool"]))
        );

        assert!(!sync_optional_trusted_tools_if_changed(
            &mut missing,
            Some(vec![])
        ));
        assert_eq!(missing.get("kiroAgent.trustedTools"), None);
    }

    #[test]
    fn sync_optional_configure_mcp_if_changed_only_updates_for_explicit_app_values() {
        let mut unchanged = serde_json::json!({
            "kiroAgent.configureMCP": "Enabled"
        });
        let mut changed = unchanged.clone();
        let mut missing = serde_json::json!({});

        assert!(!sync_optional_configure_mcp_if_changed(
            &mut unchanged,
            None
        ));
        assert_eq!(
            unchanged
                .get("kiroAgent.configureMCP")
                .and_then(serde_json::Value::as_str),
            Some("Enabled")
        );

        assert!(sync_optional_configure_mcp_if_changed(
            &mut changed,
            Some("Disabled".to_string())
        ));
        assert_eq!(
            changed
                .get("kiroAgent.configureMCP")
                .and_then(serde_json::Value::as_str),
            Some("Disabled")
        );

        assert!(!sync_optional_configure_mcp_if_changed(
            &mut missing,
            Some("Enabled".to_string())
        ));
        assert_eq!(missing.get("kiroAgent.configureMCP"), None);
    }
}
