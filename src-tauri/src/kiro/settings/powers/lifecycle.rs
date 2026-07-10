use std::collections::HashMap;
use std::fs;

use super::models::{DismissedEntry, InstalledPowerEntry, PowerInfo};
use super::PowersManager;

impl PowersManager {
    /// 加载所有已安装 Power 的详细信息
    pub fn load_all() -> Result<Vec<PowerInfo>, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let installed_dir = dir.join("installed");
        let installed_file = Self::load_installed()?;

        // 建立 name -> entry 映射
        let entry_map: HashMap<String, &InstalledPowerEntry> = installed_file
            .installed_powers
            .iter()
            .map(|e| (e.name.clone(), e))
            .collect();

        let mut powers = vec![];

        if !installed_dir.exists() {
            return Ok(powers);
        }

        for entry in
            fs::read_dir(&installed_dir).map_err(|e| format!("读取 installed 目录失败: {e}"))?
        {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let power_md_path = path.join("POWER.md");
            let power_md = fs::read_to_string(&power_md_path).unwrap_or_default();
            let fm = Self::parse_power_md(&power_md);

            let installed_entry = entry_map.get(&name);

            powers.push(PowerInfo {
                display_name: if fm.display_name.is_empty() {
                    fm.name.clone()
                } else {
                    fm.display_name.clone()
                },
                description: fm.description,
                author: fm.author,
                license: fm.license,
                keywords: fm.keywords,
                registry_id: installed_entry.map_or_else(String::new, |e| e.registry_id.clone()),
                auto_installed: installed_entry.is_some_and(|e| e.auto_installed),
                power_md,
                mcp_servers: Self::get_mcp_server_names(&path),
                steering_files: Self::get_steering_files(&path),
                size: Self::dir_size(&path),
                name,
            });
        }

        powers.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(powers)
    }

    /// 获取单个 Power 详情
    pub fn load(name: &str) -> Result<PowerInfo, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let installed_base = dir.join("installed");
        let power_dir = Self::safe_power_subdir(&installed_base, name)?;
        if !power_dir.exists() {
            return Err(format!("Power 不存在: {name}"));
        }

        let installed_file = Self::load_installed()?;
        let installed_entry = installed_file
            .installed_powers
            .iter()
            .find(|e| e.name == name);

        let power_md_path = power_dir.join("POWER.md");
        let power_md = fs::read_to_string(&power_md_path).unwrap_or_default();
        let fm = Self::parse_power_md(&power_md);

        Ok(PowerInfo {
            display_name: if fm.display_name.is_empty() {
                fm.name.clone()
            } else {
                fm.display_name.clone()
            },
            description: fm.description,
            author: fm.author,
            license: fm.license,
            keywords: fm.keywords,
            registry_id: installed_entry.map_or_else(String::new, |e| e.registry_id.clone()),
            auto_installed: installed_entry.is_some_and(|e| e.auto_installed),
            power_md,
            mcp_servers: Self::get_mcp_server_names(&power_dir),
            steering_files: Self::get_steering_files(&power_dir),
            size: Self::dir_size(&power_dir),
            name: name.to_string(),
        })
    }

    /// 安装推荐 Power（与 Kiro IDE 一致的安装流程）
    /// 1. git clone 到 ~/.kiro/powers/repos/<name>/
    /// 2. 只复制 POWER.md, mcp.json, steering/*.md 到 ~/.kiro/powers/installed/<name>/
    /// 3. 更新 installed.json
    pub fn install(
        name: &str,
        clone_url: &str,
        path_in_repo: &str,
        branch: &str,
    ) -> Result<(), String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;

        Self::validate_power_name(name)?;
        Self::validate_clone_url(clone_url)?;
        Self::validate_branch_name(branch)?;

        let installed_base = dir.join("installed");
        let repos_base = dir.join("repos");
        let install_path = Self::safe_power_subdir(&installed_base, name)?;

        if install_path.exists() {
            return Err(format!("Power 已存在: {name}"));
        }

        // 1) clone 到 repos/<name>（与 Kiro IDE 一致）
        let clone_path = Self::safe_power_subdir(&repos_base, name)?;
        // 清理旧的 clone
        if clone_path.exists() {
            let _ = fs::remove_dir_all(&clone_path);
        }

        let branch_arg = if branch.is_empty() {
            "main".to_string()
        } else {
            branch.to_string()
        };

        // 转换 SSH URL 为 HTTPS（与 Kiro 的 convertToHttpsUrl 一致）
        let https_url = Self::convert_to_https_url(clone_url);

        fs::create_dir_all(clone_path.parent().unwrap_or(&dir))
            .map_err(|e| format!("创建 repos 目录失败: {e}"))?;

        let output = std::process::Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--single-branch",
                "--branch",
                &branch_arg,
                &https_url,
            ])
            .arg(&clone_path)
            .output()
            .map_err(|e| format!("执行 git clone 失败（请确保已安装 git）: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = fs::remove_dir_all(&clone_path);
            return Err(format!("git clone 失败: {stderr}"));
        }

        // 2) 确定源目录
        let source_path = Self::safe_path_in_repo(&clone_path, path_in_repo)?;

        if !source_path.exists() {
            let _ = fs::remove_dir_all(&clone_path);
            return Err(format!("仓库中未找到路径: {path_in_repo}"));
        }

        let clone_path_canonical =
            fs::canonicalize(&clone_path).map_err(|e| format!("解析仓库目录失败: {e}"))?;
        let source_path_canonical =
            fs::canonicalize(&source_path).map_err(|e| format!("解析仓库内路径失败: {e}"))?;

        if !source_path_canonical.starts_with(&clone_path_canonical) {
            let _ = fs::remove_dir_all(&clone_path);
            return Err("仓库内路径非法".to_string());
        }

        let source_path = source_path_canonical;

        // 3) 只复制允许的文件到 installed/<name>/（与 Kiro copyPowerFiles 一致）
        fs::create_dir_all(&install_path).map_err(|e| format!("创建安装目录失败: {e}"))?;

        // 复制 POWER.md 和 mcp.json
        for file in &["POWER.md", "mcp.json"] {
            let src = source_path.join(file);
            if src.exists() {
                fs::copy(&src, install_path.join(file))
                    .map_err(|e| format!("复制 {file} 失败: {e}"))?;
            }
        }

        // 复制 steering/ 目录（只复制 .md 文件）
        let steering_src = source_path.join("steering");
        if steering_src.exists() && steering_src.is_dir() {
            let steering_dst = install_path.join("steering");
            Self::copy_steering_dir(&steering_src, &steering_dst)?;
        }

        // 4) 更新 installed.json
        let mut installed = Self::load_installed()?;
        if !installed.installed_powers.iter().any(|e| e.name == name) {
            installed.installed_powers.push(InstalledPowerEntry {
                name: name.to_string(),
                registry_id: "kiro-recommended".to_string(),
                auto_installed: false,
            });
        }
        // 从 dismissed 列表中移除
        installed.dismissed_auto_installs.retain(|d| d.name != name);
        Self::save_installed(&installed)?;

        Ok(())
    }

    /// 复制 steering 目录（只复制 .md 文件，递归子目录）
    fn copy_steering_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
        fs::create_dir_all(dst).map_err(|e| format!("创建 steering 目录失败: {e}"))?;
        for entry in fs::read_dir(src).map_err(|e| format!("读取 steering 目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            let metadata =
                fs::symlink_metadata(&src_path).map_err(|e| format!("读取文件元信息失败: {e}"))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                Self::copy_steering_dir(&src_path, &dst_path)?;
            } else if metadata.is_file() && src_path.extension().is_some_and(|e| e == "md") {
                fs::copy(&src_path, &dst_path).map_err(|e| format!("复制文件失败: {e}"))?;
            }
        }
        Ok(())
    }

    /// 卸载 Power（删除目录 + 从 installed.json 中移除）
    pub fn uninstall(name: &str) -> Result<(), String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let installed_base = dir.join("installed");
        let power_dir = Self::safe_power_subdir(&installed_base, name)?;
        if power_dir.exists() {
            fs::remove_dir_all(&power_dir).map_err(|e| format!("删除 Power 目录失败: {e}"))?;
        }

        // 从 installed.json 移除
        let mut installed = Self::load_installed()?;
        installed.installed_powers.retain(|e| e.name != name);
        // 加入 dismissed 列表防止自动重装
        if !installed
            .dismissed_auto_installs
            .iter()
            .any(|d| d.name == name)
        {
            installed.dismissed_auto_installs.push(DismissedEntry {
                name: name.to_string(),
                registry_id: String::new(),
            });
        }
        Self::save_installed(&installed)?;

        Ok(())
    }
}
