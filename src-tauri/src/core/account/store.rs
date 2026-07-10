use super::entity::Account;
use super::normalize::normalize_accounts;
use std::path::PathBuf;

pub struct AccountStore {
    pub accounts: Vec<Account>,
    file_path: PathBuf,
    /// 账号文件损坏且无可用备份时置位。此时内存中账号为空，但绝不能把这份空数据
    /// 写回磁盘覆盖掉（可能仍可人工恢复的）损坏文件——`try_save_to_file` 会据此拒绝保存。
    /// 以前这里是 `panic!`，会污染持有 store 的 Mutex，导致后续所有账号操作永久失败。
    load_failed: bool,
}

impl AccountStore {
    pub fn new() -> Self {
        let file_path = Self::get_storage_path();
        let (accounts, load_failed) = Self::load_from_file(&file_path);
        let mut store = Self {
            accounts,
            file_path,
            load_failed,
        };

        if store.normalize_in_place() {
            if let Err(error) = store.try_save_to_file() {
                eprintln!("[AccountStore] 规范化账号文件回写失败: {error}");
            }
        }

        store
    }

    fn get_storage_path() -> PathBuf {
        let data_dir = dirs::data_dir().unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
        });
        data_dir.join(".kiro-account-manager").join("accounts.json")
    }

    fn backup_path_for(path: &PathBuf) -> PathBuf {
        path.with_extension("json.bak")
    }

    fn backup_candidates_for(path: &PathBuf) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        let latest_backup = Self::backup_path_for(path);
        if latest_backup.exists() {
            candidates.push(latest_backup);
        }

        if let Some(parent) = path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                let mut timestamped: Vec<PathBuf> = entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|entry_path| {
                        entry_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(|name| {
                                name.starts_with("accounts.backup-") && name.ends_with(".json")
                            })
                            .unwrap_or(false)
                    })
                    .collect();
                timestamped.sort_by_key(|entry_path| {
                    std::fs::metadata(entry_path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                });
                timestamped.reverse();
                candidates.extend(timestamped);
            }
        }

        candidates
    }

    fn parse_accounts_json(path: &PathBuf, content: &str) -> Result<Vec<Account>, String> {
        serde_json::from_str::<Vec<Account>>(content)
            .map_err(|e| format!("账号文件解析失败: {}; 错误: {e}", path.display()))
    }

    /// 主文件损坏时尝试从备份恢复。
    ///
    /// 返回 `Ok(accounts)` 表示成功从某个备份加载（并已尝试修复主文件）；
    /// 返回 `Err(())` 表示所有备份都不可用——此时**不再 panic**（panic 会污染持有
    /// store 的 Mutex，让后续所有账号操作永久失败），改由调用方置 `load_failed`
    /// 标志、返回空账号，并在保存路径上拒绝覆盖，从而既不丢数据也不崩进程。
    fn load_backup_or_recover(path: &PathBuf, original_error: &str) -> Result<Vec<Account>, ()> {
        let mut backup_errors = Vec::new();
        for backup_path in Self::backup_candidates_for(path) {
            match std::fs::read_to_string(&backup_path)
                .map_err(|e| format!("读取备份失败: {}; 错误: {e}", backup_path.display()))
                .and_then(|content| Self::parse_accounts_json(&backup_path, &content))
            {
                Ok(accounts) => {
                    eprintln!(
                        "[AccountStore] 主账号文件损坏，已从备份加载 {} 个账号: {}",
                        accounts.len(),
                        backup_path.display()
                    );
                    if let Err(error) = std::fs::copy(&backup_path, path) {
                        eprintln!("[AccountStore] 从备份修复主账号文件失败: {error}");
                    }
                    return Ok(accounts);
                }
                Err(error) => backup_errors.push(error),
            }
        }

        eprintln!(
            "[AccountStore] 账号文件已损坏且没有可用备份，已阻止覆盖以避免清空: {original_error}; 备份错误: {}",
            backup_errors.join("; ")
        );
        Err(())
    }

    /// 返回 `(accounts, load_failed)`。`load_failed = true` 表示文件损坏且无备份可恢复，
    /// 内存账号为空但必须禁止回写覆盖。
    fn load_from_file(path: &PathBuf) -> (Vec<Account>, bool) {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                let original_error = format!("无法读取账号文件: {}; 错误: {error}", path.display());
                if !Self::backup_candidates_for(path).is_empty() {
                    eprintln!("[AccountStore] {original_error}，尝试从备份恢复");
                    return match Self::load_backup_or_recover(path, &original_error) {
                        Ok(accounts) => (accounts, false),
                        Err(()) => (Vec::new(), true),
                    };
                }
                // 文件不存在且无备份：全新安装的正常情况，空账号且允许保存
                eprintln!("[AccountStore] 无法读取文件: {}", path.display());
                return (Vec::new(), false);
            }
        };

        match Self::parse_accounts_json(path, &content) {
            Ok(accounts) => {
                eprintln!("[AccountStore] 成功加载 {} 个账号", accounts.len());
                (accounts, false)
            }
            Err(error) => {
                eprintln!("[AccountStore] {error}");
                match Self::load_backup_or_recover(path, &error) {
                    Ok(accounts) => (accounts, false),
                    Err(()) => (Vec::new(), true),
                }
            }
        }
    }

    pub fn save_to_file(&self) -> bool {
        self.try_save_to_file().is_ok()
    }

    fn backup_path(&self) -> PathBuf {
        Self::backup_path_for(&self.file_path)
    }

    fn validate_existing_file_before_save(&self) -> Result<(), String> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| format!("读取现有账号文件失败: {e}"))?;
        serde_json::from_str::<Vec<Account>>(&content).map_err(|e| {
            format!(
                "现有账号文件已损坏，已拒绝覆盖保存以避免数据丢失: {}; 错误: {e}",
                self.file_path.display()
            )
        })?;

        std::fs::copy(&self.file_path, self.backup_path())
            .map_err(|e| format!("备份账号文件失败: {e}"))?;
        Ok(())
    }

    pub fn try_save_to_file(&self) -> Result<(), String> {
        // 加载时文件损坏且无备份可恢复：内存账号为空，绝不能回写覆盖那份可能仍可
        // 人工恢复的损坏文件。直接拒绝保存（配合 validate_existing_file_before_save
        // 形成双保险）。以前这里靠加载时 panic 阻止，改用标志后不再污染 Mutex。
        if self.load_failed {
            return Err(
                "账号文件此前加载失败（已损坏且无可用备份），已拒绝保存以避免覆盖清空。\
                 请先检查并修复账号数据文件，或从备份手动恢复后重启。"
                    .to_string(),
            );
        }

        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[AccountStore] 创建目录失败: {e}");
                return Err(format!("创建账号目录失败: {e}"));
            }
        }

        let json = serde_json::to_string_pretty(&self.accounts).map_err(|e| {
            eprintln!("[AccountStore] 序列化失败: {e}");
            format!("序列化账号数据失败: {e}")
        })?;

        self.validate_existing_file_before_save()?;

        let temp_path = self.file_path.with_extension("json.tmp");
        std::fs::write(&temp_path, json).map_err(|e| {
            eprintln!("[AccountStore] 写入临时账号文件失败: {e}");
            format!("写入临时账号文件失败: {e}")
        })?;

        let backup_path = self.backup_path();
        if self.file_path.exists() {
            std::fs::remove_file(&self.file_path)
                .map_err(|e| format!("替换账号文件前删除旧文件失败: {e}"))?;
        }
        std::fs::rename(&temp_path, &self.file_path).map_err(|e| {
            if !self.file_path.exists() && backup_path.exists() {
                if let Err(restore_error) = std::fs::copy(&backup_path, &self.file_path) {
                    eprintln!("[AccountStore] 替换失败后恢复账号文件失败: {restore_error}");
                }
            }
            let _ = std::fs::remove_file(&temp_path);
            eprintln!("[AccountStore] 替换账号文件失败: {e}");
            format!("替换账号文件失败: {e}")
        })?;

        Ok(())
    }

    pub fn get_all(&self) -> Vec<Account> {
        self.accounts.clone()
    }

    pub fn reload(&mut self) {
        let (accounts, load_failed) = Self::load_from_file(&self.file_path);
        self.accounts = accounts;
        self.load_failed = load_failed;
        self.normalize_in_place();
    }

    fn normalize_in_place(&mut self) -> bool {
        let current = std::mem::take(&mut self.accounts);
        let (normalized, changed) = normalize_accounts(current);
        self.accounts = normalized;
        changed
    }

    pub fn delete(&mut self, id: &str) -> Result<bool, String> {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| a.id != id);
        let deleted = self.accounts.len() < len_before;
        if deleted {
            self.try_save_to_file()?;
        }
        Ok(deleted)
    }

    pub fn delete_many(&mut self, ids: &[String]) -> Result<usize, String> {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| !ids.contains(&a.id));
        let deleted = len_before - self.accounts.len();
        if deleted > 0 {
            self.try_save_to_file()?;
        }
        Ok(deleted)
    }

    pub fn import_from_json(&mut self, json: &str) -> Result<usize, String> {
        match serde_json::from_str::<Vec<Account>>(json) {
            Ok(imported) => {
                let mut added = 0;
                for mut account in imported {
                    // 修复导入账号的 provider（如果为 null）
                    if account.provider.is_none() && account.auth_method.as_deref() == Some("IdC") {
                        // IdC 账号：根据 start_url 或 client_secret（JWT）判断是否 Enterprise
                        // BuilderId: https://view.awsapps.com/start
                        // Enterprise: d-xxx.awsapps.com
                        let is_enterprise = if let Some(ref start_url) = account.start_url {
                            // 有 start_url：检查是否非 BuilderId 的 awsapps.com 域名
                            !crate::commands::common::is_builder_id_start_url(start_url)
                                && start_url.contains("awsapps.com")
                        } else if let Some(ref client_secret) = account.client_secret {
                            // 无 start_url：从 client_secret（JWT）提取 initiateLoginUri 判断
                            use crate::utils::client_id_hash::extract_start_url_from_client_secret;
                            extract_start_url_from_client_secret(client_secret)
                                .map(|url| {
                                    !crate::commands::common::is_builder_id_start_url(&url)
                                        && url.contains("awsapps.com")
                                })
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        account.provider = Some(if is_enterprise {
                            "Enterprise".to_string()
                        } else {
                            "BuilderId".to_string()
                        });
                    } else if account.provider.is_none() && account.auth_method.as_deref() == Some("social") {
                        // Social 账号但 provider 为 null，根据邮箱判断
                        if let Some(ref email) = account.email {
                            if email.contains("gmail") {
                                account.provider = Some("Google".to_string());
                            } else if email.contains("github") {
                                account.provider = Some("Github".to_string());
                            } else {
                                account.provider = Some("Google".to_string());
                            }
                        } else {
                            account.provider = Some("Google".to_string());
                        }
                    }

                    // 修复导入账号的 authMethod（如果为 null）
                    if account.auth_method.is_none() {
                        if account.client_id.is_some() && account.client_secret.is_some() {
                            account.auth_method = Some("IdC".to_string());
                        } else {
                            account.auth_method = Some("social".to_string());
                        }
                    }

                    let exists = self.accounts.iter().any(|a| {
                        if let (Some(a_uid), Some(acc_uid)) = (&a.user_id, &account.user_id) {
                            return a_uid == acc_uid;
                        }

                        false
                    });

                    if !exists {
                        // 如果没有 machine_id，生成一个
                        if account.machine_id.is_none() {
                            account.machine_id =
                                Some(uuid::Uuid::new_v4().to_string().to_lowercase());
                        }
                        self.accounts.push(account);
                        added += 1;
                    }
                }
                self.normalize_in_place();
                self.try_save_to_file()?;
                Ok(added)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    #[allow(dead_code)]
    pub fn export_to_json(&self) -> String {
        serde_json::to_string_pretty(&self.accounts).unwrap_or_default()
    }

    /// 获取可用账号列表（用于自动换号）
    pub fn get_available_accounts(&self) -> Vec<&Account> {
        self.accounts.iter().filter(|a| a.is_available()).collect()
    }

    /// 按分组筛选账号
    pub fn get_accounts_by_group(&self, group_id: &str) -> Vec<&Account> {
        self.accounts
            .iter()
            .filter(|a| a.group_id.as_deref() == Some(group_id))
            .collect()
    }

    /// 按标签筛选账号
    pub fn get_accounts_by_tag(&self, tag_id: &str) -> Vec<&Account> {
        self.accounts
            .iter()
            .filter(|a| a.tag_links.iter().any(|l| l.tag_id == tag_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::AccountStore;
    use crate::core::account::Account;
    use std::path::PathBuf;

    fn unique_test_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kiro-account-store-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("accounts.json")
    }

    fn cleanup_test_path(path: PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn load_from_file_recovers_from_backup_when_primary_json_is_missing() {
        let path = unique_test_path("missing");
        let backup_path = path.with_extension("json.bak");
        let backup_account = Account::new("backup@example.com".to_string(), "backup".to_string());
        std::fs::write(
            &backup_path,
            serde_json::to_string_pretty(&vec![backup_account]).unwrap(),
        )
        .unwrap();

        let (accounts, load_failed) = AccountStore::load_from_file(&path);

        assert!(!load_failed);
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].email.as_deref(), Some("backup@example.com"));
        let repaired = std::fs::read_to_string(&path).unwrap();
        assert!(repaired.contains("backup@example.com"));

        cleanup_test_path(path);
    }
    #[test]
    fn load_from_file_recovers_from_backup_when_primary_json_is_corrupt() {
        let path = unique_test_path("corrupt");
        let backup_path = path.with_extension("json.bak");
        std::fs::write(&path, "[").unwrap();
        let backup_account = Account::new("backup@example.com".to_string(), "backup".to_string());
        std::fs::write(
            &backup_path,
            serde_json::to_string_pretty(&vec![backup_account]).unwrap(),
        )
        .unwrap();

        let (accounts, load_failed) = AccountStore::load_from_file(&path);

        assert!(!load_failed);
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].email.as_deref(), Some("backup@example.com"));
        let repaired = std::fs::read_to_string(&path).unwrap();
        assert!(repaired.contains("backup@example.com"));

        cleanup_test_path(path);
    }

    #[test]
    fn load_from_file_flags_load_failed_on_corrupt_account_json_without_backup() {
        // 主文件损坏且无备份：不再 panic（那会污染 store 的 Mutex），改为返回空账号 +
        // load_failed=true，由 try_save_to_file 拒绝覆盖来保护损坏文件。
        let path = unique_test_path("corrupt-no-backup");
        std::fs::write(&path, "[").unwrap();

        let (accounts, load_failed) = AccountStore::load_from_file(&path);

        assert!(load_failed, "损坏且无备份应置 load_failed");
        assert!(accounts.is_empty(), "加载失败时内存账号应为空");

        // 且此时保存被拒绝，不会用空数据覆盖损坏文件
        let store = AccountStore {
            accounts,
            file_path: path.clone(),
            load_failed,
        };
        let save_result = store.try_save_to_file();
        assert!(save_result.is_err(), "load_failed 时保存必须被拒绝");
        // 损坏文件内容保持不变
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[");

        cleanup_test_path(path);
    }

    #[test]
    fn try_save_to_file_preserves_previous_valid_file_as_latest_backup_only() {
        let path = unique_test_path("backup");
        let mut existing = Account::new("old@example.com".to_string(), "old".to_string());
        existing.user_id = Some("old-user".to_string());
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&vec![existing]).unwrap(),
        )
        .unwrap();

        let mut fresh = Account::new("new@example.com".to_string(), "new".to_string());
        fresh.user_id = Some("new-user".to_string());
        let store = AccountStore {
            accounts: vec![fresh],
            file_path: path.clone(),
            load_failed: false,
        };

        store.try_save_to_file().unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("new@example.com"));
        let backup_path = path.with_extension("json.bak");
        let backup = std::fs::read_to_string(&backup_path).unwrap();
        assert!(backup.contains("old@example.com"));

        let history_backups: Vec<PathBuf> = AccountStore::backup_candidates_for(&path)
            .into_iter()
            .filter(|candidate| {
                candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("accounts.backup-") && name.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            history_backups.is_empty(),
            "regular account saves must not create timestamped backups"
        );

        cleanup_test_path(path);
    }
}
