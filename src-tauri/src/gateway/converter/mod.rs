// 保留对外/内部公共项的门面再导出以维持 `crate::gateway::converter::xxx` 路径稳定。
// 其中 6 个内部 pub 项当前仅在本模块内部/测试中使用,尚无跨 crate 引用,
// 故门面 `pub use` 会触发 unused_imports;此处显式允许,不收窄公共面。
#![allow(unused_imports)]

mod content;
mod history;
mod images;
mod model_id;
mod normalize;
mod payload;
mod tools;

#[cfg(test)]
mod tests;

// —— 对外稳定接口(被 gateway/mod.rs / proxy.rs / compress.rs 以 converter::xxx 调用)——
pub use payload::build_kiro_payload;
pub use model_id::get_available_models;
pub use normalize::{
    normalize_anthropic_request, normalize_openai_chat_payload, normalize_openai_chat_request,
    normalize_openai_responses_request,
};

// —— 目前仅本 crate 内部/测试使用,但保持公共面稳定,预防未来引用 ——
pub use history::history_assistant_message_from_response_content;
pub use model_id::{get_internal_model_id, get_internal_model_id_with_fallback};
pub use normalize::convert_openai_chat_messages;
pub use tools::{convert_openai_chat_tools, TOOL_DESCRIPTION_MAX_LENGTH};
