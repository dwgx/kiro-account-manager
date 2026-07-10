use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// 自定义反序列化：处理 tag_links 的 null 值
pub(crate) fn deserialize_tag_links<'de, D>(
    deserializer: D,
) -> Result<Vec<AccountTagLink>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<Vec<AccountTagLink>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

// ============================================================
// 分组与标签系统
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountGroup {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub order: i32,
    pub created_at: String,
}

impl AccountGroup {
    pub fn new(name: String, color: Option<String>) -> Self {
        let now: DateTime<Local> = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            order: 0,
            created_at: now.format("%Y/%m/%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTag {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

impl AccountTag {
    pub fn new(name: String, color: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            created_at: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
        }
    }
}

// ============================================================
// 账号标签关联（带时间戳和标签名）
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTagLink {
    pub tag_id: String,
    #[serde(default)]
    pub tag_name: Option<String>,
    pub linked_at: String,
}

impl AccountTagLink {
    pub fn new(tag_id: String, tag_name: Option<String>) -> Self {
        Self {
            tag_id,
            tag_name,
            linked_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

// ============================================================
// 分组与标签存储
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GroupTagData {
    pub groups: Vec<AccountGroup>,
    pub tags: Vec<AccountTag>,
}

pub struct GroupTagStore {
    data: GroupTagData,
    file_path: PathBuf,
}

impl GroupTagStore {
    pub fn new() -> Self {
        let file_path = Self::get_storage_path();
        let data = Self::load_from_file(&file_path);
        Self { data, file_path }
    }

    fn get_storage_path() -> PathBuf {
        let data_dir = dirs::data_dir().unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
        });
        data_dir
            .join(".kiro-account-manager")
            .join("groups-tags.json")
    }

    fn load_from_file(path: &PathBuf) -> GroupTagData {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            GroupTagData::default()
        }
    }

    pub fn try_save_to_file(&self) -> Result<(), String> {
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[GroupTagStore] 创建目录失败: {e}");
                return Err(format!("创建分组标签目录失败: {e}"));
            }
        }
        match serde_json::to_string_pretty(&self.data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.file_path, json) {
                    eprintln!("[GroupTagStore] 写入文件失败: {e}");
                    return Err(format!("写入分组标签文件失败: {e}"));
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("[GroupTagStore] 序列化失败: {e}");
                Err(format!("序列化分组标签数据失败: {e}"))
            }
        }
    }

    // 分组操作
    pub fn get_groups(&self) -> Vec<AccountGroup> {
        self.data.groups.clone()
    }

    pub fn add_group(
        &mut self,
        name: String,
        color: Option<String>,
    ) -> Result<AccountGroup, String> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        // 分组数量不会超过 i32 范围
        let order = self.data.groups.len() as i32;
        let mut group = AccountGroup::new(name, color);
        group.order = order;
        self.data.groups.push(group.clone());
        self.try_save_to_file()
            .map_err(|_| "保存分组失败".to_string())?;
        Ok(group)
    }

    pub fn update_group(
        &mut self,
        id: &str,
        name: Option<String>,
        color: Option<String>,
    ) -> Result<AccountGroup, String> {
        let group = self
            .data
            .groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or("分组不存在")?;
        if let Some(n) = name {
            group.name = n;
        }
        if let Some(c) = color {
            group.color = Some(c);
        }
        let result = group.clone();
        self.try_save_to_file()
            .map_err(|_| "保存分组失败".to_string())?;
        Ok(result)
    }

    pub fn delete_group(&mut self, id: &str) -> Result<bool, String> {
        let len_before = self.data.groups.len();
        self.data.groups.retain(|g| g.id != id);
        let deleted = self.data.groups.len() < len_before;
        if deleted {
            self.try_save_to_file()
                .map_err(|_| "保存分组失败".to_string())?;
        }
        Ok(deleted)
    }

    pub fn reorder_groups(&mut self, ids: &[String]) -> Result<bool, String> {
        for (order, id) in ids.iter().enumerate() {
            if let Some(group) = self.data.groups.iter_mut().find(|g| &g.id == id) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                // 分组数量不会超过 i32 范围
                {
                    group.order = order as i32;
                }
            }
        }
        self.data.groups.sort_by_key(|g| g.order);
        self.try_save_to_file()
            .map_err(|_| "保存分组失败".to_string())?;
        Ok(true)
    }

    // 标签操作
    pub fn get_tags(&self) -> Vec<AccountTag> {
        self.data.tags.clone()
    }

    pub fn add_tag(&mut self, name: String, color: String) -> Result<AccountTag, String> {
        let tag = AccountTag::new(name, color);
        self.data.tags.push(tag.clone());
        self.try_save_to_file()
            .map_err(|_| "保存标签失败".to_string())?;
        Ok(tag)
    }

    pub fn update_tag(
        &mut self,
        id: &str,
        name: Option<String>,
        color: Option<String>,
    ) -> Result<AccountTag, String> {
        let tag = self
            .data
            .tags
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or("标签不存在")?;
        if let Some(n) = name {
            tag.name = n;
        }
        if let Some(c) = color {
            tag.color = c;
        }
        let result = tag.clone();
        self.try_save_to_file()
            .map_err(|_| "保存标签失败".to_string())?;
        Ok(result)
    }

    pub fn delete_tag(&mut self, id: &str) -> Result<bool, String> {
        let len_before = self.data.tags.len();
        self.data.tags.retain(|t| t.id != id);
        let deleted = self.data.tags.len() < len_before;
        if deleted {
            self.try_save_to_file()
                .map_err(|_| "保存标签失败".to_string())?;
        }
        Ok(deleted)
    }
}
