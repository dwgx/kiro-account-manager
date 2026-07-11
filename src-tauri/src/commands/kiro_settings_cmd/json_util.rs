// 纯 serde_json::Value upsert / 取值工具 (无副作用)

pub(super) fn upsert_bool_if_changed(json: &mut serde_json::Value, key: &str, desired: bool) -> bool {
    if json.get(key).and_then(serde_json::Value::as_bool) == Some(desired) {
        return false;
    }

    if let Some(obj) = json.as_object_mut() {
        obj.insert(key.to_string(), serde_json::Value::Bool(desired));
        return true;
    }

    false
}

pub(super) fn upsert_string_if_changed(json: &mut serde_json::Value, key: &str, desired: &str) -> bool {
    if json.get(key).and_then(serde_json::Value::as_str) == Some(desired) {
        return false;
    }

    if let Some(obj) = json.as_object_mut() {
        obj.insert(
            key.to_string(),
            serde_json::Value::String(desired.to_string()),
        );
        return true;
    }

    false
}

pub(super) fn upsert_json_if_changed(
    json: &mut serde_json::Value,
    key: &str,
    desired: serde_json::Value,
) -> bool {
    if json.get(key) == Some(&desired) {
        return false;
    }

    if let Some(obj) = json.as_object_mut() {
        obj.insert(key.to_string(), desired);
        return true;
    }

    false
}

pub(super) fn get_string_value(json: &serde_json::Value, key: &str) -> Option<String> {
    json.get(key)
        .and_then(|value| value.as_str())
        .map(std::string::ToString::to_string)
}

pub(super) fn get_optional_string_array(json: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    json.get(key).and_then(|value| {
        value.as_array().map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{
        get_optional_string_array, upsert_bool_if_changed, upsert_json_if_changed,
        upsert_string_if_changed,
    };

    #[test]
    fn upsert_bool_if_changed_only_marks_when_value_changes() {
        let mut json = serde_json::json!({
            "kiroAgent.enableDebugLogs": false
        });

        assert!(!upsert_bool_if_changed(
            &mut json,
            "kiroAgent.enableDebugLogs",
            false
        ));
        assert!(upsert_bool_if_changed(
            &mut json,
            "kiroAgent.enableDebugLogs",
            true
        ));
        assert_eq!(
            json.get("kiroAgent.enableDebugLogs")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn upsert_string_and_json_helpers_preserve_expected_values() {
        let mut json = serde_json::json!({
            "kiroAgent.configureMCP": "Enabled",
            "kiroAgent.trustedTools": ["tool-a"]
        });

        assert!(!upsert_string_if_changed(
            &mut json,
            "kiroAgent.configureMCP",
            "Enabled"
        ));
        assert!(upsert_string_if_changed(
            &mut json,
            "kiroAgent.configureMCP",
            "Disabled"
        ));
        assert!(upsert_json_if_changed(
            &mut json,
            "kiroAgent.trustedTools",
            serde_json::json!(["tool-b", "tool-c"])
        ));

        assert_eq!(
            json.get("kiroAgent.configureMCP")
                .and_then(serde_json::Value::as_str),
            Some("Disabled")
        );
        assert_eq!(
            json.get("kiroAgent.trustedTools"),
            Some(&serde_json::json!(["tool-b", "tool-c"]))
        );
    }

    #[test]
    fn upsert_helpers_leave_non_object_json_unchanged() {
        let mut json = serde_json::json!(["not-an-object"]);

        assert!(!upsert_bool_if_changed(
            &mut json,
            "kiroAgent.enableDebugLogs",
            true
        ));
        assert!(!upsert_string_if_changed(
            &mut json,
            "kiroAgent.configureMCP",
            "Disabled"
        ));
        assert!(!upsert_json_if_changed(
            &mut json,
            "kiroAgent.trustedTools",
            serde_json::json!(["tool-a"])
        ));
        assert_eq!(json, serde_json::json!(["not-an-object"]));
    }

    #[test]
    fn get_optional_string_array_filters_non_string_entries() {
        let json = serde_json::json!({
            "kiroAgent.trustedTools": ["tool-a", 1, null, "tool-b"]
        });

        assert_eq!(
            get_optional_string_array(&json, "kiroAgent.trustedTools"),
            Some(vec!["tool-a".to_string(), "tool-b".to_string()])
        );
        assert_eq!(get_optional_string_array(&json, "missing"), None);
    }
}
