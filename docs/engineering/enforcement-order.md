# Engineering enforcement order and deferred evidence

Status: v2 release engineering contract.

## Strongest-practical enforcement order

New invariants are pushed through these channels in order:

1. **Closed value/type construction** — enums, exact digests, canonical paths, explicit option values, stable IDs, and terminal lifecycle values.
2. **Pure deterministic validation** — closure, uniqueness, complete surfaces, freshness, role coverage, policy ratchets, no-laundering aggregation, and impact fixed points.
3. **Structural lint/build gates** — `no_std` closure, Dylint FCIS rules, hashing policy, identifier policy, dependency features, denied compiler warnings, and adapter attachment.
4. **Executable positive/negative fixtures** — frozen manifest polarity and stable diagnostic families.
5. **Properties and exhaustive finite models** — permutation/metamorphic checks and exact bounded fixed points.
6. **Selected formal proof** — the smallest integrity obligations under exact bindings and negative controls.
7. **Mutation tripwires** — deliberate faulty implementations for high-consequence paths.
8. **Review records and prose** — rationale and human correspondence where stronger automation cannot decide meaning.

The [identifier boundary](identifier-boundaries.md) defines stable-ID construction and the related repository gate.

A checklist never substitutes for an enforceable type, lint, test, model, or proof that is practical for the invariant. Conversely, a lint hit, annotation, passing test, finite-model result, digest equality, or formal theorem never establishes a property beyond its exact detector, generator, state space, representation, assumptions, and exclusions.

## No intentional duplicate without a derivation

A mirrored table, copied gate, generated source, checked-in report, or equivalent intentional duplicate must have one `Derivation` binding:

- source object and/or raw-artifact digests;
- target artifact digests;
- generator or checker executable identity;
- configuration and template digests;
- declared relation and exact scope;
- consistency oracle; and
- exclusions.

Static freshness and consistency execution remain separate. Byte equality or structural equivalence never establishes behavior. A possible duplicate without a derivation remains advisory debt and cannot be silently dismissed as an implementation detail.

## Phase handoff protocol

`docs/implementation/phase-handoffs.md` defines each phase boundary and focused oracle. A phase checkbox moves only after its focused command is green.

New findings become tracked obligations. They do not weaken fixture polarity or broaden assurance claims.

`devenv test` is the only final acceptance oracle. The resolved Devenv graph runs P0 through P3 once, in order.

Phase scripts do not run child commands. Each focused child belongs to one phase task.

Later phase commands run their complete upstream graph. Duplicate assertions do not count as independent evidence.

## Deferred-evidence ledger

| ID | Deferred evidence | Current handling | Discharge condition |
|---|---|---|---|
| `DEFER-FULL-AENEAS-BACKEND` | Full Mathlib-dependent Aeneas Lean backend replay | Minimal bound `Result`/`U8`/`U64` model; scope explicitly narrow | Offline Nix closure for exact backend dependencies plus complete external-model proof bindings |
| `DEFER-UNICODE-CASEFOLD-AUDIT` | Independent audit of every Unicode portable collision edge | Pinned normalization crate, tested NFC and casefold-sensitive examples | Cross-implementation Unicode corpus and version migration review |
| `DEFER-PARSER-PROOF` | Formal proof of Markdown/TOML/Rust adapters | Parser pins, positive/negative fixtures, spans, mutation controls, total diagnostics | Approved parser model and proof without expanding the core effect boundary |
| `DEFER-COMPILER-CHAIN` | Verified rustc/LLVM/native translation | Explicit exclusion in every source-level proof | Separately bound verified translation/backend evidence |
| `DEFER-PROVIDER-ADAPTERS` | Provider metadata materialization | Provider state excluded from canonical semantics | Separate shell adapter change producing neutral repository-owned records |
| `DEFER-CHELIS-INTEGRATION` | Chelis capability rows and command wiring | Explicitly out of this repository-independent change | Separate OpenSpec change after this final oracle is green |

These entries are advisory and visible. None satisfies a blocking technical obligation or upgrades assurance.

## Stable command surfaces

The generated [quality-assurance status](../assurance/quality-assurance-status.md) and its [maintainer workflow](../assurance/quality-assurance-registry.md) inventory these commands without promoting registration to execution.

- `devenv tasks run buoy:devenv`
- `devenv tasks run buoy:quality`
- `devenv tasks run buoy:fixtures`
- `devenv tasks run buoy:hashing-policy`
- `devenv tasks run buoy:identifier-policy`
- `devenv tasks run buoy:properties`
- `devenv tasks run buoy:core`
- `devenv tasks run buoy:adapters`
- `devenv tasks run buoy:integration`
- `devenv tasks run buoy:obligations`
- `devenv tasks run buoy:p0` through `devenv tasks run buoy:p3`
- `devenv test`

A command rename requires updates to phase handoffs, the workflow policy, downstream references, and negative fixtures.

`buoy-gate-wiring` reads `DEVENV_TASK_FILE`. Source text in a Nix module does not establish final-gate inclusion.

The checker rejects an omitted phase, nested ownership, duplicate ownership, a direct sink child, or a missing lifecycle path.
