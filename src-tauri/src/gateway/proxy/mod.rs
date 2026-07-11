use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use chrono::Local;
use futures_util::StreamExt;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    clients::{
        http_client::{
            build_kiro_custom_user_agent, build_kiro_x_amz_user_agent,
            build_streaming_http_client_for_account, resolve_kiro_upstream_region,
            should_add_redirect_for_internal, should_send_codewhisperer_optout,
        },
        kiro_client::{build_generate_assistant_response_url, build_kiro_runtime_host, KiroClient},
    },
    commands::common::{
        account_machine_id_or_new, get_usage_by_account, is_token_expired,
        refresh_token_by_provider_with_account_proxy, resolve_profile_arn_from_candidates,
        update_account_status, RefreshResult,
    },
    core::account::{Account, AccountStore},
};

use crate::gateway::{
    append_gateway_request_log,
    converter::{
        build_kiro_payload, get_available_models, normalize_anthropic_request,
        normalize_openai_chat_payload, normalize_openai_responses_request,
    },
    effective_client_api_keys,
    eventstream::decode_message,
    models::{
        AnthropicContentBlock, AnthropicMessagesRequest, AnthropicMessagesResponse, AnthropicUsage,
        ModelsResponse, NormalizedMessage, NormalizedRequest, OpenAIChatRequest, Tool, ToolCall,
        ToolCallFunction,
    },
    stream::{self, parse_kiro_event_full, KiroEvent},
    thinking_parser::{SegmentType, ThinkingParser},
    GatewayConfig, GatewayRequestLogEntry, ResponseFormat, ResponsesSessionEntry, RouterState,
    DEFAULT_AGENT_MODE,
};

// 目录模块聚合(由 proxy.rs 拆分而来,纯搬迁)。
mod convert;
mod credentials;
mod errors;
mod handlers;
mod logging;
mod request;
mod request_setup;
mod session;
mod streaming;
mod tokens;
mod types;
mod upstream;
mod util;

// 私有 glob 内联:把各子模块的 pub(super) 项拉进 proxy 根作用域,
// 供兄弟子模块 `use super::*;` 与 tests `use super::*;` 访问。
use convert::*;
use credentials::*;
use errors::*;
use logging::*;
use request_setup::*;
use session::*;
use streaming::*;
use tokens::*;
use types::*;
use upstream::*;
use util::*;

// 对外稳定接口:6 个 axum handler,保持 `proxy::<name>` 路径不变。
pub use handlers::{
    anthropic_count_tokens_handler, health_handler, models_handler, openai_chat_handler,
    openai_tokens_handler,
};
pub use request::proxy_handler;

#[cfg(test)]
mod tests;
