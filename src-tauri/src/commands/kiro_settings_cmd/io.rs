// 平台路径 + settings.json 读写 + 异步 blocking runner

use std::path::{Path, PathBuf};

pub(super) fn get_kiro_settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|appdata| {
            PathBuf::from(appdata)
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("Kiro")
                .join("User")
                .join("settings.json")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

pub(super) fn load_kiro_settings_json(path: &Path) -> Result<serde_json::Value, String> {
    if path.exists() {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("读取设置文件失败: {e}"))?;
        Ok(serde_json::from_str(&content).unwrap_or(serde_json::json!({})))
    } else {
        Ok(serde_json::json!({}))
    }
}

pub(super) fn write_kiro_settings_json(
    path: &Path,
    settings: &serde_json::Value,
) -> Result<(), String> {
    let content =
        serde_json::to_string_pretty(settings).map_err(|e| format!("序列化设置失败: {e}"))?;

    std::fs::write(path, content).map_err(|e| format!("写入设置文件失败: {e}"))
}

pub(super) async fn run_kiro_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("Task failed: {e}"))?
}
