use crate::core::account::Account;
use uuid::Uuid;

fn has_value(value: Option<&String>) -> bool {
    value.is_some_and(|item| !item.trim().is_empty())
}

fn infer_auth_method(account: &Account) -> Option<String> {
    if account
        .provider
        .as_deref()
        .is_some_and(|provider| provider == "BuilderId" || provider == "Enterprise")
        || (account.client_id.is_some() && account.client_secret.is_some())
    {
        return Some("IdC".to_string());
    }

    if account.profile_arn.is_some()
        || account
            .provider
            .as_deref()
            .is_some_and(|provider| provider == "Google" || provider == "Github")
    {
        return Some("social".to_string());
    }

    None
}

fn normalize_account(account: &mut Account) -> bool {
    let mut changed = false;

    if !has_value(account.auth_method.as_ref()) {
        if let Some(auth_method) = infer_auth_method(account) {
            account.auth_method = Some(auth_method);
            changed = true;
        }
    }

    changed
}

fn same_account_identity(left: &Account, right: &Account) -> bool {
    let left_user_id = left
        .user_id
        .as_ref()
        .map(|value| value.trim())
        .unwrap_or("");
    let right_user_id = right
        .user_id
        .as_ref()
        .map(|value| value.trim())
        .unwrap_or("");
    !left_user_id.is_empty() && !right_user_id.is_empty() && left_user_id == right_user_id
}

fn account_quality_score(account: &Account) -> usize {
    let important_fields = [
        account.email.as_ref(),
        account.user_id.as_ref(),
        account.auth_method.as_ref(),
        account.provider.as_ref(),
        account.access_token.as_ref(),
        account.refresh_token.as_ref(),
        account.expires_at.as_ref(),
        account.client_id.as_ref(),
        account.client_secret.as_ref(),
        account.client_id_hash.as_ref(),
        account.profile_arn.as_ref(),
        account.machine_id.as_ref(),
    ];

    important_fields
        .into_iter()
        .filter(|field| has_value(*field))
        .count()
        + usize::from(account.usage_data.is_some())
        + usize::from(account.available_models_cache.is_some())
}

fn merge_accounts(preferred: &mut Account, candidate: Account) -> bool {
    let mut changed = false;

    macro_rules! fill_option {
        ($field:ident) => {
            if preferred.$field.is_none() && candidate.$field.is_some() {
                preferred.$field = candidate.$field;
                changed = true;
            }
        };
    }

    fill_option!(email);
    fill_option!(password);
    fill_option!(access_token);
    fill_option!(refresh_token);
    fill_option!(expires_at);
    fill_option!(provider);
    fill_option!(user_id);
    fill_option!(auth_method);
    fill_option!(client_id);
    fill_option!(client_secret);
    fill_option!(region);
    fill_option!(client_id_hash);
    fill_option!(sso_session_id);
    fill_option!(id_token);
    fill_option!(start_url);
    fill_option!(profile_arn);
    fill_option!(usage_data);
    fill_option!(group_id);
    fill_option!(machine_id);
    fill_option!(available_models_cache);

    if preferred.tag_links.is_empty() && !candidate.tag_links.is_empty() {
        preferred.tag_links = candidate.tag_links;
        changed = true;
    }

    if preferred.label.trim().is_empty() && !candidate.label.trim().is_empty() {
        preferred.label = candidate.label;
        changed = true;
    }

    if preferred.status.trim().is_empty() && !candidate.status.trim().is_empty() {
        preferred.status = candidate.status;
        changed = true;
    }

    changed
}

pub(crate) fn normalize_accounts(accounts: Vec<Account>) -> (Vec<Account>, bool) {
    let mut changed = false;
    let mut normalized = Vec::with_capacity(accounts.len());

    for mut account in accounts {
        if normalize_account(&mut account) {
            changed = true;
        }

        if let Some(existing_index) = normalized
            .iter()
            .position(|existing| same_account_identity(existing, &account))
        {
            let existing = normalized.remove(existing_index);
            let candidate_score = account_quality_score(&account);
            let existing_score = account_quality_score(&existing);
            let (mut preferred, secondary) = if candidate_score > existing_score {
                changed = true;
                (account, existing)
            } else {
                changed = true;
                (existing, account)
            };

            if merge_accounts(&mut preferred, secondary) {
                changed = true;
            }

            normalized.insert(existing_index, preferred);
            continue;
        }

        normalized.push(account);
    }

    for account in &mut normalized {
        if !has_value(account.machine_id.as_ref()) {
            account.machine_id = Some(Uuid::new_v4().to_string().to_lowercase());
            changed = true;
        }
    }

    let mut machine_id_counts = std::collections::HashMap::<String, usize>::new();
    for account in &normalized {
        if let Some(machine_id) = account.machine_id.as_deref() {
            let normalized_id = machine_id.trim().to_lowercase();
            if !normalized_id.is_empty() {
                *machine_id_counts.entry(normalized_id).or_default() += 1;
            }
        }
    }

    for account in &mut normalized {
        let is_duplicate = account
            .machine_id
            .as_deref()
            .map(|machine_id| {
                let normalized_id = machine_id.trim().to_lowercase();
                machine_id_counts
                    .get(&normalized_id)
                    .copied()
                    .unwrap_or_default()
                    > 1
            })
            .unwrap_or(false);

        if is_duplicate {
            account.machine_id = Some(Uuid::new_v4().to_string().to_lowercase());
            changed = true;
        }
    }

    (normalized, changed)
}

#[cfg(test)]
mod tests {
    use super::normalize_accounts;
    use crate::core::account::Account;

    #[test]
    fn normalize_accounts_fills_missing_auth_method_from_provider() {
        let mut builder = Account::new("builder@example.com".to_string(), "builder".to_string());
        builder.provider = Some("BuilderId".to_string());
        builder.client_id = Some("client-id".to_string());
        builder.client_secret = Some("client-secret".to_string());
        builder.auth_method = None;

        let (normalized, changed) = normalize_accounts(vec![builder]);

        assert!(changed);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].auth_method.as_deref(), Some("IdC"));
    }

    #[test]
    fn normalize_accounts_merges_when_user_id_matches() {
        let mut legacy = Account::new("dup@example.com".to_string(), "legacy".to_string());
        legacy.provider = Some("BuilderId".to_string());
        legacy.user_id = Some("dup-user".to_string());
        legacy.auth_method = Some("IdC".to_string());

        let mut fresh = Account::new("other@example.com".to_string(), "fresh".to_string());
        fresh.provider = Some("Google".to_string());
        fresh.user_id = Some("dup-user".to_string());
        fresh.auth_method = Some("social".to_string());
        fresh.machine_id = Some("machine-1".to_string());

        let (normalized, changed) = normalize_accounts(vec![legacy, fresh]);

        assert!(changed);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].auth_method.as_deref(), Some("social"));
        assert_eq!(normalized[0].machine_id.as_deref(), Some("machine-1"));
        assert_eq!(normalized[0].email.as_deref(), Some("other@example.com"));
    }

    #[test]
    fn normalize_accounts_does_not_merge_when_user_id_differs_even_if_email_matches() {
        let mut social = Account::new("dup@example.com".to_string(), "social".to_string());
        social.provider = Some("Google".to_string());
        social.user_id = Some("user-1".to_string());
        social.auth_method = Some("social".to_string());

        let mut idc = Account::new("dup@example.com".to_string(), "idc".to_string());
        idc.provider = Some("Google".to_string());
        idc.user_id = Some("user-2".to_string());
        idc.auth_method = Some("IdC".to_string());

        let (normalized, changed) = normalize_accounts(vec![social, idc]);

        assert!(changed);
        assert_eq!(normalized.len(), 2);
        assert!(normalized.iter().all(|account| account
            .machine_id
            .as_ref()
            .is_some_and(|id| !id.trim().is_empty())));
    }

    #[test]
    fn normalize_accounts_fills_missing_machine_ids() {
        let mut missing = Account::new("missing@example.com".to_string(), "missing".to_string());
        missing.user_id = Some("missing-user".to_string());
        missing.machine_id = None;

        let mut blank = Account::new("blank@example.com".to_string(), "blank".to_string());
        blank.user_id = Some("blank-user".to_string());
        blank.machine_id = Some("   ".to_string());

        let (normalized, changed) = normalize_accounts(vec![missing, blank]);

        assert!(changed);
        assert_eq!(normalized.len(), 2);
        assert!(normalized.iter().all(|account| account
            .machine_id
            .as_ref()
            .is_some_and(|id| !id.trim().is_empty())));
        assert_ne!(normalized[0].machine_id, normalized[1].machine_id);
    }

    #[test]
    fn normalize_accounts_rotates_duplicate_machine_ids() {
        let mut first = Account::new("first@example.com".to_string(), "first".to_string());
        first.user_id = Some("user-1".to_string());
        first.machine_id = Some("duplicate-machine".to_string());

        let mut second = Account::new("second@example.com".to_string(), "second".to_string());
        second.user_id = Some("user-2".to_string());
        second.machine_id = Some(" DUPLICATE-MACHINE ".to_string());

        let (normalized, changed) = normalize_accounts(vec![first, second]);

        assert!(changed);
        assert_eq!(normalized.len(), 2);
        assert_ne!(normalized[0].machine_id, normalized[1].machine_id);
        assert_ne!(
            normalized[0].machine_id.as_deref(),
            Some("duplicate-machine")
        );
        assert_ne!(
            normalized[1].machine_id.as_deref().map(|id| id.trim()),
            Some("duplicate-machine")
        );
    }
}
