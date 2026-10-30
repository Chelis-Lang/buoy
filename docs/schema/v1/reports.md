# Version 1 change and report schemas

Status: frozen schema decision for task 2.5. This document defines provider-neutral change inputs and deterministic static, execution, and impact report values. It does not define a GitHub, GitLab, or other provider adapter.

## General report rules

Every report contains canonical facts plus canonical diagnostics. Facts are orthogonal: no renderer may replace several facts with one optimistic summary. Fact sets and diagnostics use the total ordering in `canonical-encoding.md`.

Canonical machine reports use the v1 binary encoding. A deterministic JSON renderer may be added for interchange, but JSON bytes are a derived artifact and never replace the binary identity unless a later schema says so. Human rendering is a deterministic function of canonical values and fixed presentation configuration; terminal width, color detection, locale, and timestamps are noncanonical and disabled in authoritative output.

A report never contains raw process logs, absolute paths, provider users, review timestamps, issue state, random IDs, PIDs, or wall-clock durations.

## Fact status enums

| enum | tags |
|---|---|
| validity | `valid=1`, `invalid=2` |
| completeness | `complete=1`, `incomplete=2`, `invalid=3` |
| freshness | `fresh=1`, `stale=2`, `invalid=3` |
| satisfaction | `satisfied=1`, `unsatisfied=2`, `not-applicable=3` |
| selection | `selected=1`, `not-selected=2`, `invalid=3` |
| closure | `closed=1`, `open=2`, `invalid=3` |
| consistency | `not-run=1`, `consistent=2`, `inconsistent=3`, `error=4`, `timeout=5` |

These tags serialize values only. They are not general severity rankings.

## Static report facts

### `AuthorityFact`

Fields:

1. atom or tombstone subject reference;
2. validity;
3. lifecycle state;
4. authority-source count;
5. approval validity;
6. stable diagnostic-code set.

A proposed atom can have valid sole authority while remaining ineligible for a ratified-authority obligation.

### `SurfaceFact`

Fields:

1. decision-surface reference;
2. domain digest;
3. completeness;
4. expected member count;
5. actual member count;
6. missing member-key set;
7. duplicate member-key set;
8. unknown member-key set;
9. members lacking required atom authority set;
10. diagnostic-code set.

Counts never substitute for key comparison: an unknown key plus a missing key remains incomplete even when counts match.

### `RegistrationFact`

Fields:

1. evidence-carrier reference;
2. parsed-item identity digest;
3. role;
4. atom-revision set;
5. declared scope digest;
6. registration validity;
7. diagnostic-code set.

Registration says only that syntax-attached potential evidence exists.

### `PolicyFact`

Fields:

1. policy reference;
2. subject reference;
3. obligation reference;
4. enforcement mode;
5. applicability validity;
6. satisfaction;
7. satisfying carrier/reference set;
8. active debt-reference set;
9. active waiver-reference set;
10. diagnostic-code set.

Debt and waiver sets never change `satisfaction` from unsatisfied to satisfied.

### `FreshnessFact`

Fields:

1. subject reference;
2. freshness;
3. expected binding digest set;
4. current binding digest set;
5. stale binding-key set;
6. diagnostic-code set.

Carrier, derivation, property, model, assumption-dependent value, and proof freshness use this shared shape with kind-specific binding keys.

### `DebtWaiverFact`

Fields:

1. debt or waiver reference;
2. validity;
3. lifecycle state;
4. obligation-reference set;
5. unsatisfied obligation-reference set;
6. owner ID;
7. discharge-condition digest;
8. discharge-evidence reference set;
9. replacement reference set;
10. diagnostic-code set.

A valid waiver can coexist only with visible unsatisfied obligations; otherwise the report is invalid because it would hide what was waived.

### `DerivationFact`

Fields:

1. derivation reference;
2. static freshness;
3. source-binding set;
4. target-binding set;
5. relation;
6. consistency-oracle reference;
7. consistency execution status;
8. declared scope digest;
9. exclusions;
10. diagnostic-code set.

Static freshness and consistency execution are independent.

### `ProofFact`

Fields:

1. proof-record reference;
2. registration validity;
3. correspondence-approval validity;
4. binding completeness;
5. freshness;
6. verifier selection;
7. verifier verdict;
8. negative-fixture satisfaction;
9. exact scope digest;
10. assumptions and exclusions;
11. formally-proved satisfaction;
12. diagnostic-code set.

`formally-proved` satisfaction is true only when every required fact permits it.

## `StaticReport`

Fields are fixed by the `static-report` digest domain:

1. complete graph digest;
2. authority facts;
3. surface facts;
4. registration facts;
5. policy facts;
6. freshness facts;
7. debt/waiver facts;
8. derivation facts;
9. proof facts;
10. canonical diagnostics.

A static report contains no oracle pass invented from discovery. Blocking static acceptance succeeds only when graph validity, authority, surface completeness, registration freshness, and static policy obligations are satisfied or the policy explicitly defines a process waiver path. The waiver path never changes the underlying facts.

## Execution values

### `NormalizedVerdictSet`

Contains one verdict per selected oracle/subject tuple. Duplicate tuples are invalid. A required tuple absent from the set remains `not-run`; it is not silently omitted from the execution report.

### `EvidenceFact`

Fields:

1. policy obligation reference;
2. carrier/reference set;
3. registration validity;
4. freshness;
5. selection;
6. verdict set;
7. exact claim-scope digest;
8. assumptions and exclusions;
9. assurance-class set actually established;
10. satisfaction;
11. diagnostic-code set.

A point-scope passing test can establish only `tested` for that exact scope. Exhaustive, formal, structural, and review evidence remain separate facts.

## `ExecutionReport`

Fields:

1. static semantic or complete-graph digest selected by report configuration;
2. normalized verdict-set digest;
3. evidence facts;
4. canonical diagnostics.

The static graph digest cannot change because a runner passes, fails, crashes, or times out. Two executions with the same normalized verdict set produce byte-identical execution reports regardless of completion order or raw logs.

## Provider-neutral change values

### `ChangeManifest`

Contains:

- stable manifest ID;
- exact base complete-graph digest;
- exact head complete-graph digest;
- canonical change-disposition set; and
- optional repository-owned approval binding.

The manifest is a repository artifact supplied explicitly to the core. A shell adapter may generate a candidate from a provider, but canonical acceptance uses the materialized bytes and bindings only.

A manifest with equal base/head digests is allowed only with an empty disposition set. A manifest cannot refer to a third graph.

### `ChangeDisposition`

Contains disposition ID, impacted object kind and stable ID, optional base/head digests, disposition, optional reaffirmation digest, optional waiver reference, and replacement-reference set.

Validity by disposition:

- `reaffirmed`: base and head digests are present and different; object kind and stable ID are unchanged; exactly one valid reaffirmation is present; waiver and replacements are absent.
- `revised`: base and head digests are present and different; reaffirmation and waiver are absent; replacements are empty when stable identity remains and nonempty only when the impacted schema permits split/merge revision.
- `retired`: base digest is present; head digest and reaffirmation are absent; replacement references follow the retired object's lifecycle schema; waiver is absent.
- `waived`: base and/or head digest matches the actual impact; exactly one current waiver is present; reaffirmation is absent; replacements are absent.

A disposition is inapplicable if the base/head graph pair does not produce the identified impact. Missing, duplicate, stale, and inapplicable dispositions are errors.

### `ImpactSet`

Contains direct changes, transitive impacts, and policy-ratchet impacts.

Direct changes are determined by canonical kind/ID key comparison and object digests. Transitive impact follows only schema-declared edges, including:

- Atom revisions bind external trace links, carriers, implementation links, surface dispositions, policies, properties, models, assumptions, and proofs.
- object/artifact binding to derivations and proofs;
- policy revision to its obligations, debt, and waivers;
- assumption revision to every dependent carrier, model, property, oracle configuration, and proof;
- oracle/tool revision to carriers, derivations, models, and proofs; and
- domain revision to its decision surface, member dispositions, policies, and evidence.

Traversal continues to a fixed point. Visit order is nonsemantic. The final set is canonical and duplicate-free.

### `DispositionClosureFact`

Fields:

1. impacted object kind and stable ID;
2. base digest option;
3. head digest option;
4. direct-or-transitive enum;
5. required disposition class set;
6. matching change-disposition reference option;
7. closure;
8. review-only reaffirmation fact option;
9. waiver fact option;
10. diagnostic-code set.

Reaffirmation and waiver remain orthogonal to freshness and assurance.

## `ImpactReport`

Fields:

1. base complete-graph digest;
2. head complete-graph digest;
3. change-manifest digest;
4. impact-set digest;
5. disposition-closure facts;
6. canonical diagnostics.

Impact acceptance fails when any required impact lacks exactly one valid disposition or when a policy ratchet weakens without the policy-defined reviewed migration. Provider metadata does not affect the result.

## Assurance aggregation

Aggregation emits a set of scoped assurance classes, never one maximum level.

- `linked`: valid fresh registration for the exact atom revision.
- `tested`: required fresh test carriers selected and passing for the exact declared scope.
- `exhaustively-checked`: a declared finite model reached a fixed point under its exact digest and bounds.
- `formally-proved`: a complete fresh proof under exact accepted bindings and explicit trust/exclusions.
- `structurally-enforced`: a selected structural/consistency oracle passed for its declared checker scope.
- `review-only`: a valid approval or reaffirmation exists for its exact review scope.

No class implies another unless the report shows the independent facts for both. A waiver, debt record, implementation link, unselected carrier, discovered test, or passing sibling cannot create a class.

## Stable diagnostic schema

Every diagnostic contains:

1. stable ASCII diagnostic code;
2. path and source span option;
3. primary object kind option;
4. primary stable ID option;
5. structured field record under the diagnostic-code schema;
6. severity enum `advisory=1` or `blocking=2`; and
7. noncanonical raw-detail handle option excluded from canonical output.

The code, location, primary identity, structured fields, and severity determine canonical rendering. Unknown diagnostic fields are invalid. Raw operating-system, parser, runner, and verifier text is retained outside the canonical report.

## Required diagnostic families

V1 reserves stable families for:

- `SPG-AUTH-*`: authority, lifecycle, and tombstone failures;
- `SPG-REF-*`: dangling or wrong-kind references;
- `SPG-SURFACE-*`: domain, member, disposition, and authority failures;
- `SPG-DERIVATION-*`: binding and relation failures;
- `SPG-ADAPTER-*`: malformed, unsupported, or vacuous selected inputs;
- `DET-PATH-*`, `DET-ENC-*`, `DET-DUP-*`, and `DET-ORDER-*`;
- `EVD-POLICY-*`, `EVD-FRESH-*`, `EVD-EXEC-*`, and `EVD-LAUNDER-*`;
- `FPR-BIND-*`, `FPR-TRUST-*`, and `FPR-VERDICT-*`; and
- `CHG-IMPACT-*`, `CHG-DISPOSITION-*`, and `CHG-RATCHET-*`.

Exact individual codes and structured field schemas are frozen with the positive and negative fixture manifest before implementation.
