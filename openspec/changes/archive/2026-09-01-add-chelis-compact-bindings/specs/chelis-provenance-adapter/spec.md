## ADDED Requirements

### Requirement: CHELIS-PROV-019 Content-addressed binding resolution
The Surf grammar MUST accept the `binding` record kind: the marker line plus exactly one `record = <digest>` field, anchored to the next top-level declaration. Any other field set MUST fail with `CHELIS-PROV-MALFORMED-RECORD` and extract no record. Extraction MUST resolve the digest against the configured `binding-store` mount prefix at exactly `<binding-store>/buoy/objects/v1/<algorithm>/<hex>.toml`, MUST verify that the stored bytes hash to the address, and MUST materialize the parsed `buoy/implementation-binding/v1` record into the core graph. A missing store object, a hash mismatch, a malformed stored record, or a build whose active member is not collision resistant MUST fail with `CHELIS-PROV-BINDING-ADDRESS` and the static exit class.

#### Scenario: Positive - one line resolves the full binding
- **GIVEN** a two-line binding record above a declaration and a store object whose bytes hash to the address
- **WHEN** the static check runs
- **THEN** the binding materializes into `graph.implementation_bindings` and the verdict reflects the graph

#### Scenario: Negative - a change-detection member resolves nothing
- **GIVEN** a compact binding in a build whose active digest member is `xxh3-128`
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-ADDRESS` with `algorithm-not-collision-resistant` and materializes no binding

#### Scenario: Negative - tampered store object dangles
- **GIVEN** a store object edited after the address was recorded
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-ADDRESS` with the address and the recomputed hash, and MUST NOT materialize the binding

### Requirement: CHELIS-PROV-020 Binding source currency
A resolved binding whose subject digest differs from the attached declaration MUST fail with `CHELIS-PROV-BINDING-SUBJECT-STALE`. A resolved binding whose requirement digest differs from the exact authority record block MUST fail with `CHELIS-PROV-BINDING-REQUIREMENT-STALE`. Both diagnostics MUST carry bound and current values. The compact form MUST NOT weaken any enforcement the inline form carries.

Shared core validation MUST check each compact binding's atom revision with the same `SPG-REF-DANGLING-OR-STALE` rule as an inline link.

#### Scenario: Positive - subject matches the declaration
- **GIVEN** a binding whose subject digest equals the attached declaration's parsed node digest
- **WHEN** the static check runs
- **THEN** no subject staleness diagnostic appears

#### Scenario: Negative - body edit invalidates the subject
- **GIVEN** an edit to the attached declaration's body after binding
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-SUBJECT-STALE` with bound and current digests

#### Scenario: Negative - authority edit invalidates the atom revision
- **GIVEN** an edit to the bound authority after binding
- **WHEN** the static check runs
- **THEN** shared core validation reports `SPG-REF-DANGLING-OR-STALE` for the compact binding

#### Scenario: Negative - authority formatting invalidates the requirement
- **GIVEN** a formatting-only edit to the bound authority record block
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-REQUIREMENT-STALE` with bound and current digests

### Requirement: CHELIS-PROV-021 Sanctioned conversion and regeneration
The `bind` subcommand MUST convert one inline `implementation-link` record into a compact binding: write the content-addressed object, replace the inline record with the two-line form, and change nothing else. `rebind` MUST regenerate stale bindings by writing the successor object, updating the in-source address, and removing the superseded object. `trace` MUST expand bindings through the store with every hop recomputed. All writes stay behind explicit flags with deterministic dry-run plans, and declarations stay byte-identical.

#### Scenario: Positive - convert, verify, trace
- **GIVEN** an inline implementation link
- **WHEN** `bind --write` converts it and the static check and `trace` run
- **THEN** the graph is semantically unchanged, the source carries only the two-line form, and the trace expands the stored binding with matching hops

#### Scenario: Positive - rebind supersedes the object
- **GIVEN** a binding left stale by a reviewed spec edit
- **WHEN** `rebind` runs without and then with `--write`
- **THEN** the dry run names the object and address updates, the write leaves exactly one current object with a matching in-source address, and a second run reports an empty plan

#### Scenario: Negative - dry runs cannot mutate
- **GIVEN** any repository state
- **WHEN** `bind` or `rebind` runs without `--write`
- **THEN** every saved byte, including the store, is unchanged

#### Scenario: Negative - inline and compact forms cannot coexist
- **GIVEN** one relation ID in both an inline link and a compact binding
- **WHEN** the static check runs
- **THEN** the check reports `CHELIS-PROV-BINDING-LINK-DUPLICATE`
