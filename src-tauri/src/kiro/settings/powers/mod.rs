// Powers 管理（v0.10.32 registry-v2: ~/.kiro/powers/）

// 保留全部 pub 类型的重导出以维持与拆分前单文件“全 pub 可见”一致的对外可见性;
// 其中部分类型当前无外部消费者,故对未使用的重导出放行(与拆分前语义等价)。
#![allow(unused_imports)]

mod lifecycle;
mod manager;
mod metadata;
mod models;
mod registry;
mod validation;

// 重导出:保持 `crate::kiro::settings::powers::{PowerInfo, PowersManager,
// RecommendedPower, RegistryInfo, ...}` 外部路径不变。
pub use manager::PowersManager;
pub use models::{
    DismissedEntry, InstalledPowerEntry, InstalledPowersFile, PowerFrontMatter, PowerInfo,
    RecommendedPower, RegistryInfo,
};
