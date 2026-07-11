//! 网关请求编排:跨子模块共享的数据类型与常量(由 proxy.rs 拆分而来,纯搬迁)。
use super::*;

#[derive(Debug, Clone)]
pub(super) struct UpstreamCredentials {
    pub(super) account_id: String,
    pub(super) access_token: String,
    pub(super) machine_id: String,
    /// 发送正式 Kiro 请求时使用的 profileArn；BuilderId/Social 会按 provider 兜底。
    pub(super) profile_arn: Option<String>,
    /// ListAvailableModels 探测使用的 profileArn。
    ///
    /// BuilderId 账号本地常见为 `profileArn=null`，但真实 IDE 抓包会带固定
    /// BuilderId profileArn；不带时上游会返回 `Invalid profileArn`。
    /// 因此这里使用有效 profileArn（账号/刷新返回值优先，否则 provider 默认值），
    /// 但 machineId 必须仍使用账号自己的 machineId。
    pub(super) available_models_profile_arn: Option<String>,
    pub(super) provider: Option<String>,
    pub(super) region: String,
    pub(super) source_label: String,
    pub(super) user_agent: String,
    #[allow(dead_code)]
    pub(super) auth_method: Option<String>,
    pub(super) send_opt_out: bool,
    pub(super) http: Client,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ResponsesOutputText {
    pub(super) text: String,
    pub(super) annotations: Vec<Value>,
}

pub(super) type UpstreamRequestError = (StatusCode, &'static str, String, Option<String>);

#[allow(dead_code)]
pub(super) const STREAMING_RESPONSE_PLACEHOLDER: &str = "[streaming response omitted from request log]";
