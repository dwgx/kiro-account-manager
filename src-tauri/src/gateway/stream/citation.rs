use super::types::AggregatedCitation;

/// 解析 codeReferenceEvent
pub(super) fn parse_code_reference_event(value: &serde_json::Value) -> Option<AggregatedCitation> {
    let references = value.get("references")?.as_array()?;
    let first_ref = references.first()?;

    let repository = first_ref
        .get("repository")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let license_name = first_ref
        .get("licenseName")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 构造 link（使用 repository 作为链接）
    let link = if !repository.is_empty() {
        repository.to_string()
    } else {
        return None;
    };

    // 构造 text（显示许可证信息）
    let text = if !license_name.is_empty() {
        Some(format!("Code reference ({})", license_name))
    } else {
        Some("Code reference".to_string())
    };

    // 构造 target（保留原始的 recommendationContentSpan）
    let target = first_ref
        .get("recommendationContentSpan")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    Some(AggregatedCitation { text, link, target })
}

/// 解析 supplementaryWebLinksEvent
pub(super) fn parse_supplementary_web_links_event(
    value: &serde_json::Value,
) -> Option<AggregatedCitation> {
    let links = value.get("supplementaryWebLinks")?.as_array()?;
    let first_link = links.first()?;

    let url = first_link
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())?
        .to_string();

    let title = first_link
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let snippet = first_link
        .get("snippet")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    // 使用 title 或 snippet 作为显示文本
    let text = title.or(snippet);

    // 构造 target（保留原始的 url 和 snippet）
    let target = serde_json::json!({
        "url": url,
        "snippet": first_link.get("snippet").cloned().unwrap_or(serde_json::Value::Null)
    });

    Some(AggregatedCitation {
        text,
        link: url,
        target,
    })
}

pub(super) fn parse_citation_event(value: &serde_json::Value) -> Option<AggregatedCitation> {
    let target = value.get("target")?.clone();
    let link = value
        .get("citationLink")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())?
        .to_string();
    let text = value
        .get("citationText")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string);
    ensure_citation_target_supported(&target)?;

    Some(AggregatedCitation { text, link, target })
}

fn ensure_citation_target_supported(target: &serde_json::Value) -> Option<()> {
    if let Some(range) = target.get("range") {
        let start_index = range.get("start").and_then(|item| item.as_u64())? as usize;
        let end_index = range.get("end").and_then(|item| item.as_u64())? as usize;
        if end_index < start_index {
            return None;
        }
        return Some(());
    }

    if target
        .get("location")
        .and_then(|item| item.as_u64())
        .is_some()
    {
        return Some(());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_citation_target_supported_accepts_location_without_guessing_range() {
        assert_eq!(
            ensure_citation_target_supported(&serde_json::json!({ "location": 6 })),
            Some(())
        );
    }
}
