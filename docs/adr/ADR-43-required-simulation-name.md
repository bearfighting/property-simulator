# ADR-43: Required simulation name

- **Status:** Accepted direction
- **Date:** 2026-10-23
- **Related:** [Domain model](../domain-model.md), [ADR-40](ADR-40-separate-core-application-presentation-workspaces.md), [ADR-42](ADR-42-user-settings-and-opening-values.md)
- **Verification:** Core Context construction requires a non-empty `SimulationName`; application defaults remain outside Core.

## Context

Users want to compare results from separately created simulations and need a human-readable label for each simulation. The user may omit a name, while Core must not depend on UI policy or system time.

## Decision

- Core Context contains a required `SimulationName` value, separate from `UserSettings` and `root_key`.
- `SimulationName` stores a non-empty string and trims surrounding whitespace. It is descriptive metadata, not an ID and not required to be unique.
- Presentation may collect an optional name. Before constructing Context, Application resolves a missing name using its creation-time naming policy (currently expected to use the current date/time) and passes a required `SimulationName` to Core.
- Settings updates preserve the simulation name. Core does not read the clock or choose a fallback name.

## Alternatives considered

- Keep an optional name in Core: rejected because it would make every Context consumer handle an unresolved presentation choice.
- Generate a fallback in Core: rejected because Core is independent of system time and application policy.
- Use the name as the simulation identifier: rejected because labels may be duplicated or changed by higher-level product policy.

## Consequences

- Data Access persists the required name alongside each simulation's Context or simulation metadata.
- A future comparison interface can display user-provided or generated labels without treating them as unique keys.
- The exact date/time format and timezone for generated names belong to Application/Presentation design.
