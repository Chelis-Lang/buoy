# Version 1 fixture corpus

This directory is test support, not a product input format. It defines immutable pre-implementation examples consumed by the phase-3 reference runner.

## Case files

Each requirement file contains one or more `[[case]]` tables with closed fields:

- `id`: globally unique lowercase fixture ID;
- `requirement`: exact OpenSpec requirement ID;
- `scenario`: exact scenario slug;
- `polarity`: `positive` or `negative` relative to the requirement claim;
- `operation`: one closed runner operation;
- `baseline`: repository-relative path to a concrete fixture baseline;
- `mutations`: ordered array of mutation IDs, empty when none; and
- `expected`: one inline table with `outcome`, stable `codes`, and asserted `facts`.

`outcome` is `accept` or `reject`. A negative scenario can still expect `accept` when the required behavior is non-extraction or an unsatisfied fact rather than malformed input.

Case input order is never semantically meaningful. Mutation order is meaningful and is listed explicitly.

## Baselines

A baseline is strict fixture-only TOML with:

- `schema = "spec-provenance-fixture-baseline/v1"`;
- a stable baseline `id`;
- optional ordered `includes` of other baseline paths (acyclic, duplicate object keys rejected);
- optional repository files under `[[files]]`; and
- optional direct core values under object-specific arrays.

For the `architecture-gate` operation only, `baseline` may name an existing Cargo fixture directory under `fixtures/dylint/` instead of a TOML file.

The fixture runner rejects unknown fields. Direct values are complete logical schema values with symbolic digests resolved before the operation.

## Symbolic values

The fixture layer defines these deterministic symbols:

- `$digest:<domain>:<logical-id>`: digest the named current logical value under the frozen v1 encoder;
- `$raw:<path>`: raw-artifact digest for the named baseline file;
- `$approval:<authority-id>:<logical-id>`: construct a repository-owned approval binding from the baseline's generated review record; and
- `$item:<path>:<item-name>`: parser-canonical item identity from the named source item.

Symbols are permitted only in fixture baselines. Expanded adapter source must contain the exact product spelling before the production parser sees it. An unknown or cyclic symbol is a fixture-runner error, not an expected product diagnostic.

## SPG operations

- `extract-markdown`: parse selected Markdown and return authorities/source facts.
- `extract-rust`: parse selected Rust and return carrier/link facts.
- `build-graph`: construct and validate a graph from baseline values and extracted values.
- `transition-lifecycle`: compare declared base/head lifecycle values.
- `evaluate-coverage`: build the graph and evaluate static role coverage.
- `evaluate-freshness`: build the graph and evaluate carrier/derivation freshness.
- `validate-surface`: validate domain, member, disposition, and authority completeness.
- `validate-adapter-totality`: require semantic/nonnormative/diagnostic disposition for selected files.
- `render-static`: construct canonical graph and static-report bytes.
- `normalize-snapshot`: normalize and validate repository paths/bytes.
- `encode-canonical`: produce domain-separated preimages and full identities.
- `render-execution`: combine one static graph with normalized verdicts.
- `render-diagnostics`: serialize structured canonical and noncanonical diagnostics.
- `architecture-gate`: run the existing FCIS positive/negative fixture oracle.
- `compute-impact`: compare an explicit base/head graph and neutral manifest.
- `validate-impact-closure`: require one current disposition per policy-required impact.
- `report-evidence`: emit independent evidence and lifecycle facts.
- `evaluate-required-roles`: materialize and check policy role obligations.
- `aggregate-assurance`: aggregate only facts valid for one exact claim scope.
- `normalize-verdict`: map one selected runner result to the closed verdict enum.
- `run-finite-model`: explore the declared finite model to a fixed point.
- `compare-frontends`: compare CLI and LSP semantic diagnostics before presentation.
- `compare-policy-adoption`: compare base/head materialized policy obligations.
- `validate-formal-property`: validate correspondence, scope, assumptions, and atom bindings.
- `validate-proof-record`: validate complete proof-record closure.
- `evaluate-proof-freshness`: compare every recorded proof binding to current inputs.
- `aggregate-formal-assurance`: require correspondence, freshness, verifier acceptance, and negative controls.
- `render-proof-trust`: render exact tools, representations, assumptions, and exclusions.
- `evaluate-proof-eligibility`: enforce the safe sequential verified-kernel subset.
- `run-formal-oracle`: normalize one fully pinned offline formal invocation.
- `select-formal-lane`: compare equivalent spikes or a reviewed infeasibility blocker.

## SPG mutations

The closed task-3.1 mutation vocabulary is:

- `duplicate-atom-authority`
- `move-atom-authority`
- `edit-atom-statement-keep-bindings`
- `supersede-ratified-atom`
- `reuse-tombstoned-atom-id`
- `attach-carrier-to-test`
- `replace-carrier-with-mentions`
- `set-carrier-oracle-unknown`
- `remove-all-evidence-carriers`
- `prefix-authority-with-multibyte-unicode`
- `replace-authority-with-literal-fenced-example`
- `remove-surface-member`
- `duplicate-surface-member`
- `replace-surface-member-with-unknown`
- `remove-disposition-authority`
- `change-derivation-target-keep-binding`
- `replace-adapter-version-with-unsupported`
- `remove-required-root-authorities`

The closed task-3.2 mutation vocabulary is:

- `permute-repository-traversal`
- `vary-ambient-timezone-locale-host-root-provider`
- `collide-normalized-paths`
- `replace-path-with-absolute`
- `replace-path-with-escaping-parent`
- `replace-path-with-platform-ambiguous-form`
- `permute-canonical-collection`
- `duplicate-canonical-key`
- `mutate-digest-bound-field`
- `encode-ambiguous-concatenation-pair`
- `reinterpret-sha256-identity-under-other-algorithm`
- `replace-identity-with-truncated-digest`
- `replace-crlf-with-lf`
- `edit-normative-prose-by-one-byte`
- `replace-nfc-with-decomposed-equivalent`
- `permute-verdict-completion`
- `timeout-one-oracle`
- `relocate-invalid-repository`
- `add-raw-log-host-data`
- `relocate-graph-pair`
- `vary-provider-metadata`
- `permute-impact-inputs`
- `remove-one-impact-disposition`
- `duplicate-one-impact-disposition`
- `stale-one-impact-disposition`

The closed task-3.3 mutation vocabulary is:

- `leave-oracles-not-run`
- `attempt-linked-to-tested-upgrade`
- `register-positive-and-negative-roles`
- `rename-positive-carrier-like-negative`
- `remove-negative-role`
- `select-oracle-pass`
- `select-oracle-not-run`
- `select-oracle-fail`
- `select-oracle-error`
- `select-oracle-timeout`
- `set-all-required-verdicts-pass`
- `timeout-one-required-carrier`
- `accept-structural-checker`
- `label-exhaustive-model-as-formal`
- `explore-model-to-fixed-point`
- `stop-model-at-depth-cutoff`
- `compare-cli-lsp-saved-snapshot`
- `supply-unsaved-green-editor-result`
- `grow-blocking-policy-scope`
- `remove-required-role-without-disposition`
- `pass-point-corpus`
- `generalize-point-examples-to-property`
- `attach-debt-to-failing-carrier`
- `waive-missing-passing-carrier`
- `update-carrier-and-reaffirm`
- `reaffirm-with-stale-carrier`

The closed task-3.4 mutation vocabulary is:

- `remove-formalization-approval`
- `remove-external-model-binding`
- `mutate-proof-atom-binding`
- `mutate-proof-property-binding`
- `mutate-proof-implementation-binding`
- `mutate-proof-assumption-binding`
- `mutate-proof-translator-binding`
- `mutate-proof-translator-configuration`
- `mutate-proof-verifier-binding`
- `mutate-proof-verifier-configuration`
- `mutate-proof-external-model-binding`
- `mutate-proof-generated-artifact-binding`
- `mutate-proof-source-binding`
- `mutate-proof-verdict-binding`
- `mutate-proof-scope-binding`
- `mutate-proof-exclusions-binding`
- `accept-verifier-with-stale-atom`
- `render-explicit-source-trust`
- `claim-unverified-backend-correctness`
- `select-pure-lifecycle-function`
- `target-unsupported-effect`
- `run-with-offline-pins`
- `replace-pin-with-live-branch`
- `compare-verus-aeneas-spikes`
- `select-lane-by-preference-only`

Each mutation has exactly one implementation in the independent fixture runner. A mutation that cannot find exactly one expected target is a fixture error.

## Expected facts

Facts use stable strings until typed report values exist. The phase-3 manifest maps each string to an exact report field. Required SPG fact names include:

- `authority-count:<ID>=<n>`
- `atom-revision:<ID>=same|changed`
- `complete-graph=changed`
- `lifecycle:<ID>=<state>`
- `replacement:<ID>=<ID>`
- `carrier-count:<ID>=<n>`
- `carrier:<ID>=fresh|stale|attached-test`
- `implementation-link:<ID>=navigational`
- `reference-closure=closed|open`
- `coverage:<ID>:<role>=satisfied|unsatisfied`
- `source-span:<ID>=exact`
- `surface:<ID>=complete|incomplete`
- `surface-member:<key>=authority|undecided`
- `derivation:<ID>=fresh|stale`
- `adapter-disposition:<path>=semantic|nonnormative|diagnostic`
- `adapter-node:<path>:<node>=semantic|nonnormative|diagnostic`

Required DET fact names additionally include:

- `canonical-graph-bytes=same|different`
- `static-report-bytes=same|different`
- `ambient-inputs-consumed=<n>`
- `normalized-path:<input>=<canonical>`
- `path-selection=none`
- `diagnostic-bytes=same|different`
- `diagnostics=total-order`
- `duplicate-selection=none`
- `canonical-preimage=same|different`
- `raw-concatenation=same|different`
- `full-digest=same|different`
- `digest-algorithm=sha256`
- `identity-reinterpreted=false`
- `identity-width=invalid|full`
- `semantic-content-bytes=same|different`
- `raw-artifact-bytes=same|different`
- `unicode-normalization=nfc`
- `execution-report-bytes=same|different`
- `static-graph-identity=same|different`
- `verdict:<oracle>=not-run|pass|fail|error|timeout`
- `canonical-diagnostic-host-data=absent|present`
- `noncanonical-diagnostic-host-data=absent|present`
- `fcis-no-std=pass|fail`
- `fcis-ambient-state=pass|fail`
- `fcis-callable-interface=pass|fail`
- `assurance=structural-only`
- `impact-report-bytes=same|different`
- `impact-count=<n>`
- `provider-inputs-consumed=<n>`
- `impact-order=canonical`
- `impact-closure=complete|incomplete`
- `disposition-count=<n>`

Required EVD fact names additionally include:

- `assurance:<subject>=linked|tested|exhaustively-checked|formally-proved|structurally-enforced|review-only`
- `assurance:<subject>:tested=false`
- `assurance:<subject>:property=false`
- `assurance:<subject>:exhaustively-checked=false`
- `assurance:<subject>:formally-proved=false`
- `assurance-scope:<subject>=<scope-id>`
- `corpus:<subject>=<corpus-id>`
- `model:<ID>=fixed-point|incomplete`
- `model-reachable-states:<ID>=<n>`
- `semantic-diagnostics:cli=lsp`
- `authoritative-input=saved-snapshot`
- `editor-result=noncanonical`
- `ci-acceptance=false`
- `policy-ratchet:<ID>=strengthened|weakened|equivalent`
- `policy-mode:<ID>=advisory|blocking`
- `policy-scope:<ID>=advisory|blocking`
- `removed-obligation:<role>=visible`
- `debt:<ID>=active|discharged|superseded`
- `waiver:<ID>=active|discharged|revoked|superseded`
- `obligation:<ID>=satisfied|unsatisfied`
- `reaffirmation:<ID>=valid|invalid`

Required FPR fact names additionally include:

- `formal-property:<ID>=eligible|ineligible`
- `correspondence:<ID>=review-only`
- `proof:<ID>=complete|incomplete|freshness-eligible|fresh|stale`
- `proof-control:<ID>=pass|expected-fail|unexpected-pass|unexpected-fail`
- `trusted-tool:<ID>=visible`
- `representation:<ID>=<schema-id>`
- `exclusion:<ID>=visible`
- `backend-correctness=unproved`
- `proof-target:<ID>=eligible|ineligible`
- `effects:<ID>=none|filesystem|process|network|concurrency|unsafe`
- `formal-tools=fully-pinned|not-fully-pinned`
- `network-consumed=false|true`
- `formal-verdict=pass|fail|error|timeout|not-run`
- `proof-record=reproducible|nonreproducible`
- `spike:<lane>=complete|blocked|missing`
- `lane-decision=evidence-eligible|incomplete`
- `production-formal-claims=blocked|eligible`

Unknown expected facts fail the fixture runner.

## Freeze rule

Once `spec-provenance:fixtures` exists, a case changes only with the owning OpenSpec requirement and reviewed reason. Production implementation never regenerates expected outcomes.
