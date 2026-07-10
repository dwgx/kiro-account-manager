use serde::Deserialize;

/// 最大思考预算 tokens
const MAX_BUDGET_TOKENS: i32 = 24576;

pub(super) fn default_budget_tokens() -> i32 {
    20000
}

pub(super) fn default_max_tokens() -> i32 {
    4096
}

pub(super) fn deserialize_budget_tokens<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i32::deserialize(deserializer)?;
    Ok(value.min(MAX_BUDGET_TOKENS))
}

pub(super) fn default_stream() -> bool {
    true
}
