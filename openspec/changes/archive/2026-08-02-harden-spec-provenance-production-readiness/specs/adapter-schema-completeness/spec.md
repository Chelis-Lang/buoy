## ADDED Requirements

### Requirement: ASC-001 Complete Markdown semantic materialization
Every supported Markdown v1 fence for atoms, decision-surface domains/surfaces/dispositions, coverage policies, debt, waivers, reaffirmations, derivations, assumptions, finite models, formal properties, and proof records MUST parse strict closed TOML into the corresponding complete validated core value. Retaining only stable ID plus opaque raw TOML MUST NOT count as semantic materialization.

#### Scenario: Positive - complete waiver fence is parsed
- **GIVEN** a valid waiver fence containing every required obligation, object, approval, lifecycle, discharge, replacement, rationale, owner, and condition field
- **WHEN** Markdown extraction runs
- **THEN** it returns one complete validated `Waiver` value and parser-backed source mapping

#### Scenario: Negative - policy fence contains only an ID
- **GIVEN** a policy fence with a stable ID but missing scope, roles, enforcement, ratchet, authority expectations, lifecycle, and approval
- **WHEN** Markdown extraction runs
- **THEN** it emits deterministic missing-field diagnostics and MUST NOT retain the fence as a generic semantic policy

### Requirement: ASC-002 Complete Rust attachment materialization
Rust carrier, implementation-link, governed-surface, derivation, structural, and verifier attributes MUST bind exact supported parsed-item identity, role, atom revisions, oracle, execution configuration, corpora, claim scope, assumptions, exclusions, navigation/relation fields, and schema version as required by their v1 object types.

#### Scenario: Positive - exact carrier attachment
- **GIVEN** complete carrier metadata attached to one eligible parsed test item
- **WHEN** Rust extraction runs
- **THEN** one complete `EvidenceCarrier` and parsed-item/source-map values are emitted with exact scope and bindings

#### Scenario: Negative - carrier omits exclusions
- **GIVEN** an attribute schema requiring explicit exclusions but the attached carrier omits that field
- **WHEN** extraction runs
- **THEN** extraction fails closed and MUST NOT substitute an empty default

### Requirement: ASC-003 Parser-canonical item identities and spans
Parsed-item identities MUST follow the frozen language/schema, canonical path, item kind, semantic token bytes, and enclosing-item rules. Source spans MUST use original-byte offsets and remain correct for Unicode, CRLF, nested modules, attributes, macros, and parser recovery. Formatting trivia declared nonsemantic MUST preserve item identity.

#### Scenario: Positive - item is reformatted and moved across roots
- **GIVEN** semantically identical eligible Rust items with only formatting changes under the same canonical path in separate absolute roots
- **WHEN** item identity and spans are extracted
- **THEN** semantic item digests match while each original-byte span identifies its exact source

#### Scenario: Negative - macro invocation is treated as expanded item
- **GIVEN** carrier-like metadata inside a macro invocation without an explicitly supported invocation attachment schema
- **WHEN** Rust extraction runs
- **THEN** no expanded carrier is synthesized and the occurrence receives nonnormative or deterministic unsupported treatment

### Requirement: ASC-004 Versioned complete selector and glob semantics
Repository selectors MUST use the frozen versioned portable glob grammar, full selector identity, exact adapter/schema version, include/exclude sets, and authority expectation. Matching and overlap detection MUST be independent of host glob libraries, `.gitignore`, filesystem order, and locale. Two selectors selecting one path MUST fail.

#### Scenario: Positive - portable include and exclude match
- **GIVEN** one selector with valid v1 include/exclude patterns and a canonical snapshot
- **WHEN** matching runs on Linux, macOS, and Windows
- **THEN** the same canonical selected path set is produced

#### Scenario: Negative - selectors share a root but differ semantically
- **GIVEN** two selectors with the same root and overlapping patterns but different adapter or authority expectations
- **WHEN** selection runs
- **THEN** overlap is detected by complete selector/path identity and no selector wins

### Requirement: ASC-005 Total fail-loud disposition
Every selected syntax node and configured syntax class MUST receive a supported complete semantic value, explicit schema-defined nonnormative disposition, or deterministic malformed/unsupported diagnostic. Unknown fields/versions, duplicate keys, malformed authority-like forms, unattached/file-level metadata, disabled items, unsupported item kinds, and required selectors producing no authorities MUST NOT become empty success.

#### Scenario: Positive - ordinary prose and complete fences coexist
- **GIVEN** selected Markdown containing ordinary prose and complete supported semantic fences
- **WHEN** total extraction runs
- **THEN** prose is explicitly nonnormative and every supported fence becomes a complete value

#### Scenario: Negative - unknown semantic field is retained opaquely
- **GIVEN** a supported-version object fence containing an unknown field
- **WHEN** extraction runs
- **THEN** it fails with a deterministic unknown-field diagnostic and MUST NOT hide the field in an opaque payload

### Requirement: ASC-006 Cross-adapter semantic parity
Markdown and Rust adapters MUST map equivalent schema-owned declarations to the same core identities, reference semantics, freshness behavior, and canonical diagnostics independent of extraction order and absolute root. Adapters MUST NOT define separate authority, lifecycle, policy, or assurance rules.

#### Scenario: Positive - extraction order is permuted
- **GIVEN** one saved snapshot containing mutually referencing Markdown and Rust declarations
- **WHEN** adapter order and file traversal order are permuted
- **THEN** the complete canonical graph/report bytes are identical

#### Scenario: Negative - Rust adapter freshens stale metadata
- **GIVEN** a Rust carrier binding a prior atom revision while the Markdown authority contains the current revision
- **WHEN** cross-adapter graph validation runs
- **THEN** the carrier remains stale under core semantics and MUST NOT be repaired by the Rust adapter

### Requirement: ASC-007 Shell-side adapter SDK and consumer-owned UX
The system MAY expose a versioned adapter contract from the imperative shell or a separate `std` shell-side adapter SDK crate. Every configured adapter MUST declare an immutable adapter identity, parser/schema identity, supported syntax capabilities, and exact configuration binding; MUST consume only canonical saved-snapshot paths/bytes and explicit configuration; and MUST emit complete validated core values with parser-backed source maps, explicit schema-defined nonnormative dispositions, or deterministic diagnostics. Adapter discovery and registration order, daemon or editor state, unsaved buffers, dashboards, LSP, MCP, annotations, and watchers MUST NOT alter canonical graph or report identity, and no adapter callback, parser dependency, host-language variant, or developer-interface behavior MAY enter `buoy-core`.

#### Scenario: Positive - shell-side host adapter is deterministic
- **GIVEN** one versioned host adapter receives byte-identical saved snapshots and explicit configuration under different absolute roots and adapter registration orders
- **WHEN** the shell materializes and validates its complete output
- **THEN** the resulting core values, source maps, diagnostics, canonical graph, and reports are byte-identical while downstream UX state remains excluded

#### Scenario: Negative - host item is mislabeled as a frozen item kind
- **GIVEN** a Chelis or other host-language parsed item encoded as `MarkdownFence` or `RustItem` without a separately approved canonical schema version and compatibility mapping
- **WHEN** adapter and parsed-item validation runs
- **THEN** validation fails with a deterministic unsupported-item-schema diagnostic and MUST NOT reinterpret the host item under the frozen kind
