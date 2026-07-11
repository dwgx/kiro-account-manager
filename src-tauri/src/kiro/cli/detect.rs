use serde::{Deserialize, Serialize};

// ============================================================
// CLI 2.0 环境检测
// ============================================================

/// CLI 安装检测结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CliInstallationInfo {
    pub cli_installed: bool,
    pub cli_path: Option<String>,
    pub db_path: Option<String>,
    pub db_exists: bool,
}

/// 检测 CLI 2.0 是否安装
pub fn check_cli_installation() -> CliInstallationInfo {
    let cli_path = detect_cli_executable();
    let db_path = detect_cli_database();

    let cli_installed = cli_path.is_some();
    let db_exists = db_path
        .as_ref()
        .is_some_and(|p| std::path::Path::new(p).exists());

    CliInstallationInfo {
        cli_installed,
        cli_path,
        db_path,
        db_exists,
    }
}

/// 检测 CLI 可执行文件
pub fn detect_cli_executable() -> Option<String> {
    let candidates = get_cli_executable_paths();

    for path in candidates {
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

/// 获取 CLI 可执行文件候选路径
fn get_cli_executable_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            paths.push(
                std::path::PathBuf::from(local_app_data)
                    .join("Kiro-Cli")
                    .join("kiro-cli.exe"),
            );
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            // macOS 可能的安装位置
            paths.push(std::path::PathBuf::from("/usr/local/bin/kiro-cli"));
            paths.push(
                std::path::PathBuf::from(&home)
                    .join("Library")
                    .join("Application Support")
                    .join("kiro-cli")
                    .join("bin")
                    .join("kiro-cli"),
            );
        }
    } else {
        // Linux
        if let Ok(home) = std::env::var("HOME") {
            paths.push(std::path::PathBuf::from("/usr/local/bin/kiro-cli"));
            paths.push(std::path::PathBuf::from(&home).join(".local/bin/kiro-cli"));
        }
    }

    paths
}

/// 检测 CLI 数据库
pub fn detect_cli_database() -> Option<String> {
    let candidates = get_cli_database_paths();

    for path in &candidates {
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    // 返回默认路径（即使不存在）
    candidates.first().map(|p| p.to_string_lossy().to_string())
}

/// 获取 CLI 数据库候选路径
fn get_cli_database_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            paths.push(
                std::path::PathBuf::from(local_app_data)
                    .join("Kiro-Cli")
                    .join("data.sqlite3"),
            );
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                std::path::PathBuf::from(&home)
                    .join("Library")
                    .join("Application Support")
                    .join("kiro-cli")
                    .join("data.sqlite3"),
            );
        }
    } else {
        // Linux
        if let Ok(home) = std::env::var("HOME") {
            if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
                paths.push(
                    std::path::PathBuf::from(xdg_data_home)
                        .join("kiro-cli")
                        .join("data.sqlite3"),
                );
            }
            paths.push(
                std::path::PathBuf::from(&home)
                    .join(".local")
                    .join("share")
                    .join("kiro-cli")
                    .join("data.sqlite3"),
            );
        }
    }

    paths
}
