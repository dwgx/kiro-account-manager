use super::response::ListAvailableModelsResponse;
use crate::core::account::{Account, AvailableModelsCacheEntry};

pub(super) const AVAILABLE_MODELS_CACHE_TTL_SECONDS: i64 = 30 * 60;

fn now_unix_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

pub(super) fn is_available_models_cache_fresh(cached_at: i64, now: i64) -> bool {
    now.saturating_sub(cached_at) <= AVAILABLE_MODELS_CACHE_TTL_SECONDS
}

pub fn read_available_models_cache(
    account: &Account,
    force_refresh: bool,
) -> Option<ListAvailableModelsResponse> {
    if force_refresh {
        return None;
    }
    let cache = account.available_models_cache.as_ref()?;
    if !is_available_models_cache_fresh(cache.cached_at, now_unix_timestamp()) {
        return None;
    }
    serde_json::from_value(cache.response.clone()).ok()
}

pub fn write_available_models_cache(
    account: &mut Account,
    response: &ListAvailableModelsResponse,
) -> Result<(), String> {
    let response_value =
        serde_json::to_value(response).map_err(|error| format!("序列化模型缓存失败: {error}"))?;
    account.available_models_cache = Some(AvailableModelsCacheEntry {
        response: response_value,
        cached_at: now_unix_timestamp(),
    });
    Ok(())
}

pub fn clear_available_models_cache(account: &mut Account) {
    account.available_models_cache = None;
}
