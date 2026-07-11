mod compress;
mod config;
pub(crate) mod converter;
mod eventstream;
pub(crate) mod load_balancer;
pub mod log_store;
mod models;
pub mod prompt_cache;
pub mod prompt_filter;
mod proxy;
mod request_log;
pub mod response_cache;
mod stream;
mod thinking_parser;
mod token_cache;
mod token_estimator;

use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::{Json, Response},
    routing::{get, post},
    Router,
};
use reqwest::Client;
use serde_json::Value;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
use tauri::{AppHandle, Manager};
use tokio::{
    net::TcpListener,
    sync::{oneshot, Mutex as AsyncMutex},
    task::JoinHandle,
};

use crate::clients::http_client::build_streaming_http_client;
use crate::gateway::token_cache::TokenCache;

// 门面再导出：保持 crate::gateway::X 与 super::X 路径稳定（config/request_log 外移后不改对外可达性）。
// ModelMappingRule 仅作公共 API 路径保持,crate 内无直接引用 → 局部 allow 抑制 unused。
#[allow(unused_imports)]
pub use config::{
    get_gateway_config, load_gateway_config, resolve_model_mapping, save_gateway_config,
    GatewayConfig, GatewayRequestStats, GatewayStatus, ModelMappingRule, PromptFilterRule,
};
pub(crate) use config::effective_client_api_keys;
pub use request_log::{
    append_gateway_request_log, GatewayRequestLogEntry, RequestSummary, ResponseSummary, StreamInfo,
};

// mod-root(runtime 簇/tests)使用的 config/request_log 内部项（pub(super)，不经门面对外导出）。
use config::{build_bind_addr, ensure_config_valid, gateway_log_dir_raw, normalize_config};
use request_log::{
    clear_gateway_request_logs_at_path, get_gateway_request_logs_from_path,
    get_gateway_request_stats_from_path, request_log_path,
};

/// 获取可用模型列表（仅用于前端展示，返回 model ID 列表）
pub fn get_available_models() -> Vec<String> {
    converter::get_available_models()
        .into_iter()
        .map(|model| model.id)
        .collect()
}

#[derive(Debug)]
pub struct GatewayRuntime {
    pub config: GatewayConfig,
    pub request_count: Arc<AtomicU64>,
    pub last_error: Arc<AsyncMutex<Option<String>>>,
    pub log_store: Arc<log_store::LogStore>,
    pub response_cache: Arc<AsyncMutex<response_cache::ResponseCache>>,
    pub load_balancer: Arc<load_balancer::LoadBalancer>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    server_task: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResponsesSessionEntry {
    #[allow(dead_code)]
    pub response_id: String,
    pub previous_response_id: Option<String>,
    pub request_messages: Vec<crate::gateway::models::NormalizedMessage>,
    pub request_tools: Option<Vec<crate::gateway::models::Tool>>,
    pub request_tool_choice: Option<serde_json::Value>,
    pub response_text: String,
    pub tool_calls: Vec<(String, String, String)>,
    pub updated_at: Instant,
}

pub(crate) type ResponsesSessionStore = Arc<AsyncMutex<HashMap<String, ResponsesSessionEntry>>>;

#[derive(Clone)]
struct RouterState {
    config: GatewayConfig,
    request_count: Arc<AtomicU64>,
    last_error: Arc<AsyncMutex<Option<String>>>,
    http: Client,
    responses_sessions: ResponsesSessionStore,
    #[allow(dead_code)]
    token_cache: Arc<AsyncMutex<TokenCache>>,
    load_balancer: Arc<load_balancer::LoadBalancer>,
    log_store: Arc<log_store::LogStore>,
    response_cache: Arc<AsyncMutex<response_cache::ResponseCache>>,
}

#[derive(Debug, Clone, Copy)]
enum ResponseFormat {
    Anthropic,
    Responses,
    OpenAI,
}

const DEFAULT_AGENT_MODE: &str = "q-developer-converse";

pub async fn get_gateway_request_logs(
    state: &tauri::State<'_, crate::state::AppState>,
    limit: Option<usize>,
) -> Result<Vec<GatewayRequestLogEntry>, String> {
    // 尝试从运行中的 gateway 的内存存储获取
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        // 从内存存储获取
        let logs = log_store.get_last(limit.unwrap_or(50)).await;
        return Ok(logs);
    }

    // 如果 gateway 未运行，从文件读取
    let path = request_log_path()?;
    get_gateway_request_logs_from_path(&path, limit)
}

pub async fn get_gateway_request_stats(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<GatewayRequestStats, String> {
    // 尝试从运行中的 gateway 的内存存储获取
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        // 从内存存储获取统计
        let stats = log_store.get_stats().await;
        let all_logs = log_store.get_all().await;

        // 计算最大延迟
        let max_duration_ms = all_logs
            .iter()
            .map(|log| log.duration_ms)
            .max()
            .unwrap_or(0);

        return Ok(GatewayRequestStats {
            total: stats.total,
            success: stats.success,
            error: stats.error,
            streaming: stats.streaming,
            total_input_tokens: stats.total_input_tokens as i64,
            total_output_tokens: stats.total_output_tokens as i64,
            total_cache_read_tokens: stats.total_cache_read_tokens as i64,
            total_cache_creation_tokens: stats.total_cache_creation_tokens as i64,
            requests_with_cache: stats.requests_with_cache,
            max_duration_ms,
            avg_duration_ms: stats.avg_duration_ms,
        });
    }

    // 如果 gateway 未运行，从文件读取
    let path = request_log_path()?;
    get_gateway_request_stats_from_path(&path)
}

pub async fn get_gateway_model_stats(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<log_store::ModelStat>, String> {
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        return Ok(log_store.get_model_stats().await);
    }

    Ok(Vec::new())
}

pub async fn get_gateway_endpoint_stats(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<log_store::EndpointStat>, String> {
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        return Ok(log_store.get_endpoint_stats().await);
    }

    Ok(Vec::new())
}

pub async fn clear_gateway_request_logs(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<(), String> {
    // 清空内存存储
    let log_store_opt = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| rt.log_store.clone())
    };

    if let Some(log_store) = log_store_opt {
        log_store.clear().await;
    }

    // 清空文件
    let path = request_log_path()?;
    clear_gateway_request_logs_at_path(&path)
}

pub async fn start_gateway(
    state: &tauri::State<'_, crate::state::AppState>,
    config: GatewayConfig,
) -> Result<GatewayStatus, String> {
    let config = normalize_config(&config);
    ensure_config_valid(&config)?;

    let existing = {
        let mut guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;
        guard.take()
    };

    if let Some(mut rt) = existing {
        stop_runtime(&mut rt).await;
    }

    let runtime = spawn_runtime(config.clone()).await?;
    let status = GatewayStatus {
        running: true,
        host: config.host.clone(),
        port: config.port,
        request_count: 0,
        last_error: None,
        runtime_config: Some(config.clone()),
    };

    let mut guard = state
        .gateway
        .lock()
        .map_err(|_| "获取 gateway 状态失败".to_string())?;
    *guard = Some(runtime);

    Ok(status)
}

pub async fn stop_gateway(state: &tauri::State<'_, crate::state::AppState>) -> Result<(), String> {
    let existing = {
        let mut guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;
        guard.take()
    };

    if let Some(mut rt) = existing {
        stop_runtime(&mut rt).await;
    }

    Ok(())
}

pub async fn get_gateway_status(
    state: &tauri::State<'_, crate::state::AppState>,
) -> Result<GatewayStatus, String> {
    let snapshot = {
        let guard = state
            .gateway
            .lock()
            .map_err(|_| "获取 gateway 状态失败".to_string())?;

        guard.as_ref().map(|rt| {
            (
                rt.config.clone(),
                rt.request_count.load(Ordering::Relaxed),
                rt.last_error.clone(),
                rt.server_task.is_some(),
            )
        })
    };

    if let Some((config, request_count, last_error, running)) = snapshot {
        let last_error_text = last_error.lock().await.clone();
        Ok(GatewayStatus {
            running,
            host: config.host.clone(),
            port: config.port,
            request_count,
            last_error: last_error_text,
            runtime_config: Some(config),
        })
    } else {
        let cfg = load_gateway_config().unwrap_or_default();
        Ok(GatewayStatus::stopped(&cfg))
    }
}

fn router(state: RouterState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/models", get(models_handler))
        .route("/v1/messages", post(anthropic_messages_handler))
        .route(
            "/v1/messages/count_tokens",
            post(anthropic_count_tokens_handler),
        )
        .route("/v1/responses", post(openai_responses_handler))
        .route("/v1/responses/input_tokens", post(openai_tokens_handler))
        .route("/v1/chat/completions", post(openai_chat_handler))
        .with_state(state)
}

async fn spawn_runtime(config: GatewayConfig) -> Result<GatewayRuntime, String> {
    ensure_config_valid(&config)?;

    let request_count = Arc::new(AtomicU64::new(0));
    let last_error = Arc::new(AsyncMutex::new(None));
    let responses_sessions = Arc::new(AsyncMutex::new(HashMap::new()));
    let token_cache = Arc::new(AsyncMutex::new(TokenCache::new()));

    let http = build_streaming_http_client().map_err(|e| format!("初始化 HTTP 客户端失败: {e}"))?;

    // 初始化负载均衡器
    let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
    let load_balancer = Arc::new(load_balancer::LoadBalancer::new(strategy));

    // 初始化内存日志存储（保存最近 10000 条日志）
    let log_store = Arc::new(log_store::LogStore::new(10000));

    // 从文件加载历史日志到内存（启动时恢复）
    if let Ok(path) = request_log_path() {
        if let Ok(history) = get_gateway_request_logs_from_path(&path, Some(500)) {
            let store_clone = log_store.clone();
            tokio::spawn(async move {
                // 历史日志是倒序的（最新在前），需要反转后逐条添加
                for entry in history.into_iter().rev() {
                    store_clone.add(entry).await;
                }
            });
        }
    }

    // 初始化响应缓存
    let cache_config = response_cache::CacheConfig {
        summary_cache_enabled: config.response_cache_enabled,
        summary_cache_max_age_seconds: config.response_cache_ttl,
        ..response_cache::CacheConfig::default()
    };
    let cache_dir = dirs::data_dir().map(|p| p.join(".kiro-account-manager").join("cache"));
    let response_cache = Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
        cache_config,
        cache_dir,
    )));

    let state = RouterState {
        config: config.clone(),
        request_count: request_count.clone(),
        last_error: last_error.clone(),
        http,
        responses_sessions,
        token_cache,
        load_balancer: load_balancer.clone(),
        log_store: log_store.clone(),
        response_cache: response_cache.clone(),
    };

    let app = router(state);
    let addr = build_bind_addr(&config.host, config.port)?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("绑定端口失败: {e}"))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = shutdown_rx.await;
    });

    let server_task = tokio::spawn(async move {
        if let Err(e) = server.await {
            log::error!("网关服务器错误: {e}");
        }
    });

    Ok(GatewayRuntime {
        config,
        request_count,
        last_error,
        log_store,
        response_cache,
        load_balancer,
        shutdown_tx: Some(shutdown_tx),
        server_task: Some(server_task),
    })
}

async fn stop_runtime(runtime: &mut GatewayRuntime) {
    if let Some(tx) = runtime.shutdown_tx.take() {
        let _ = tx.send(());
    }
    if let Some(task) = runtime.server_task.take() {
        let _ = task.await;
    }
}

pub async fn auto_start_if_enabled(app: &AppHandle) -> Result<(), String> {
    let cfg = load_gateway_config()?;
    if !cfg.enabled {
        return Ok(());
    }

    let state = app.state::<crate::state::AppState>();
    let _ = start_gateway(&state, cfg).await?;
    Ok(())
}

pub fn gateway_log_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    gateway_log_dir_raw()
}

pub fn get_gateway_log_dir(app: &AppHandle) -> Result<String, String> {
    gateway_log_dir(app).map(|path| path.to_string_lossy().to_string())
}

pub fn open_gateway_log_dir(app: &AppHandle) -> Result<String, String> {
    let dir = gateway_log_dir(app)?;
    open::that(&dir).map_err(|e| format!("打开日志目录失败: {e}"))?;
    Ok(dir.to_string_lossy().to_string())
}

async fn health_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
) -> Response {
    proxy::health_handler(state, addr, headers).await
}

async fn models_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
) -> Response {
    proxy::models_handler(state, addr, headers).await
}
async fn anthropic_count_tokens_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::anthropic_count_tokens_handler(state, addr, headers, payload).await
}

async fn openai_tokens_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::openai_tokens_handler(state, addr, headers, payload).await
}

async fn anthropic_messages_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::proxy_handler(state, addr, headers, payload, ResponseFormat::Anthropic).await
}

async fn openai_responses_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::proxy_handler(state, addr, headers, payload, ResponseFormat::Responses).await
}

async fn openai_chat_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RouterState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    proxy::openai_chat_handler(state, addr, headers, payload).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::request_log::{set_request_log_path_override, REQUEST_LOG_FILE};
    use axum::body::Body;
    use std::fs;
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue, Method, Request, StatusCode};
    use serde_json::json;
    use std::{
        process,
        sync::{
            atomic::{AtomicU64, Ordering},
            Mutex,
        },
    };
    use tower::util::ServiceExt;

    static REQUEST_LOG_TEST_MUTEX: Mutex<()> = Mutex::new(());
    static REQUEST_LOG_TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct RequestLogTestFixture {
        path: PathBuf,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl RequestLogTestFixture {
        fn new() -> Self {
            let guard = REQUEST_LOG_TEST_MUTEX
                .lock()
                .expect("request log test mutex should lock");
            let dir = std::env::temp_dir().join(format!(
                "kiro-gateway-request-log-test-{}-{}",
                process::id(),
                REQUEST_LOG_TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).expect("request log test dir should create");
            let path = dir.join(REQUEST_LOG_FILE);
            set_request_log_path_override(Some(path.clone()));
            Self {
                path,
                _guard: guard,
            }
        }
    }

    impl Drop for RequestLogTestFixture {
        fn drop(&mut self) {
            set_request_log_path_override(None);
            if let Some(dir) = self.path.parent() {
                let _ = fs::remove_dir_all(dir);
            }
        }
    }

    fn gateway_runtime_test_state() -> RouterState {
        let config = GatewayConfig {
            access_token: Some("sk-test".to_string()),
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            ..GatewayConfig::default()
        };
        let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
        RouterState {
            config,
            request_count: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(AsyncMutex::new(None)),
            http: Client::new(),
            responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
            token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
            load_balancer: Arc::new(load_balancer::LoadBalancer::new(strategy)),
            log_store: Arc::new(log_store::LogStore::new(1000)),
            response_cache: Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
                response_cache::CacheConfig::default(),
                None,
            ))),
        }
    }

    fn auth_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer sk-test"));
        headers
    }

    fn test_router_state() -> RouterState {
        let config = GatewayConfig::default();
        let strategy = load_balancer::LoadBalancerStrategy::from_str(&config.strategy);
        RouterState {
            config,
            request_count: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(AsyncMutex::new(None)),
            http: Client::new(),
            responses_sessions: Arc::new(AsyncMutex::new(HashMap::new())),
            token_cache: Arc::new(AsyncMutex::new(TokenCache::new())),
            load_balancer: Arc::new(load_balancer::LoadBalancer::new(strategy)),
            log_store: Arc::new(log_store::LogStore::new(1000)),
            response_cache: Arc::new(AsyncMutex::new(response_cache::ResponseCache::new(
                response_cache::CacheConfig::default(),
                None,
            ))),
        }
    }

    fn runtime_test_gateway_config(port: u16) -> GatewayConfig {
        GatewayConfig {
            port,
            local_only: false,
            allowed_ips: vec!["127.0.0.1".to_string()],
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        }
    }

    #[tokio::test]
    async fn health_route_is_reachable() {
        let app = router(test_router_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn responses_route_is_reachable() {
        let app = router(test_router_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/responses")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "claude-sonnet-4-5-20250929",
                            "input": [{ "role": "user", "content": "hello" }]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn openai_chat_completions_endpoint_accepts_requests() {
        let app = router(test_router_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "claude-sonnet-4-5-20250929",
                            "messages": [{ "role": "user", "content": "hello" }]
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn lightweight_routes_increment_request_count_and_write_logs() {
        let fixture = RequestLogTestFixture::new();
        let state = gateway_runtime_test_state();
        let client_addr: SocketAddr = "127.0.0.1:4317".parse().expect("socket addr should parse");

        let health = proxy::health_handler(state.clone(), client_addr, auth_headers()).await;
        assert_eq!(health.status(), StatusCode::OK);

        let models = proxy::models_handler(state.clone(), client_addr, auth_headers()).await;
        assert_eq!(models.status(), StatusCode::OK);
        let count_tokens = proxy::anthropic_count_tokens_handler(
            state.clone(),
            client_addr,
            auth_headers(),
            json!({
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{ "role": "user", "content": "hello world" }]
            }),
        )
        .await;
        assert_eq!(count_tokens.status(), StatusCode::OK);

        assert_eq!(state.request_count.load(Ordering::Relaxed), 3);

        let logs = get_gateway_request_logs_from_path(fixture.path.as_path(), Some(10))
            .expect("request logs should read");
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].endpoint, "count_tokens");
        assert_eq!(logs[1].endpoint, "models");
        assert_eq!(logs[2].endpoint, "health");
        assert_eq!(logs[0].status_code, 200);
        assert_eq!(logs[1].status_code, 200);
        assert_eq!(logs[2].status_code, 200);
        assert_eq!(logs[0].outcome, "success");
        assert_eq!(logs[1].outcome, "success");
        assert_eq!(logs[2].outcome, "success");
        assert_eq!(logs[0].client_ip, "127.0.0.1");
        assert!(
            logs[0].request_body.is_none(),
            "request body should not be logged by default"
        );
        assert!(
            logs[0].response_body.is_none(),
            "response body should not be logged by default"
        );
    }

    #[test]
    fn rejects_unsupported_region() {
        let config = GatewayConfig {
            region: "moon-east-1".to_string(),
            ..GatewayConfig::default()
        };

        let err = ensure_config_valid(&config).expect_err("unsupported region should fail");
        assert!(err.contains("region 不受支持"));
    }

    #[test]
    fn rejects_local_account_mode_for_gateway() {
        let config = GatewayConfig {
            account_mode: "local".to_string(),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        };

        let err = ensure_config_valid(&config).expect_err("local mode should fail");
        assert!(
            err.contains("不再支持 local 模式"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn accepts_known_regions() {
        let mut config = GatewayConfig {
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        };
        for region in [
            "us-east-1",
            "eu-central-1",
            "us-west-2",
            "ap-northeast-1",
            "ap-southeast-1",
            "us-gov-west-1",
        ] {
            config.region = region.to_string();
            ensure_config_valid(&config).expect("known region should pass validation");
        }
    }

    #[test]
    fn rejects_remote_access_without_api_key() {
        let config = GatewayConfig {
            local_only: false,
            account_id: Some("test-account".to_string()),
            access_token: None,
            ..GatewayConfig::default()
        };

        let err =
            ensure_config_valid(&config).expect_err("remote access without api key should fail");
        assert!(err.contains("API Key"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_remote_access_without_allowlist() {
        let config = GatewayConfig {
            local_only: false,
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            allowed_ips: Vec::new(),
            ..GatewayConfig::default()
        };

        let err =
            ensure_config_valid(&config).expect_err("remote access without allowlist should fail");
        assert!(err.contains("白名单"), "unexpected error: {err}");
    }

    #[test]
    fn normalize_config_promotes_legacy_access_token_to_client_api_keys() {
        let config = GatewayConfig {
            access_token: Some(" sk-primary ".to_string()),
            client_api_keys: Vec::new(),
            ..GatewayConfig::default()
        };

        let normalized = normalize_config(&config);

        assert_eq!(normalized.client_api_keys, vec!["sk-primary".to_string()]);
        assert_eq!(normalized.access_token.as_deref(), Some("sk-primary"));
    }

    #[test]
    fn normalize_config_deduplicates_client_api_keys() {
        let config = GatewayConfig {
            access_token: Some("sk-primary".to_string()),
            client_api_keys: vec![
                " sk-primary ".to_string(),
                "sk-secondary".to_string(),
                "".to_string(),
                "sk-secondary".to_string(),
            ],
            ..GatewayConfig::default()
        };

        let normalized = normalize_config(&config);

        assert_eq!(
            normalized.client_api_keys,
            vec!["sk-primary".to_string(), "sk-secondary".to_string()]
        );
    }

    #[test]
    fn rejects_config_without_any_client_api_keys() {
        let config = GatewayConfig {
            access_token: Some("   ".to_string()),
            client_api_keys: vec!["".to_string(), "   ".to_string()],
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            ..GatewayConfig::default()
        };

        let err = ensure_config_valid(&normalize_config(&config))
            .expect_err("missing client api keys should fail");
        assert!(err.contains("客户端 API Key"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn runtime_serves_health_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/health"))
            .bearer_auth("sk-test")
            .send()
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_serves_models_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/v1/models"))
            .bearer_auth("sk-test")
            .send()
            .await
            .expect("models request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: Value = response
            .json()
            .await
            .expect("models response should be json");
        assert_eq!(payload.get("object").and_then(Value::as_str), Some("list"));
        assert!(
            payload
                .get("data")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty()),
            "models response should include at least one model"
        );
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_serves_count_tokens_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/messages/count_tokens"))
            .bearer_auth("sk-test")
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "messages": [{ "role": "user", "content": "hello world" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("count tokens request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: Value = response
            .json()
            .await
            .expect("count tokens response should be json");
        assert_eq!(payload.get("input_tokens").and_then(Value::as_u64), Some(2));
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_health_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/health"))
            .send()
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_raw_authorization_header_without_bearer_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/health"))
            .header("Authorization", "sk-test")
            .send()
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_models_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/v1/models"))
            .send()
            .await
            .expect("models request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_count_tokens_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/messages/count_tokens"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "input": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("count tokens request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_responses_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/responses"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "input": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("responses request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_rejects_unauthenticated_messages_requests_over_real_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = runtime_test_gateway_config(port);
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/messages"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "messages": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("messages request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }

    #[tokio::test]
    async fn runtime_requires_client_api_key_even_when_local_only() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener
            .local_addr()
            .expect("local addr should resolve")
            .port();
        drop(listener);

        let config = GatewayConfig {
            port,
            local_only: true,
            account_mode: "single".to_string(),
            account_id: Some("test-account".to_string()),
            access_token: Some("sk-test".to_string()),
            ..GatewayConfig::default()
        };
        let mut runtime = spawn_runtime(config).await.expect("runtime should start");

        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/v1/responses"))
            .header("content-type", "application/json")
            .body(
                json!({
                    "model": "claude-sonnet-4-5-20250929",
                    "input": [{ "role": "user", "content": "hello" }]
                })
                .to_string(),
            )
            .send()
            .await
            .expect("responses request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        stop_runtime(&mut runtime).await;
    }
}
