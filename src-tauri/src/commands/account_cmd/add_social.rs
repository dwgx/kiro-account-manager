// 添加 social 账号 + 本地 Kiro 账号导入分发器

use super::add_idc::add_account_by_idc;
use super::AddAccountResult;
use crate::auth::{refresh_token_desktop, User};
use crate::commands::common::{
    extract_user_info, find_existing_account_idx, generate_account_machine_id,
    get_usage_by_provider_with_machine_id, lock_store, save_store, update_account_status,
};
use crate::core::account::Account;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn add_account_by_social(
    state: State<'_, AppState>,
    refresh_token: String,
    provider: Option<String>,
    machine_id: Option<String>,
    access_token: Option<String>,
) -> Result<AddAccountResult, String> {
    let idp = provider.as_deref().unwrap_or("Google").to_string(); // ✅ 避免不必要的 clone
    let account_machine_id = machine_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(generate_account_machine_id);

    // 先尝试用传入的 access_token 获取配额
    let (final_access_token, final_refresh_token, final_profile_arn, usage_result) =
        if let Some(at) = access_token {
            match get_usage_by_provider_with_machine_id(&idp, &at, &account_machine_id).await {
                Ok(result) if result.is_auth_error => {
                    // 401 了，刷新 token
                    let refresh_result = refresh_token_desktop(&refresh_token).await?;
                    let new_usage = get_usage_by_provider_with_machine_id(
                        &idp,
                        &refresh_result.access_token,
                        &account_machine_id,
                    )
                    .await?;
                    (
                        refresh_result.access_token,
                        refresh_result.refresh_token,
                        refresh_result.profile_arn,
                        new_usage,
                    )
                }
                Ok(result) => {
                    // access_token 有效，但没有 profile_arn，需要刷新一次获取
                    let refresh_result = refresh_token_desktop(&refresh_token).await?;
                    (
                        at,
                        refresh_token.clone(),
                        refresh_result.profile_arn,
                        result,
                    )
                }
                Err(e) => return Err(e),
            }
        } else {
            // 没有 access_token，直接刷新
            let refresh_result = refresh_token_desktop(&refresh_token).await?;
            let usage_result = get_usage_by_provider_with_machine_id(
                &idp,
                &refresh_result.access_token,
                &account_machine_id,
            )
            .await?;
            (
                refresh_result.access_token,
                refresh_result.refresh_token,
                refresh_result.profile_arn,
                usage_result,
            )
        };

    // 封禁账号直接报错
    if usage_result.is_banned {
        return Err("BANNED: 账号已被封禁".to_string());
    }

    let (new_email, user_id) = extract_user_info(&usage_result.usage_data);

    // BuilderId 账号允许使用 userId 或 email，如果都没有则用 refreshToken 作为标识
    let final_email = new_email
        .or(user_id.clone())
        .unwrap_or_else(|| format!("builderid_{}", &refresh_token[..8]));

    // 根据邮箱推断最终 provider
    let idp = provider.unwrap_or_else(|| {
        if final_email.contains("gmail") {
            "Google".to_string()
        } else if final_email.contains("github") {
            "Github".to_string()
        } else {
            "Google".to_string()
        }
    });

    let mut store = lock_store(&state.store, "store")?;
    let existing_idx = find_existing_account_idx(
        &store.accounts,
        Some(&final_email),
        &idp,
        &final_refresh_token,
        user_id.as_ref(),
    );

    let is_new = existing_idx.is_none();

    let account = if let Some(idx) = existing_idx {
        let existing = &mut store.accounts[idx];
        // 直接移动所有权，避免 clone
        existing.access_token = Some(final_access_token.clone()); // ✅ 后面还要用，必须 clone
        existing.refresh_token = Some(final_refresh_token.clone()); // ✅ 后面还要用，必须 clone
        existing.profile_arn = Some(final_profile_arn.clone()); // ✅ 保存 profile_arn
        existing.user_id = user_id;
        existing.usage_data = Some(usage_result.usage_data);
        if existing
            .machine_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
        {
            existing.machine_id = Some(account_machine_id.clone());
        }
        update_account_status(existing, usage_result.is_banned, usage_result.is_auth_error);
        existing.clone() // ✅ 必须 clone，因为要返回给前端
    } else {
        let mut account = Account::new(final_email.clone(), format!("Kiro {idp} 账号"));
        account.access_token = Some(final_access_token.clone()); // ✅ 后面还要用，必须 clone
        account.refresh_token = Some(final_refresh_token.clone()); // ✅ 后面还要用，必须 clone
        account.profile_arn = Some(final_profile_arn.clone()); // ✅ 保存 profile_arn
        account.provider = Some(idp.clone());
        account.auth_method = Some("social".to_string());
        account.user_id = user_id;
        account.usage_data = Some(usage_result.usage_data);
        update_account_status(
            &mut account,
            usage_result.is_banned,
            usage_result.is_auth_error,
        );
        // 使用传入的 machine_id，没有则自动生成账号独立 ID
        account.machine_id = Some(account_machine_id);
        store.accounts.insert(0, account.clone());
        account
    };

    save_store(&store)?;
    drop(store);

    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        email: account.email.clone(), // ✅ 必须 clone，因为 account 被移动了
        name: account
            .email
            .as_ref()
            .and_then(|e| e.split('@').next())
            .unwrap_or("User")
            .to_string(),
        avatar: None,
        provider: idp,
    };
    *lock_store(&state.auth.user, "auth user")? = Some(user);
    *lock_store(&state.auth.access_token, "auth access_token")? = Some(final_access_token);

    Ok(AddAccountResult { account, is_new })
}

/// 添加本地 Kiro IDE 账号
#[tauri::command]
pub async fn add_local_kiro_account(
    state: State<'_, AppState>,
) -> Result<AddAccountResult, String> {
    use crate::kiro::ide::{get_client_registration, get_kiro_local_token};

    let local_token = get_kiro_local_token()
        .await
        .ok_or("未找到本地 Kiro 账号，请先在 Kiro IDE 中登录")?;

    let refresh_token = local_token
        .refresh_token
        .ok_or("本地账号缺少 refresh_token")?;

    let auth_method = local_token.auth_method.as_deref().unwrap_or("social");
    let provider = local_token
        .provider
        .clone()
        .unwrap_or_else(|| "Google".to_string());

    // 根据 auth_method 调用对应的添加函数
    if auth_method == "IdC" {
        let hash = local_token
            .client_id_hash
            .clone()
            .ok_or("IdC 账号缺少 clientIdHash")?;
        let region = local_token
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string());

        let client_reg = get_client_registration(&hash)
            .await
            .ok_or(format!("未找到客户端注册信息: {hash}.json"))?;

        // 统一调用 add_account_by_idc（展开参数）
        add_account_by_idc(
            state,
            Some(provider),                   // provider: BuilderId 或 Enterprise
            refresh_token,                    // refresh_token
            client_reg.client_id,             // client_id
            client_reg.client_secret,         // client_secret
            Some(region),                     // region
            None,                             // machine_id: 本地导入不指定，自动生成
            local_token.access_token.clone(), // access_token
            None,                             // password: 本地导入无密码
            None,                             // start_url: 本地导入无 start_url
            Some(hash),                       // client_id_hash: 直接使用 Kiro IDE 提供的
        )
        .await
    } else {
        add_account_by_social(
            state,
            refresh_token,
            Some(provider),
            None,                             // 本地导入不指定 machine_id，自动生成
            local_token.access_token.clone(), // 传入 access_token
        )
        .await
    }
}
