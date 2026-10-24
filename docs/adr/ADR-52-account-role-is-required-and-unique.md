# ADR-52: Account role is required and unique

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related:** supersedes the optional-role clause in [ADR-36](ADR-36-ledger-capability-boundaries.md); [ADR-37](ADR-37-slice-journal-entry-contract.md), [Domain model](../domain-model.md)
- **Verification examples:** every `Account` has an `AccountRole`; an `AccountCatalogue` rejects duplicate roles; resolving a role returns its corresponding `AccountCode`; JournalEntry lines continue to use `AccountCode`.

## Context

The Application selects configured accounts for each Slice and passes the posting codes needed to construct journal entries. `AccountClass` describes broad accounting nature, while `AccountRole` names the semantic account used by simulation configuration. These are distinct: a broad class cannot identify a posting account.

## Decision

- Every `Account` has one required `AccountRole` and one `AccountCode`.
- Within an `AccountCatalogue`, each role and each code is unique. The catalogue resolves an `AccountRole` to its corresponding `AccountCode`.
- `AccountClass` remains the broad accounting classification used by reports. It is not a JournalEntry account identifier.
- `AccountRole` is a stable semantic string value suitable for user configuration and persistence. Account names remain display text and are not used for lookup.
- `JournalEntry` and `JournalLine` continue to carry `AccountCode`; Application resolves roles before constructing Slice inputs.

## Alternatives considered

- Make `AccountRole` optional: rejected because configured account roles must resolve to a posting code.
- Identify posting accounts by `AccountClass`: rejected because a broad class may contain multiple accounts and does not identify the intended posting destination.
- Infer roles from account names or codes: rejected because display names and numeric codes do not carry stable semantic meaning.

## Consequences

- `AccountCatalogue` validates both code and role uniqueness and exposes role-to-code lookup.
- Account persistence must retain the role value with its account code. This does not promise schema migration or cross-version compatibility.
- Cash-flow classifications remain separately supplied to Reporting; `AccountRole` does not itself define operating, investing, or financing semantics.
