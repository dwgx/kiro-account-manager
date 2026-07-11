// 账号相关命令 - 直接存储原始 usage_data

#![allow(clippy::needless_pass_by_value)] // Tauri 命令需要按值传递 State
#![allow(clippy::too_many_lines)] // 命令文件包含多个函数

mod add_external_idp;
mod add_idc;
mod add_social;
mod crud;
mod models;
mod overage;
mod sync;
mod token_status;
mod verify;

// 再导出全部对外符号，保持 commands::account_cmd::<name> 路径不变
pub use add_external_idp::*;
pub use add_idc::*;
pub use add_social::*;
pub use crud::*;
pub use models::*;
pub use overage::*;
pub use sync::*;
pub use token_status::*;
pub use verify::*;

use crate::commands::account_models::clear_available_models_cache;
use crate::commands::common::RefreshResult;
use crate::core::account::Account;
use serde::{Deserialize, Serialize};

/// 添加账号的返回结果（包含是否新增）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAccountResult {
    pub account: Account,
    #[serde(rename = "isNew")]
    pub is_new: bool, // true = 新增，false = 更新
}

/// account_cmd 的扩展：在通用 token 应用之上额外做缓存清理 / status 重置
fn apply_refreshed_account_tokens(account: &mut Account, refresh: &RefreshResult) {
    clear_available_models_cache(account);
    crate::commands::common::apply_refreshed_account_tokens(account, refresh);
    account.status = "active".to_string();
}
