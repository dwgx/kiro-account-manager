use super::machine_id::account_machine_id_or_new;
use super::token_expiry::calc_expires_at;
use crate::auth::providers::AuthProvider;
use crate::auth::providers::{ExternalIdpProvider, IdcProvider, RefreshMetadata, SocialProvider};
use crate::core::account::Account;

/// 判断账号是否为 external_idp（微软 / Azure AD）。
///
/// 口径与导入分类、KiroStudio 对齐：auth_method（大小写不敏感）命中
/// external_idp / external-idp / externalidp / azure / azuread / azure_ad 即判定；
/// 导入时我们落的是 "external_idp"，所以主要看 auth_method。为兜底历史/异常数据，
/// 若带微软 token_endpoint / issuer_url 也视为 external_idp（这类凭据只可能是微软号，
/// 且它们的 clientId/secret 是微软的，绝不能走 AWS OIDC 刷新）。
fn is_external_idp_account(account: &Account) -> bool {
    if let Some(method) = account.auth_method.as_deref() {
        let m = method.trim();
        if m.eq_ignore_ascii_case("external_idp")
            || m.eq_ignore_ascii_case("external-idp")
            || m.eq_ignore_ascii_case("externalidp")
            || m.eq_ignore_ascii_case("azure")
            || m.eq_ignore_ascii_case("azuread")
            || m.eq_ignore_ascii_case("azure_ad")
        {
            return true;
        }
    }
    account
        .token_endpoint
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
        || account
            .issuer_url
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
}

/// Token 刷新结果
#[derive(Debug)]
pub struct RefreshResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub profile_arn: Option<String>,
    pub id_token: Option<String>,
    pub sso_session_id: Option<String>,
    pub machine_id: Option<String>,
}

/// 把 token refresh 结果应用到 Account 上
///
/// 字段更新策略：
/// - access_token：总是覆盖（refresh 一定返回新值）
/// - refresh_token / profile_arn / id_token / sso_session_id：仅当返回了新值才覆盖
///   （避免 social refresh 没返回某字段时把已有的清掉）
/// - expires_at：根据 expires_in 重算
pub fn apply_refreshed_account_tokens(account: &mut Account, refresh: &RefreshResult) {
    account.access_token = Some(refresh.access_token.clone());
    if let Some(refresh_token) = refresh.refresh_token.clone() {
        account.refresh_token = Some(refresh_token);
    }
    if let Some(profile_arn) = refresh.profile_arn.clone() {
        account.profile_arn = Some(profile_arn);
    }
    if let Some(id_token) = refresh.id_token.clone() {
        account.id_token = Some(id_token);
    }
    if let Some(sso_session_id) = refresh.sso_session_id.clone() {
        account.sso_session_id = Some(sso_session_id);
    }
    if let Some(machine_id) = refresh.machine_id.clone() {
        account.machine_id = Some(machine_id);
    }
    account.expires_at = Some(calc_expires_at(refresh.expires_in));
}

async fn refresh_token_by_provider_inner(
    account: &Account,
    use_account_proxy: bool,
) -> Result<RefreshResult, String> {
    let refresh_token = account.refresh_token.as_ref().ok_or("No refresh token")?;

    // external_idp（微软/Azure AD）优先级最高：这类号带微软 clientId/secret + 微软
    // token_endpoint，必须走 OAuth2 refresh_token grant POST 到微软；若落到下面的
    // IdC 分支会打 oidc.*.amazonaws.com，AWS 不认微软 clientId → 400 invalid_request。
    if is_external_idp_account(account) {
        let metadata = RefreshMetadata {
            client_id: account.client_id.clone(),
            client_secret: account.client_secret.clone(),
            region: account.region.clone(),
            profile_arn: account.profile_arn.clone(),
            token_endpoint: account.token_endpoint.clone(),
            issuer_url: account.issuer_url.clone(),
            scopes: account.scopes.clone(),
            account: use_account_proxy.then(|| account.clone()),
            ..Default::default()
        };
        let auth = ExternalIdpProvider::new()
            .refresh_token(refresh_token, metadata)
            .await?;
        return Ok(RefreshResult {
            access_token: auth.access_token,
            refresh_token: Some(auth.refresh_token),
            expires_in: auth.expires_in,
            // profile_arn 原样保留（external_idp 号必须透传真实 arn）
            profile_arn: auth.profile_arn,
            id_token: auth.id_token,
            sso_session_id: None,
            machine_id: None,
        });
    }

    let provider = account.provider.as_deref().unwrap_or("Google");

    if provider == "BuilderId" || provider == "Enterprise" {
        let metadata = RefreshMetadata {
            client_id: account.client_id.clone(),
            client_secret: account.client_secret.clone(),
            region: account.region.clone(),
            account: use_account_proxy.then(|| account.clone()),
            ..Default::default()
        };
        let region = metadata.region.as_deref().unwrap_or("us-east-1");
        // Enterprise 使用保存的 start_url
        let start_url = if provider == "Enterprise" {
            account.start_url.clone()
        } else {
            None
        };
        let idc_provider = IdcProvider::new(provider, region, start_url);
        let auth = idc_provider.refresh_token(refresh_token, metadata).await?;
        Ok(RefreshResult {
            access_token: auth.access_token,
            refresh_token: Some(auth.refresh_token),
            expires_in: auth.expires_in,
            profile_arn: None,
            id_token: auth.id_token,
            sso_session_id: auth.sso_session_id,
            machine_id: None,
        })
    } else {
        let metadata = RefreshMetadata {
            profile_arn: account.profile_arn.clone(),
            machine_id: Some(account_machine_id_or_new(&account.machine_id)),
            account: use_account_proxy.then(|| account.clone()),
            ..Default::default()
        };
        let social_provider = SocialProvider::new(provider);
        let auth = social_provider
            .refresh_token(refresh_token, metadata)
            .await?;
        Ok(RefreshResult {
            access_token: auth.access_token,
            refresh_token: Some(auth.refresh_token),
            expires_in: auth.expires_in,
            profile_arn: auth.profile_arn,
            id_token: None,
            sso_session_id: None,
            machine_id: auth.machine_id,
        })
    }
}

/// 根据 provider 刷新 token（普通账号管理路径，沿用应用通用代理设置）
pub async fn refresh_token_by_provider(account: &Account) -> Result<RefreshResult, String> {
    refresh_token_by_provider_inner(account, false).await
}

/// 根据 provider 刷新 token（Reverse Proxy 路径，使用账号级代理）
pub async fn refresh_token_by_provider_with_account_proxy(
    account: &Account,
) -> Result<RefreshResult, String> {
    refresh_token_by_provider_inner(account, true).await
}
