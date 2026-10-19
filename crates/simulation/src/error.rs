use property_simulation_accounting::AccountError;
use property_simulation_domain::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationError {
    InvalidInput(&'static str),
    InvalidProposal(&'static str),
    Domain(DomainError),
    Accounting(AccountError),
    DuplicateOutputId,
}
impl From<DomainError> for SimulationError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}
impl From<AccountError> for SimulationError {
    fn from(e: AccountError) -> Self {
        Self::Accounting(e)
    }
}
