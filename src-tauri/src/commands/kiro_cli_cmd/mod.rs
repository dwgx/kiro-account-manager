// kiro-cli 账号导入 / 切号命令
//
// - shared:  expand_home_dir、lock_account_store 等 import/switch 共用工具
// - mapping: KiroCliAccount → 内部 Account 的纯映射/判定
// - import:  从 kiro-cli DB 导入账号
// - switch:  切号 / 回滚 / 退出 / 安装探测 / 快照

// KiroCliImportResult 仅作 import 命令的返回类型对外再导出以保持路径稳定，
// 无显式外部 import，会触发门面 pub use 的 unused_imports；此处按门面约定放行。
#![allow(unused_imports)]

mod import;
mod mapping;
mod shared;
mod switch;

pub use import::{get_kiro_cli_default_path, import_from_kiro_cli, KiroCliImportResult};
pub use switch::{
    check_cli_installation, logout_cli_account, read_cli_db_snapshot, rollback_cli_switch,
    switch_to_cli_account,
};
