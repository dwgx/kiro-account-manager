use super::response::{
    first_available_profile_arn, normalize_list_available_models_response,
    ListAvailableModelsResponse, ListAvailableProfilesResponse,
};
use crate::commands::common::resolve_kiro_call_context;
use crate::core::account::Account;

#[derive(Debug, Clone)]
pub struct FetchAvailableModelsResult {
    pub response: ListAvailableModelsResponse,
    pub resolved_profile_arn: Option<String>,
}

/// 获取账号可用模型列表（直接使用 KiroClient，无需重复实现）
pub async fn fetch_all_available_models(
    account: &Account,
    access_token: &str,
) -> Result<FetchAvailableModelsResult, String> {
    use crate::clients::kiro_client::KiroClient;

    let ctx = resolve_kiro_call_context(account, "us-east-1");
    let client = KiroClient::new()?;

    let resolved_profile_arn = if ctx.profile_arn.is_some() {
        ctx.profile_arn.clone()
    } else {
        match client.list_available_profiles(access_token, &ctx.region).await {
            Ok(value) => {
                let profiles: ListAvailableProfilesResponse = serde_json::from_value(value)
                    .map_err(|error| format!("解析 ListAvailableProfiles 响应失败: {error}"))?;
                first_available_profile_arn(profiles)
            }
            Err(error) => {
                log::warn!(
                    "[ListAvailableModels] ListAvailableProfiles 兜底失败，继续使用现有 profileArn 解析结果: {}",
                    error
                );
                ctx.profile_arn.clone()
            }
        }
    };

    log::info!(
        "[ListAvailableModels] Account: {} | Provider: {} | ProfileArn (Original): {} | ProfileArn (Used): {}",
        account.id,
        account.provider.as_deref().unwrap_or("None"),
        account.profile_arn.as_deref().unwrap_or("None"),
        resolved_profile_arn.as_deref().unwrap_or("None")
    );

    let response_value = client
        .list_available_models(
            access_token,
            &ctx.machine_id,
            &ctx.region,
            resolved_profile_arn.as_deref(),
        )
        .await?;

    let mut response: ListAvailableModelsResponse = serde_json::from_value(response_value)
        .map_err(|error| format!("解析 ListAvailableModels 响应失败: {error}"))?;

    normalize_list_available_models_response(&mut response);

    Ok(FetchAvailableModelsResult {
        response,
        resolved_profile_arn,
    })
}
