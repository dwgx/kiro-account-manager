use std::sync::{Mutex, MutexGuard};

// ===== Mutex 锁辅助 =====

/// 锁定 AppState 中任意 `Mutex<T>`，统一错误信息
pub fn lock_store<'a, T>(mutex: &'a Mutex<T>, ctx: &str) -> Result<MutexGuard<'a, T>, String> {
    mutex
        .lock()
        .map_err(|_| format!("Failed to acquire {ctx} lock"))
}

/// 按 id 从 store 查账号副本，找不到时返回友好错误
pub fn find_account_by_id(
    state: &tauri::State<'_, crate::state::AppState>,
    id: &str,
) -> Result<crate::core::account::Account, String> {
    let store = lock_store(&state.store, "store")?;
    store
        .accounts
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or_else(|| format!("账号未找到 (id={id})"))
}

/// 保存账号 store 到文件，统一错误信息
///
/// 始终走 `try_save_to_file` 这个新版 API（带详细错误），不要再用旧 `save_to_file`（只返回 bool）
pub fn save_store(store: &crate::core::account::AccountStore) -> Result<(), String> {
    store.try_save_to_file()
}
