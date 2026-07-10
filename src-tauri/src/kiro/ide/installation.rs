// IDE 安装检测 + 候选路径

use serde::{Deserialize, Serialize};

use super::token::KiroLocalToken;

/// IDE 安装检测结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdeInstallationInfo {
    pub ide_installed: bool,
    pub ide_path: Option<String>,
    pub ide_executable_exists: bool,
    pub config_dir_exists: bool,
    pub error_message: Option<String>,
}

/// 检测 Kiro IDE 是否安装
#[tauri::command]
pub async fn check_ide_installation() -> IdeInstallationInfo {
    tokio::task::spawn_blocking(|| {
        let (ide_path, ide_exists) = detect_kiro_ide_executable();

        // 检查配置目录是否存在
        let config_exists = check_kiro_config_dir();

        let ide_installed = ide_exists && config_exists;

        // 生成详细的错误提示
        let error_message = if !ide_installed {
            if !ide_exists && !config_exists {
                Some("未检测到默认路径的 Kiro IDE 可执行文件和配置文件。\n\n请先安装并登录 Kiro IDE，或在「设置」→「通用」中配置「自定义 Kiro IDE 安装路径」。".to_string())
            } else if !ide_exists {
                Some("未检测到默认路径的 Kiro IDE 可执行文件。\n\n请检查 IDE 是否已安装，或在「设置」→「通用」中配置「自定义 Kiro IDE 安装路径」。".to_string())
            } else if !config_exists {
                Some("Kiro IDE 已安装，但尚未首次登录。\n\n请先在 Kiro IDE 中完成首次登录后再使用切换功能。".to_string())
            } else {
                None
            }
        } else {
            None
        };

        IdeInstallationInfo {
            ide_installed,
            ide_path,
            ide_executable_exists: ide_exists,
            config_dir_exists: config_exists,
            error_message,
        }
    })
    .await
    .unwrap_or(IdeInstallationInfo {
        ide_installed: false,
        ide_path: None,
        ide_executable_exists: false,
        config_dir_exists: false,
        error_message: Some("检测 IDE 安装状态时发生错误".to_string()),
    })
}

/// 检查 Kiro IDE 配置文件是否存在且包含有效 token
fn check_kiro_config_dir() -> bool {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"));

    if let Ok(home_dir) = home {
        let cache_dir = std::path::Path::new(&home_dir)
            .join(".aws")
            .join("sso")
            .join("cache");

        let token_file = cache_dir.join("kiro-auth-token.json");

        // 1. 检查文件是否存在
        if !token_file.exists() {
            return false;
        }

        // 2. 读取并解析文件内容
        if let Ok(content) = std::fs::read_to_string(&token_file) {
            if let Ok(token_data) = serde_json::from_str::<KiroLocalToken>(&content) {
                // 3. 验证必须有 access_token 和 refresh_token
                if token_data.access_token.is_none() || token_data.refresh_token.is_none() {
                    return false;
                }

                // 4. 验证必须有 auth_method
                let auth_method = match token_data.auth_method.as_deref() {
                    Some(method) => method,
                    None => return false,
                };

                // 5. 验证必须有 provider
                if token_data.provider.is_none() {
                    return false;
                }

                // 6. 根据 auth_method 验证特定字段
                match auth_method {
                    "social" => {
                        // Social 账号需要 profileArn
                        token_data.profile_arn.is_some()
                    }
                    "IdC" => {
                        // IdC 账号 (BuilderId/Enterprise) 需要 clientIdHash 和 region
                        token_data.client_id_hash.is_some() && token_data.region.is_some()
                    }
                    _ => false, // 未知的 auth_method
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

/// 检测 IDE 可执行文件
fn detect_kiro_ide_executable() -> (Option<String>, bool) {
    let candidates = get_kiro_ide_paths();
    for path in candidates {
        if path.exists() {
            return (Some(path.to_string_lossy().to_string()), true);
        }
    }
    (None, false)
}

/// 检测配置文件是否存在（用于切换账号前验证）
#[tauri::command]
pub async fn check_kiro_config_files(
    auth_method: String,
    client_id_hash: Option<String>,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "无法获取用户目录".to_string())?;

        let cache_dir = std::path::Path::new(&home)
            .join(".aws")
            .join("sso")
            .join("cache");

        // 检查主 token 文件
        let token_file = cache_dir.join("kiro-auth-token.json");
        if !token_file.exists() {
            return Ok(false);
        }

        // 如果是 IdC 账号，还需检查 client registration 文件
        // 注意：全代码库 IdC 标识恒为 "IdC"（见 auth/providers/idc.rs），
        // 用大小写不敏感比较以兼容前端可能传入的任意大小写变体
        if auth_method.eq_ignore_ascii_case("idc") {
            if let Some(hash) = client_id_hash {
                let client_file = cache_dir.join(format!("{}.json", hash));
                if !client_file.exists() {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// 获取 Kiro IDE 候选路径
pub fn get_kiro_ide_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // 1. 优先检查自定义路径（如果用户在设置中配置了）
    if let Ok(settings) = crate::commands::app_settings_cmd::get_app_settings_inner() {
        if let Some(custom_path) = settings.custom_kiro_path {
            let path_buf = std::path::PathBuf::from(&custom_path);
            paths.push(path_buf);
        }
    }

    // 2. 如果没有自定义路径，检查默认路径
    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            paths.push(
                std::path::PathBuf::from(local_app_data)
                    .join("Programs")
                    .join("Kiro")
                    .join("Kiro.exe"),
            );
        }
    } else if cfg!(target_os = "macos") {
        // macOS: Kiro.app 安装在 /Applications
        paths.push(std::path::PathBuf::from("/Applications/Kiro.app"));
    } else {
        // Linux: 可能在多个位置
        paths.push(std::path::PathBuf::from("/usr/bin/kiro"));

        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                std::path::PathBuf::from(&home)
                    .join(".local")
                    .join("bin")
                    .join("kiro"),
            );
        }
    }

    paths
}

