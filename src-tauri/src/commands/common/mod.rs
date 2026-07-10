// 公共工具函数 - 提取重复逻辑(按职责拆分为子模块,对外路径不变)
//
// 门面刻意保留完整对外 API 面(pub use 再导出全部子模块公开符号),其中部分符号
// 当前在 crate 内暂无消费者(仅测试/未来/外部潜在使用),拆分前它们是本文件内的
// pub 定义项(不触发告警),拆分后经 pub use 转出会触发 unused_imports。为保持
// "纯搬迁零行为变更、且不收窄对外 API 面"(见拆分方案 R5),此处统一放宽该告警。
#![allow(unused_imports)]

mod account_status;
mod idc_hash;
mod kiro_context;
mod machine_id;
mod profile_arn;
mod store;
mod task;
mod token_expiry;
mod token_refresh;
mod usage;

// ── idc_hash ──
pub use idc_hash::{
    ensure_enterprise_client_id_hash, is_builder_id_client_id_hash, is_builder_id_start_url,
    resolve_idc_client_id_hash, KIRO_BUILDER_ID_CLIENT_ID_HASH, KIRO_BUILDER_ID_START_URL,
};

// ── profile_arn ──
pub use profile_arn::{
    resolve_default_profile_arn, resolve_profile_arn_from_candidates,
    resolve_profile_arn_with_fallback, KIRO_BUILDER_ID_PROFILE_ARN, KIRO_SOCIAL_PROFILE_ARN,
};

// ── machine_id ──
pub use machine_id::{
    account_machine_id_or_new, ensure_account_machine_id, generate_account_machine_id,
};

// ── kiro_context ──
pub use kiro_context::{resolve_kiro_call_context, resolve_machine_id, KiroCallContext};

// ── token_expiry ──
pub use token_expiry::{
    calc_expires_at, is_client_registration_expiring, is_token_expired, is_token_expiring_soon,
    normalize_expires_at, token_needs_refresh, AUTH_TOKEN_INVALIDATION_OFFSET_SECONDS,
    AUTH_TOKEN_REFRESH_BEFORE_EXPIRY_SECONDS, CLIENT_REG_INVALIDATION_OFFSET_SECONDS,
    REFRESH_LOOP_INTERVAL_SECONDS,
};

// ── token_refresh ──
pub use token_refresh::{
    apply_refreshed_account_tokens, refresh_token_by_provider,
    refresh_token_by_provider_with_account_proxy, RefreshResult,
};

// ── usage ──
pub use usage::{
    get_enterprise_usage, get_usage_by_account, get_usage_by_provider_with_machine_id,
    is_auth_error_message, UsageResult,
};

// ── account_status ──
pub use account_status::{
    calc_status, extract_user_info, find_existing_account_idx, update_account_status,
};

// ── store ──
pub use store::{find_account_by_id, lock_store, save_store};

// ── task ──
pub use task::run_blocking_task;
