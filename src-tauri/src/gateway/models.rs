// 拆分自原单文件 models.rs 的 barrel 门面。
// kiro_events / openai_responses 的类型是刻意保留的 DTO（原带 #[allow(dead_code)]），
// 当前 crate 内无处经 `models::` 引用它们，故其 `pub use *` glob 会触发 unused_imports。
// 拆分前这些类型直接定义在本文件、无此告警；为保持零新增告警在此整体放宽。
#![allow(unused_imports)]

mod anthropic;
mod common;
mod helpers;
mod kiro;
mod kiro_events;
mod model_list;
mod openai_chat;
mod openai_responses;

// 保持 `crate::gateway::models::<Type>` 外部路径不变
pub use anthropic::*;
pub use common::*;
pub use kiro::*;
pub use kiro_events::*;
pub use model_list::*;
pub use openai_chat::*;
pub use openai_responses::*;
// helpers 不 re-export：外部无人使用（已 grep 确认），保持私有
