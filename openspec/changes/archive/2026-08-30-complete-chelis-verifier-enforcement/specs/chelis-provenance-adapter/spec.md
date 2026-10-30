## ADDED Requirements

### Requirement: CHELIS-PROV-015 Complete digest enforcement
The static check MUST compare every record-carried binding digest against its current recomputed value. A carrier or link whose bound item digest differs from the current parsed declaration node digest MUST fail with `CHELIS-PROV-ITEM-STALE`. An oracle that declares `runner-config-paths` MUST fail with `CHELIS-PROV-ORACLE-RUNNER-STALE` when its recorded runner digests differ from the current saved artifact bytes. No record field may be carried without enforcement or an explicit nonnormative declaration.

#### Scenario: Positive - current bindings pass
- **GIVEN** records whose item digests and declared runner digests match the current saved bytes
- **WHEN** the static check runs
- **THEN** the verdict is accept with no staleness diagnostic

#### Scenario: Negative - body edit invalidates the bound item
- **GIVEN** a bound declaration whose body bytes changed after the item digest was recorded
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-ITEM-STALE` with the static exit class

#### Scenario: Negative - runner config drift invalidates the oracle
- **GIVEN** an oracle with `runner-config-paths` whose referenced artifact bytes changed
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-ORACLE-RUNNER-STALE` with the static exit class

### Requirement: CHELIS-PROV-016 Actionable staleness contexts
Every digest- or revision-staleness diagnostic MUST carry both the bound value and the current expected value in its deterministic context. A maintainer or tool MUST be able to correct the record from the diagnostic alone, without an out-of-band recomputation build.

#### Scenario: Positive - context names both values
- **GIVEN** any staleness diagnostic from the static check
- **WHEN** the report renders
- **THEN** the context contains the bound digest and the current expected digest in a stable format

#### Scenario: Negative - contexts stay canonical
- **GIVEN** identical stale inputs under two distinct absolute roots
- **WHEN** the static check runs in both
- **THEN** the diagnostic contexts are byte-identical

### Requirement: CHELIS-PROV-017 Deterministic trace projection
The command MUST expose a `trace` subcommand that, for one stable ID, renders the complete digest chain: the atom revision, every surface row, link, and carrier that binds it, each bound item digest with its recomputed current value, and each referenced oracle digest. Every hop MUST be recomputed from saved bytes. The output MUST be canonical machine bytes, and any hop mismatch MUST produce the static exit class.

#### Scenario: Positive - full chain verifies in one command
- **GIVEN** a repository whose bindings are current
- **WHEN** `trace` runs for a bound atom ID
- **THEN** every hop reports its matching recomputed digest and the exit class is success

#### Scenario: Negative - a tampered hop fails the trace
- **GIVEN** one changed byte in the statement or a bound declaration body
- **WHEN** `trace` runs for the affected ID
- **THEN** the mismatched hop is named with bound and current values and the exit class is static rejection

### Requirement: CHELIS-PROV-018 Sanctioned rebind regeneration
The command MUST expose a `rebind` subcommand that recomputes every record-carried digest and revision field from the current graph and prints a deterministic field-level diff. Saved sources MUST change only under an explicit `--write` flag. Without `--write` the command MUST leave every byte unchanged. The rewrite MUST touch only record comment fields, never declarations, and a second run after a write MUST report an empty diff.

#### Scenario: Positive - review-then-write converges
- **GIVEN** records left stale by a reviewed spec edit
- **WHEN** `rebind` runs without and then with `--write`, followed by the static check
- **THEN** the dry run prints the exact field diff, the write applies only those fields, and the static check accepts

#### Scenario: Negative - dry run cannot mutate
- **GIVEN** any repository state
- **WHEN** `rebind` runs without `--write`
- **THEN** every saved byte is unchanged

#### Scenario: Negative - declarations are never rewritten
- **GIVEN** a stale binding on a declaration
- **WHEN** `rebind --write` runs
- **THEN** only `--`-prefixed record fields change and the declaration bytes stay byte-identical
