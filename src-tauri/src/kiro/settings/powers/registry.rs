use std::fs;

use super::models::{RecommendedPower, RecommendedRegistryResponse, RegistryInfo};
use super::PowersManager;

const RECOMMENDED_REGISTRY_URL: &str =
    "https://prod.download.desktop.kiro.dev/powers/default_registry.json";

impl PowersManager {
    /// 获取注册表列表（registries/ 目录下的 .json 文件）
    pub fn list_registries() -> Result<Vec<RegistryInfo>, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let reg_dir = dir.join("registries");
        if !reg_dir.exists() {
            return Ok(vec![]);
        }

        let mut registries = vec![];
        for entry in fs::read_dir(&reg_dir).map_err(|e| format!("读取 registries 目录失败: {e}"))?
        {
            let entry = entry.map_err(|e| format!("读取条目失败: {e}"))?;
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|e| e != "json") {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let id = file_name.trim_end_matches(".json").to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();

            registries.push(RegistryInfo {
                id,
                name: parsed
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                registry_type: parsed
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                power_count: parsed
                    .get("powers")
                    .and_then(|v| v.as_array())
                    .map_or(0, |a| a.len()),
            });
        }
        Ok(registries)
    }

    /// 拉取推荐 Powers 列表，并标记已安装状态
    pub async fn fetch_recommended() -> Result<Vec<RecommendedPower>, String> {
        let resp = reqwest::get(RECOMMENDED_REGISTRY_URL)
            .await
            .map_err(|e| format!("请求推荐列表失败: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let mut registry: RecommendedRegistryResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析推荐列表失败: {e}"))?;

        // 标记已安装
        let installed_names: std::collections::HashSet<String> = Self::load_installed()
            .unwrap_or_default()
            .installed_powers
            .into_iter()
            .map(|e| e.name)
            .collect();

        // 也检查 installed/ 目录
        let installed_dir_names: std::collections::HashSet<String> = Self::powers_dir()
            .map(|d| d.join("installed"))
            .filter(|d| d.exists())
            .and_then(|d| fs::read_dir(&d).ok())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for power in &mut registry.powers {
            power.installed =
                installed_names.contains(&power.name) || installed_dir_names.contains(&power.name);
        }

        Ok(registry.powers)
    }
}
