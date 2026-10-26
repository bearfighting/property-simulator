# ADR-69: GPUI page shell and reusable components

- **Status:** Accepted
- **Date:** 2026-10-25
- **Related:** [ADR-67](ADR-67-gpui-composition-root-and-ui-boundary.md), [UI input defaults](../ui-ux-design.md)
- **Scope:** GPUI application shell, page navigation, and presentation components

## Context

The initialization view is the first GPUI workflow. Monthly simulation and
simulation-wide summary views are expected later, and should compose into the
same application window without coupling page rendering to Application or
Data Access.

## Decision

- Keep window creation, active-page navigation, and the persistent application
  header in an application shell.
- Keep each workflow page in its own module. Page state and page layout may be
  split by responsibility when a page contains multiple independent sections.
- Put reusable visual components in a page-neutral components module. Pass
  resolved theme values and callbacks into components rather than making them
  depend on one page's state type.
- Represent navigation with a closed UI route enum. Monthly simulation and
  summary routes may temporarily render placeholders until their workflows are
  implemented.
- Keep the host callback contract in plain presentation values; pages do not
  call Application or Data Access directly.

## Alternatives considered

- Keep all pages and controls in one view module: rejected because adding pages
  would make the initialization workflow and window-level navigation harder to
  change independently.
- Give each page its own window: rejected because the expected product flow is
  navigation among views of one simulation.
- Introduce a general router framework: rejected because a small typed route
  enum is sufficient for the current three-page workflow.

## Consequences

- The application shell owns page selection while individual pages own their
  own form and presentation state.
- New page modules can be added to the shell without moving simulation
  coordination into Presentation.
- The current non-initialization routes are navigation scaffolding and do not
  imply monthly execution or reporting functionality.
