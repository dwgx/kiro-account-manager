use crate::commands::machine_guid::generate_random_machine_id;
use crate::core::account::Account;

/// Generate an account-scoped machine ID.
///
/// This deliberately does not read the system machine GUID: stored accounts must
/// have stable IDs without linking multiple accounts on the same host.
pub fn generate_account_machine_id() -> String {
    generate_random_machine_id()
}

fn normalize_account_machine_id(machine_id: &str) -> Option<String> {
    let trimmed = machine_id.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_lowercase())
    }
}

pub fn account_machine_id_or_new(machine_id: &Option<String>) -> String {
    machine_id
        .as_deref()
        .and_then(normalize_account_machine_id)
        .unwrap_or_else(generate_account_machine_id)
}

pub fn ensure_account_machine_id(account: &mut Account) -> String {
    let machine_id = account_machine_id_or_new(&account.machine_id);
    account.machine_id = Some(machine_id.clone());
    machine_id
}

#[cfg(test)]
mod tests {
    use crate::core::account::Account;

    #[test]
    fn account_machine_id_or_new_preserves_existing_account_id() {
        assert_eq!(
            super::account_machine_id_or_new(&Some(" account-machine ".to_string())),
            "account-machine"
        );
    }

    #[test]
    fn account_machine_id_or_new_generates_random_id_for_missing_values() {
        let generated_from_none = super::account_machine_id_or_new(&None);
        let generated_from_blank = super::account_machine_id_or_new(&Some("   ".to_string()));

        assert!(!generated_from_none.trim().is_empty());
        assert!(!generated_from_blank.trim().is_empty());
        assert_ne!(generated_from_none, generated_from_blank);
    }

    #[test]
    fn ensure_account_machine_id_persists_generated_id_on_account() {
        let mut account = Account::new("missing@example.com".to_string(), "missing".to_string());

        let machine_id = super::ensure_account_machine_id(&mut account);

        assert!(!machine_id.trim().is_empty());
        assert_eq!(account.machine_id.as_deref(), Some(machine_id.as_str()));
    }
}
