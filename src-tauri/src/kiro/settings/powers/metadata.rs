use std::fs;
use std::path::{Path, PathBuf};

use super::models::PowerFrontMatter;
use super::PowersManager;

impl PowersManager {
    /// 解析 POWER.md frontmatter
    pub(super) fn parse_power_md(content: &str) -> PowerFrontMatter {
        let re = regex::Regex::new(r"^---\n([\s\S]*?)\n---").ok();
        let fm_str = re.and_then(|r| r.captures(content).map(|c| c[1].to_string()));

        let mut fm = PowerFrontMatter::default();
        if let Some(s) = fm_str {
            if let Some(v) = Self::extract_field(&s, "name") {
                fm.name = v;
            }
            if let Some(v) = Self::extract_field(&s, "description") {
                fm.description = v;
            }
            if let Some(v) = Self::extract_field(&s, "author") {
                fm.author = v;
            }
            if let Some(v) = Self::extract_field(&s, "license") {
                fm.license = v;
            }
            if let Some(v) = Self::extract_field(&s, "displayName") {
                fm.display_name = v;
            }
            // keywords: [k1, k2]
            if let Some(kw) = regex::Regex::new(r"keywords:\s*\[([^\]]*)\]")
                .ok()
                .and_then(|r| r.captures(&s).map(|c| c[1].to_string()))
            {
                fm.keywords = kw
                    .split(',')
                    .map(|k| k.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                    .filter(|k| !k.is_empty())
                    .collect();
            }
        }
        fm
    }

    fn extract_field(s: &str, field: &str) -> Option<String> {
        let pattern = format!(r#"{}:\s*['"]?([^'"\n]+)['"]?"#, field);
        regex::Regex::new(&pattern)
            .ok()
            .and_then(|r| r.captures(s).map(|c| c[1].trim().to_string()))
    }

    /// 获取 Power 安装目录中的 MCP 服务器名列表
    pub(super) fn get_mcp_server_names(power_dir: &Path) -> Vec<String> {
        let mcp_path = power_dir.join("mcp.json");
        if !mcp_path.exists() {
            return vec![];
        }
        let content = fs::read_to_string(&mcp_path).unwrap_or_default();
        // mcp.json: { "mcpServers": { "name": {...}, ... } }
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
        match parsed {
            Ok(v) => v
                .get("mcpServers")
                .and_then(|s| s.as_object())
                .map(|obj| obj.keys().cloned().collect())
                .unwrap_or_default(),
            Err(_) => vec![],
        }
    }

    /// 获取 steering 目录下的 .md 文件名列表
    pub(super) fn get_steering_files(power_dir: &Path) -> Vec<String> {
        let steering_dir = power_dir.join("steering");
        if !steering_dir.exists() {
            return vec![];
        }
        fs::read_dir(&steering_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_file())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 计算目录大小
    pub(super) fn dir_size(dir: &PathBuf) -> u64 {
        if !dir.exists() {
            return 0;
        }
        fs::read_dir(dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| {
                        let p = e.path();
                        if p.is_file() {
                            fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
                        } else if p.is_dir() {
                            Self::dir_size(&p)
                        } else {
                            0
                        }
                    })
                    .sum()
            })
            .unwrap_or(0)
    }
}
