## ADDED Requirements

### Requirement: SID-001 StableId newtype at one construction boundary
The core MUST represent the uppercase stable object ID grammar with a `StableId` newtype that holds a private normalized string. `StableId` MUST expose one fallible constructor that applies NFC normalization and the frozen stable-id grammar, MUST return the typed value on success, and MUST return a deterministic diagnostic on failure. After construction, an invalid stable ID MUST be unrepresentable. The core MUST also expose a shared identifier error type whose stable-id variant returns the frozen code `SCF-CONSTRUCT-STABLE-ID`.

#### Scenario: Positive - a valid stable ID parses into StableId
- **GIVEN** a string that satisfies the frozen stable-id grammar
- **WHEN** the `StableId` constructor runs
- **THEN** it returns the typed identifier whose canonical text equals the NFC-normalized input

#### Scenario: Negative - an invalid stable ID is rejected at the boundary
- **GIVEN** a string that violates the stable-id grammar or is not NFC
- **WHEN** the `StableId` constructor runs
- **THEN** it fails with the code `SCF-CONSTRUCT-STABLE-ID` and returns no typed value

### Requirement: SID-002 Single parse with no downstream re-validation or bypass
After a stable ID crosses the `StableId` constructor, no model constructor, method, or helper MUST re-run the stable-id grammar check or accept an unvalidated string for a stable-id role. The core MUST NOT expose any constructor that stores a stable ID without a typed proof of validation. A reference or object built from an already-typed `StableId` MUST construct infallibly from the typed value.

#### Scenario: Positive - a typed stable ID builds a reference without re-parsing
- **GIVEN** a `StableId` that already crossed its constructor
- **WHEN** an object reference is built from that typed identifier
- **THEN** construction succeeds infallibly and performs no second grammar check

#### Scenario: Negative - an unchecked string bypass is unavailable
- **GIVEN** a raw string that never crossed the `StableId` constructor
- **WHEN** code attempts to build a model object or reference from it for a stable-id role
- **THEN** the code does not compile, because no constructor accepts an unvalidated string for a stable-id role

### Requirement: SID-003 Stable-id lookups accept only the typed identifier
Every registry or graph lookup keyed by a stable object ID MUST accept the borrowed `StableId` and MUST NOT accept an arbitrary string slice. A prefix comparison over a stable ID MUST take the borrowed `StableId` as its identifier argument.

#### Scenario: Positive - a reservation lookup takes a typed stable ID
- **GIVEN** a namespace registry and a `StableId`
- **WHEN** the reservation lookup is called with the borrowed `StableId`
- **THEN** it returns the allocation class for that identifier

#### Scenario: Negative - a raw slice lookup is unavailable
- **GIVEN** a raw path string that is not a `StableId`
- **WHEN** code attempts the reservation or prefix lookup with that slice
- **THEN** the code does not compile, because the lookup accepts only the borrowed `StableId`

### Requirement: SID-004 Canonical bytes and diagnostic code are preserved
The `StableId` newtype MUST serialize to the identical canonical text bytes as the prior stable-id `String` fields, MUST apply the identical NFC and grammar rules, and MUST return the identical stable diagnostic code `SCF-CONSTRUCT-STABLE-ID`. No object revision digest, object digest, canonical graph identity, migration output, or freshness verdict MUST change as a result of this representation change.

#### Scenario: Positive - regeneration produces a byte-identical diff
- **GIVEN** the committed fixtures and assurance digests before this change
- **WHEN** every canonical artifact is regenerated after stable IDs become `StableId`
- **THEN** the regeneration produces a byte-identical, empty diff

#### Scenario: Negative - a grammar or code drift is rejected
- **GIVEN** a candidate build in which `StableId` normalizes differently or returns a changed diagnostic code
- **WHEN** the stable-id grammar tests and the fixture regeneration oracle run
- **THEN** they fail with the changed byte or code reported and the change is not accepted

### Requirement: SID-005 Deterministic stable-id boundary gate
The imperative shell MUST run a deterministic gate that fails closed if the model source reintroduces an untyped stable-ID boundary. The gate MUST reject any `impl Into<String>` stable-ID parameter on a model constructor, any string bypass for a stable-ID role, and any `require_stable_id` call outside the identifier module. During this phase, the gate MUST allow only `PolicyRatchetImpact::from_validated`, because it accepts the deferred scope-ID role. Findings MUST be normalized, totally ordered, and aggregated fail-closed.

#### Scenario: Positive - a conforming tree yields an empty finding set
- **GIVEN** a model source in which every stable-ID parameter is `StableId`, all stable-ID grammar checks sit inside the identifier module, and only the deferred scope-role bypass remains
- **WHEN** the identifier-boundary gate scans the repository
- **THEN** it succeeds with a deterministic empty finding set

#### Scenario: Negative - a reintroduced string boundary is reported
- **GIVEN** a model constructor that takes `impl Into<String>` for a stable ID, a `from_validated` stable-ID bypass, or a `require_stable_id` call outside the identifier module
- **WHEN** the identifier-boundary gate scans the repository
- **THEN** it emits the applicable stable findings in total order and the aggregate gate fails
