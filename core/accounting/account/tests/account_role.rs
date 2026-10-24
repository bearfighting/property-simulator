use property_simulation_account::{
    Account, AccountCatalogue, AccountClass, AccountCode, AccountError, AccountRole,
};

fn account(code: u32, role: &str) -> Account {
    Account::new(
        AccountCode::new(code).unwrap(),
        format!("Account {code}"),
        AccountClass::Asset,
        AccountRole::new(role).unwrap(),
    )
    .unwrap()
}

#[test]
fn account_role_is_trimmed_and_resolves_to_one_posting_code() {
    let role = AccountRole::new("  cash  ").unwrap();
    let code = AccountCode::new(100).unwrap();
    let accounts = AccountCatalogue::new(vec![
        Account::new(code, "Cash", AccountClass::Asset, role.clone()).unwrap(),
    ])
    .unwrap();

    assert_eq!(role.as_str(), "cash");
    assert_eq!(accounts.code_for_role(&role), Some(code));
}

#[test]
fn account_catalogue_rejects_duplicate_roles() {
    let error =
        AccountCatalogue::new(vec![account(100, "cash"), account(101, "cash")]).unwrap_err();

    assert!(matches!(error, AccountError::DuplicateRole(_)));
}
