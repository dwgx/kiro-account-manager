// Skills 管理（读取/编辑 ~/.kiro/skills/<name>/SKILL.md 和 <project>/.kiro/skills/）

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub name: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<String>,
    pub extra_files: Vec<String>,
    /// "user" 或 "project"
    pub scope: String,
}

pub struct SkillsManager;

impl SkillsManager {
    pub fn user_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("skills"))
    }

    pub fn project_dir(project_dir: &str) -> PathBuf {
        PathBuf::from(project_dir).join(".kiro").join("skills")
    }

    fn load_from_dir(dir: &PathBuf, scope: &str) -> Result<Vec<SkillInfo>, String> {
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut skills = vec![];

        for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {e}"))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let metadata = fs::metadata(&skill_md).ok();
            let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
            let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
                let datetime: chrono::DateTime<chrono::Local> = t.into();
                datetime.format("%Y/%m/%d %H:%M:%S").to_string()
            });

            let content = fs::read_to_string(&skill_md).unwrap_or_default();

            let extra_files = fs::read_dir(&path)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .filter(|e| e.path().is_file())
                        .filter_map(|e| {
                            let fname = e.file_name().to_string_lossy().to_string();
                            if fname == "SKILL.md" {
                                None
                            } else {
                                Some(fname)
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            skills.push(SkillInfo {
                name,
                content,
                size,
                modified_at,
                extra_files,
                scope: scope.to_string(),
            });
        }

        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skills)
    }

    fn resolve_dir(scope: &str, project_dir: Option<&str>) -> Result<PathBuf, String> {
        match scope {
            "project" => {
                let pd = project_dir.ok_or("项目级操作需要提供项目目录")?;
                Ok(Self::project_dir(pd))
            }
            _ => Self::user_dir().ok_or_else(|| "无法获取用户目录".to_string()),
        }
    }

    fn validate_skill_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Skill 名称不能为空".to_string());
        }
        if name.contains('/') || name.contains('\\') {
            return Err("Skill 名称不能包含路径分隔符".to_string());
        }
        if name.contains("..") {
            return Err("Skill 名称不能包含 ..".to_string());
        }

        let path = Path::new(name);
        for comp in path.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err("Skill 名称非法".to_string());
            }
        }
        Ok(())
    }

    /// 校验用户传入的 git 分支名,防止把 `--upload-pack=...` 之类的选项或控制字符
    /// 当作 `git clone --branch <arg>` 的参数注入(H7)。与 powers.rs::validate_branch_name
    /// 保持同源规则:拒绝 `-` 开头、空白/NUL、git refname 非法字符及危险后缀。
    fn validate_branch_name(branch: &str) -> Result<(), String> {
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
            || branch.contains('~')
            || branch.contains('^')
            || branch.contains(':')
            || branch.contains('?')
            || branch.contains('*')
            || branch.contains('\\')
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

    fn safe_skill_dir(base_dir: &Path, name: &str) -> Result<PathBuf, String> {
        Self::validate_skill_name(name)?;
        let candidate = base_dir.join(name);

        if !candidate.starts_with(base_dir) {
            return Err("非法路径".to_string());
        }

        Ok(candidate)
    }

    fn extract_repo_name(repo_url: &str) -> Option<String> {
        let parsed = reqwest::Url::parse(repo_url).ok()?;
        let mut segments = parsed
            .path_segments()?
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        let repo = segments.pop()?;
        Some(repo.trim_end_matches(".git").to_string())
    }

    fn derive_skill_name(source_dir: &Path, target_name: Option<&str>) -> Result<String, String> {
        let name = target_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                source_dir
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
            })
            .ok_or_else(|| "无法推断 Skill 名称".to_string())?;

        Self::validate_skill_name(&name)?;
        Ok(name)
    }

    fn normalize_import_source(source_path: &str) -> Result<PathBuf, String> {
        let raw_path = PathBuf::from(source_path);
        if !raw_path.exists() {
            return Err("导入路径不存在".to_string());
        }

        let source_dir = if raw_path.is_file() {
            let file_name = raw_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default();
            if file_name != "SKILL.md" {
                return Err("请选择 Skill 根目录或其中的 SKILL.md".to_string());
            }
            raw_path
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| "无法解析 Skill 根目录".to_string())?
        } else {
            raw_path
        };

        if !source_dir.is_dir() {
            return Err("导入路径必须是目录".to_string());
        }

        if !source_dir.join("SKILL.md").exists() {
            return Err("选中的目录缺少 SKILL.md".to_string());
        }

        fs::canonicalize(&source_dir).map_err(|e| format!("解析导入目录失败: {e}"))
    }

    fn copy_skill_tree(source_dir: &Path, target_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(target_dir).map_err(|e| format!("创建 Skill 目录失败: {e}"))?;

        for entry in fs::read_dir(source_dir).map_err(|e| format!("读取 Skill 目录失败: {e}"))?
        {
            let entry = entry.map_err(|e| format!("读取 Skill 条目失败: {e}"))?;
            let source_path = entry.path();
            let target_path = target_dir.join(entry.file_name());
            let metadata = fs::symlink_metadata(&source_path)
                .map_err(|e| format!("读取 Skill 元信息失败: {e}"))?;

            if metadata.file_type().is_symlink() {
                continue;
            }

            if metadata.is_dir() {
                Self::copy_skill_tree(&source_path, &target_path)?;
            } else if metadata.is_file() {
                fs::copy(&source_path, &target_path)
                    .map_err(|e| format!("复制 Skill 文件失败: {e}"))?;
            }
        }

        Ok(())
    }

    fn import_from_dir(
        source_dir: &Path,
        target_name: Option<&str>,
        scope: &str,
        project_dir: Option<&str>,
        overwrite: bool,
    ) -> Result<SkillInfo, String> {
        let base_dir = Self::resolve_dir(scope, project_dir)?;
        fs::create_dir_all(&base_dir).map_err(|e| format!("创建 Skill 目录失败: {e}"))?;

        let skill_name = Self::derive_skill_name(source_dir, target_name)?;
        let target_dir = Self::safe_skill_dir(&base_dir, &skill_name)?;

        if target_dir.exists() {
            if !overwrite {
                return Err(format!("Skill 已存在: {skill_name}"));
            }
            fs::remove_dir_all(&target_dir).map_err(|e| format!("覆盖旧 Skill 失败: {e}"))?;
        }

        Self::copy_skill_tree(source_dir, &target_dir)?;
        Self::load(&skill_name, scope, project_dir)
    }

    pub fn import_local(
        source_path: &str,
        target_name: Option<&str>,
        scope: &str,
        project_dir: Option<&str>,
        overwrite: bool,
    ) -> Result<SkillInfo, String> {
        let source_dir = Self::normalize_import_source(source_path)?;
        Self::import_from_dir(&source_dir, target_name, scope, project_dir, overwrite)
    }

    pub fn import_from_github(
        repo_url: &str,
        path_in_repo: Option<&str>,
        branch: Option<&str>,
        target_name: Option<&str>,
        scope: &str,
        project_dir: Option<&str>,
        overwrite: bool,
    ) -> Result<SkillInfo, String> {
        let parsed =
            reqwest::Url::parse(repo_url).map_err(|_| "GitHub 仓库地址非法".to_string())?;
        if parsed.scheme() != "https" || parsed.host_str().unwrap_or_default() != "github.com" {
            return Err("仅支持 https://github.com/... 仓库地址".to_string());
        }

        let repo_name =
            Self::extract_repo_name(repo_url).ok_or_else(|| "无法解析仓库名称".to_string())?;
        let temp_clone_dir = std::env::temp_dir().join(format!(
            "kiro-account-manager-skill-import-{}",
            uuid::Uuid::new_v4()
        ));

        let branch_name = branch
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("main");
        Self::validate_branch_name(branch_name)?;

        let clone_result = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--single-branch",
                "--branch",
                branch_name,
                repo_url,
            ])
            .arg(&temp_clone_dir)
            .output()
            .map_err(|e| format!("执行 git clone 失败（请确保已安装 git）: {e}"))?;

        if !clone_result.status.success() {
            let stderr = String::from_utf8_lossy(&clone_result.stderr);
            let _ = fs::remove_dir_all(&temp_clone_dir);
            return Err(format!("git clone 失败: {stderr}"));
        }

        let resolved = (|| -> Result<SkillInfo, String> {
            let repo_sub_path = path_in_repo
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let source_dir = if let Some(path_in_repo) = repo_sub_path {
                let relative = Path::new(path_in_repo);
                if relative.is_absolute() {
                    return Err("仓库内路径必须是相对路径".to_string());
                }
                for component in relative.components() {
                    if !matches!(component, Component::Normal(_)) {
                        return Err("仓库内路径非法".to_string());
                    }
                }
                temp_clone_dir.join(relative)
            } else {
                temp_clone_dir.clone()
            };

            let source_dir = Self::normalize_import_source(source_dir.to_string_lossy().as_ref())?;
            let fallback_name = if repo_sub_path.is_some() {
                target_name
            } else {
                target_name.or(Some(repo_name.as_str()))
            };
            Self::import_from_dir(&source_dir, fallback_name, scope, project_dir, overwrite)
        })();

        let _ = fs::remove_dir_all(&temp_clone_dir);
        resolved
    }

    pub fn load_all(project_dir: Option<&str>) -> Result<Vec<SkillInfo>, String> {
        let mut all = vec![];
        if let Some(dir) = Self::user_dir() {
            all.extend(Self::load_from_dir(&dir, "user")?);
        }
        if let Some(pd) = project_dir {
            all.extend(Self::load_from_dir(&Self::project_dir(pd), "project")?);
        }
        Ok(all)
    }

    pub fn load(name: &str, scope: &str, project_dir: Option<&str>) -> Result<SkillInfo, String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let skill_dir = Self::safe_skill_dir(&dir, name)?;
        let skill_md = skill_dir.join("SKILL.md");

        if !skill_md.exists() {
            return Err(format!("Skill 不存在: {name}"));
        }

        let content = fs::read_to_string(&skill_md).map_err(|e| format!("读取文件失败: {e}"))?;
        let metadata = fs::metadata(&skill_md).ok();
        let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        let modified_at = metadata.and_then(|m| m.modified().ok()).map(|t| {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y/%m/%d %H:%M:%S").to_string()
        });

        let extra_files = fs::read_dir(&skill_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_file())
                    .filter_map(|e| {
                        let fname = e.file_name().to_string_lossy().to_string();
                        if fname == "SKILL.md" {
                            None
                        } else {
                            Some(fname)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SkillInfo {
            name: name.to_string(),
            content,
            size,
            modified_at,
            extra_files,
            scope: scope.to_string(),
        })
    }

    pub fn save(
        name: &str,
        content: &str,
        scope: &str,
        project_dir: Option<&str>,
    ) -> Result<(), String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let skill_dir = Self::safe_skill_dir(&dir, name)?;
        fs::create_dir_all(&skill_dir).ok();
        fs::write(skill_dir.join("SKILL.md"), content).map_err(|e| format!("写入失败: {e}"))
    }

    pub fn delete(name: &str, scope: &str, project_dir: Option<&str>) -> Result<(), String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let skill_dir = Self::safe_skill_dir(&dir, name)?;
        if skill_dir.exists() {
            fs::remove_dir_all(&skill_dir).map_err(|e| format!("删除失败: {e}"))?;
        }
        Ok(())
    }

    pub fn create(
        name: &str,
        content: &str,
        scope: &str,
        project_dir: Option<&str>,
    ) -> Result<SkillInfo, String> {
        let dir = Self::resolve_dir(scope, project_dir)?;
        let skill_dir = Self::safe_skill_dir(&dir, name)?;
        if skill_dir.exists() {
            return Err(format!("Skill 已存在: {name}"));
        }
        fs::create_dir_all(&skill_dir).map_err(|e| format!("创建目录失败: {e}"))?;
        fs::write(skill_dir.join("SKILL.md"), content).map_err(|e| format!("写入失败: {e}"))?;
        Self::load(name, scope, project_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::SkillsManager;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "kiro-account-manager-skills-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn import_local_skill_copies_skill_tree_into_project_scope() {
        let source_root = temp_dir("source");
        let source_skill = source_root.join("code-review");
        fs::create_dir_all(source_skill.join("templates")).expect("skill dir should be created");
        fs::write(
            source_skill.join("SKILL.md"),
            "---\nname: \"code-review\"\ndescription: \"Review code\"\n---\nBody\n",
        )
        .expect("skill file should be written");
        fs::write(source_skill.join("notes.txt"), "extra").expect("extra file should be written");

        let project_root = temp_dir("project");

        let imported = SkillsManager::import_local(
            source_skill.to_string_lossy().as_ref(),
            None,
            "project",
            Some(project_root.to_string_lossy().as_ref()),
            false,
        )
        .expect("local import should succeed");

        let imported_dir = project_root
            .join(".kiro")
            .join("skills")
            .join("code-review");
        assert_eq!(imported.name, "code-review");
        assert!(
            imported_dir.join("SKILL.md").exists(),
            "SKILL.md should be copied"
        );
        assert!(
            imported_dir.join("notes.txt").exists(),
            "extra files should be copied"
        );
        assert!(
            imported_dir.join("templates").is_dir(),
            "nested directories should be copied"
        );

        fs::remove_dir_all(source_root).ok();
        fs::remove_dir_all(project_root).ok();
    }
}
