use std::path::{Component, Path, PathBuf};

use super::PowersManager;

impl PowersManager {
    pub(super) fn validate_power_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Power 名称不能为空".to_string());
        }
        if name.contains('/') || name.contains('\\') {
            return Err("Power 名称不能包含路径分隔符".to_string());
        }
        if name.contains("..") {
            return Err("Power 名称不能包含 ..".to_string());
        }

        let path = Path::new(name);
        for comp in path.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err("Power 名称非法".to_string());
            }
        }
        Ok(())
    }

    pub(super) fn validate_branch_name(branch: &str) -> Result<(), String> {
        if branch.is_empty() {
            return Ok(());
        }
        if branch.starts_with('-') {
            return Err("分支名非法".to_string());
        }
        if branch.contains('\0')
            || branch.contains(' ')
            || branch.contains('\t')
            || branch.contains('\n')
            || branch.contains('\r')
        {
            return Err("分支名非法".to_string());
        }
        if branch.contains("..")
            || branch.contains("~")
            || branch.contains("^")
            || branch.contains(":")
            || branch.contains('?')
            || branch.contains('*')
            || branch.contains("\\")
        {
            return Err("分支名非法".to_string());
        }
        if branch.ends_with('.')
            || branch.ends_with('/')
            || branch.ends_with(".lock")
            || branch.contains("@{")
            || branch.contains("//")
        {
            return Err("分支名非法".to_string());
        }
        Ok(())
    }

    pub(super) fn validate_clone_url(url: &str) -> Result<(), String> {
        let https_url = Self::convert_to_https_url(url);
        let parsed = reqwest::Url::parse(&https_url).map_err(|_| "仓库 URL 非法".to_string())?;

        if parsed.scheme() != "https" {
            return Err("仅允许 https 仓库地址".to_string());
        }

        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if host != "github.com" {
            return Err("仅允许 github.com 仓库地址".to_string());
        }

        let mut segs = parsed
            .path()
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty());
        let owner = segs.next().unwrap_or_default();
        let repo = segs.next().unwrap_or_default();
        if owner.is_empty() || repo.is_empty() {
            return Err("仓库地址必须包含 owner/repo".to_string());
        }

        Ok(())
    }

    pub(super) fn safe_power_subdir(base_dir: &Path, name: &str) -> Result<PathBuf, String> {
        Self::validate_power_name(name)?;
        let candidate = base_dir.join(name);

        if !candidate.starts_with(base_dir) {
            return Err("非法路径".to_string());
        }

        Ok(candidate)
    }

    pub(super) fn safe_path_in_repo(clone_path: &Path, path_in_repo: &str) -> Result<PathBuf, String> {
        if path_in_repo.is_empty() {
            return Ok(clone_path.to_path_buf());
        }

        let relative = Path::new(path_in_repo);
        if relative.is_absolute() {
            return Err("仓库内路径必须是相对路径".to_string());
        }

        for comp in relative.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err("仓库内路径非法".to_string());
            }
        }

        let candidate = clone_path.join(relative);
        if !candidate.starts_with(clone_path) {
            return Err("仓库内路径非法".to_string());
        }

        Ok(candidate)
    }

    /// SSH URL 转 HTTPS URL
    pub(super) fn convert_to_https_url(url: &str) -> String {
        // git@github.com:user/repo.git -> https://github.com/user/repo.git
        if url.starts_with("git@") {
            let s = url.strip_prefix("git@").unwrap_or(url);
            let s = s.replacen(':', "/", 1);
            return format!("https://{s}");
        }
        url.to_string()
    }
}
