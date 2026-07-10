use std::fs;
use std::path::PathBuf;

use super::models::InstalledPowersFile;

pub struct PowersManager;

impl PowersManager {
    /// ~/.kiro/powers/
    pub fn powers_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".kiro").join("powers"))
    }

    /// 读取 installed.json
    pub fn load_installed() -> Result<InstalledPowersFile, String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        let path = dir.join("installed.json");
        if !path.exists() {
            return Ok(InstalledPowersFile::default());
        }
        let content =
            fs::read_to_string(&path).map_err(|e| format!("读取 installed.json 失败: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("解析 installed.json 失败: {e}"))
    }

    /// 保存 installed.json
    pub fn save_installed(data: &InstalledPowersFile) -> Result<(), String> {
        let dir = Self::powers_dir().ok_or("无法获取用户目录")?;
        fs::create_dir_all(&dir).ok();
        let content = serde_json::to_string_pretty(data).map_err(|e| format!("序列化失败: {e}"))?;
        fs::write(dir.join("installed.json"), content).map_err(|e| format!("写入失败: {e}"))
    }
}
