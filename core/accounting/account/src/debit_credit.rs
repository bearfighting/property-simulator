/// Side that increases an account's normal balance or records a transaction entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebitCredit {
    Debit,
    Credit,
}
