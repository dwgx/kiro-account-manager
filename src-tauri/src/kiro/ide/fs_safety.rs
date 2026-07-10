// 通用文件系统安全原语

/// 检查文件是否为符号链接（安全性检查，参考 Kiro IDE）
pub(super) fn assert_not_symlink(path: &std::path::Path) -> Result<(), String> {
    if path.exists() {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        if metadata.file_type().is_symlink() {
            return Err("Token file is a symbolic link".to_string());
        }
    }
    Ok(())
}

/// 设置文件权限为 0600（仅所有者可读写，仅 Unix 系统）
#[cfg(unix)]
pub(super) fn set_file_permissions(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?
        .permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)
        .map_err(|e| format!("Failed to set file permissions: {}", e))?;
    Ok(())
}

/// Windows 系统不需要设置权限
#[cfg(not(unix))]
pub(super) fn set_file_permissions(_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}
