## MODIFIED Requirements

### Requirement: ADV-013 Monotonic local P0-P3 phase handoffs

Local production-readiness acceptance MUST execute P0, P1, P2, and P3 once each through `devenv test`. The resolved Devenv task graph MUST encode this order with explicit dependencies.

Each handoff MUST name its artifacts, identities, focused command, diagnostics, child tasks, and success condition. Each focused child MUST belong to exactly one phase.

The final lifecycle task MUST depend on the phase chain. It MUST NOT execute a focused child directly or contain a duplicate phase sequence.

Separate external phase records MUST NOT be acceptance prerequisites. A later phase MUST NOT hide an earlier absent, stale, or failing obligation.

#### Scenario: Positive - every local phase handoff is current
- **GIVEN** current P0 evidence, P1 evidence, P2 current-host evidence, a closed P3 ledger, and the exact resolved task graph
- **WHEN** exact local final acceptance runs
- **THEN** every phase and owned child executes once in order before production readiness is accepted

#### Scenario: Negative - final command omits fixture execution
- **GIVEN** a green aggregate command whose fixture slice checks only manifest shape and never executes operations
- **WHEN** phase-handoff closure is checked
- **THEN** P0 MUST remain incomplete despite later green tests, proofs, waivers, or reviews

#### Scenario: Negative - final graph omits fixture execution
- **GIVEN** a green aggregate task whose P0 subgraph checks only fixture manifest shape and never executes fixture operations
- **WHEN** phase-handoff closure and the resolved task graph are checked
- **THEN** P0 MUST remain incomplete despite later green tests, proofs, waivers, or reviews
