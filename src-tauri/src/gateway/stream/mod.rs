mod aggregate;
mod citation;
mod event_parser;
mod openai;
mod types;

// 保持 `stream::` 路径稳定 —— proxy.rs 现有全部 `stream::` 引用不用改
pub use aggregate::{aggregate_kiro_response_from_payloads, deduplicate_tool_calls};
pub use event_parser::parse_kiro_event_full;
pub use openai::{build_openai_chunk, build_openai_response};
pub use types::{AggregatedCitation, AggregatedKiroResponse, KiroEvent};
