// Kiro CLI 集成：数据模型 / sqlite auth_kv 操作 / 环境检测

// 门面 pub use 保持 `crate::kiro::cli::*` 外部路径不变；部分再导出（如 DTO 字段类型、
// 无外部调用者的检测函数）在本 crate 内暂无直接消费者，允许 unused_imports 以维持 API 稳定。
#![allow(unused_imports)]

mod db;
mod detect;
mod types;

// 再导出以保持外部路径 `crate::kiro::cli::*` 不变
pub use types::{
    DeviceRegistration, KiroCliAccount, KiroCliAuthEntry, KiroCliDbSnapshot, KiroCliSwitchPayload,
    KiroCliWriteBackup, TokenData,
};

pub use db::{
    logout_cli_account, read_cli_db_snapshot, read_kiro_cli_accounts, rollback_cli_switch,
    switch_cli_account,
};

pub use detect::{
    check_cli_installation, detect_cli_database, detect_cli_executable, CliInstallationInfo,
};
