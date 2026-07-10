// Kiro IDE 相关功能

// 门面刻意保留完整的 public surface 再导出（模型 / pub API），
// 其中部分符号当前无 crate 内引用（仅对外/未来使用），在二进制 crate 中
// 会触发 unused_imports。这是有意保留、不收窄既有对外接口，故此处放行。
#![allow(unused_imports)]

mod fs_safety;
pub mod import;
pub mod installation;
pub mod switch;
pub mod token;

// ===== 保持 crate::kiro::ide::* 外部路径稳定的再导出 =====

// 本地 token / 客户端注册
pub use token::{
    get_client_registration, get_kiro_local_token, ClientRegistration, KiroLocalToken,
};

// 账号导入
pub use import::{read_kiro_accounts, KiroAccountInfo};

// 切换 / 退出登录
pub use switch::{
    logout_kiro_account, switch_kiro_account, SwitchAccountParams, SwitchAccountResult,
};

// 安装检测
pub use installation::{
    check_ide_installation, check_kiro_config_files, get_kiro_ide_paths, IdeInstallationInfo,
};
