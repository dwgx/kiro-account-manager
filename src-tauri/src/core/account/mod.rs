// 核心账号模块：值对象 / 实体 / 规范化 / 持久化
// 门面层 pub use 为保持 `crate::core::account::<符号>` 对外路径稳定而全量再导出；
// 其中 GroupTagData / AccountProxyProtocol 当前 crate 内无引用点，但属对外公开 API
// （拆分前是本模块直接 `pub` 的项），必须保留再导出，故允许 unused_imports。
#![allow(unused_imports)]

mod entity;
mod group_tag;
mod normalize;
mod proxy;
mod store;

// 保持 `crate::core::account::<符号>` 对外路径不变
pub use entity::{Account, AvailableModelsCacheEntry};
pub use group_tag::{AccountGroup, AccountTag, AccountTagLink, GroupTagData, GroupTagStore};
pub use proxy::{AccountProxyConfig, AccountProxyProtocol};
pub use store::AccountStore;
