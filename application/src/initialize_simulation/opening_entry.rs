use property_simulation_account::{AccountCatalogue, DebitCredit};
use property_simulation_journal_entry::{JournalEntry, JournalLine};

use super::types::{InitializeSimulationError, InitializeSimulationRequest};

pub(super) fn build(
    request: &InitializeSimulationRequest,
    accounts: &AccountCatalogue,
) -> Result<JournalEntry, InitializeSimulationError> {
    let mut lines = Vec::with_capacity(request.opening_balances.len());
    for balance in &request.opening_balances {
        let code = accounts
            .code_for_role(&balance.account_role)
            .ok_or_else(|| {
                InitializeSimulationError::MissingAccountRole(balance.account_role.clone())
            })?;
        let account = accounts.get(code).ok_or_else(|| {
            InitializeSimulationError::MissingAccountRole(balance.account_role.clone())
        })?;
        let amount = balance.amount.abs()?;
        if amount.is_zero() {
            continue;
        }

        let normal_side = account.class().normal_balance();
        let side = if balance.amount.is_negative() {
            opposite(normal_side)
        } else {
            normal_side
        };
        lines.push(JournalLine::new(code, side, amount)?);
    }

    Ok(JournalEntry::initial(lines)?)
}

const fn opposite(side: DebitCredit) -> DebitCredit {
    match side {
        DebitCredit::Debit => DebitCredit::Credit,
        DebitCredit::Credit => DebitCredit::Debit,
    }
}
