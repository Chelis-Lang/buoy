# Version 1 coverage governance

Status: frozen schema decision for task 2.7. This document defines coverage-policy adoption ratchets, weakening, debt and waiver lifecycle/discharge, and reaffirmation approval without using provider state as authority.

## Policy identity and current authority

A `CoveragePolicy` has a stable ID, semantic revision, active approval, lifecycle, replacements, and optional terminal approval.

Only `active` policies create current obligations. `superseded` and `retired` policies remain in `SpecGraph` to reserve identity and preserve history.

A policy revision digest covers exactly:

- finite scope predicate;
- required evidence-role set;
- `advisory` or `blocking` mode;
- adoption-ratchet ID;
- expected-authority selectors; and
- derivation requirements.

Policy ID, lifecycle, approvals, and replacements are excluded from the policy revision but included in policy-object and complete-graph identity.

The ratchet ID identifies one monotonic adoption line. It is stable across ordinary revisions. Changing it creates a new policy lineage and cannot hide weakening of the old lineage.

## Finite policy scope

A policy scope must evaluate against one explicit graph to a finite canonical set of `ObligationRef`s. V1 scope predicates may select by:

- exact atom ID or atom scope ID;
- exact surface ID and optional canonical member-key set;
- object kind and stable ID set;
- selector ID and expected-authority class; or
- a declared finite union or intersection of those predicates.

V1 has no negation, arbitrary code predicate, regular expression over source, or provider query in blocking policy scope. Exclusion is represented by explicit finite set subtraction whose removed subjects are themselves digest-bound in the policy revision.

An empty scope is invalid for an active policy unless it is a temporary advisory migration policy whose purpose and discharge condition are recorded by an active debt record.

## Obligation construction

For each selected subject, policy configuration creates explicit obligations for the applicable kinds:

- valid and, where required, ratified authority;
- decision-surface completeness;
- disposition authority;
- each required evidence role;
- carrier/derivation/proof freshness;
- oracle selection;
- passing execution;
- derivation consistency;
- proof acceptance; and
- complete change-impact disposition.

Obligation generation is a deterministic set operation. Names, test conventions, and provider labels cannot create or satisfy a role.

## Adoption and enforcement

### Advisory

An advisory policy evaluates and reports every obligation but does not make an unsatisfied obligation determine the authoritative command's blocking exit class. Invalid schema, graph closure, duplicate authority, and canonicalization errors remain blocking even under advisory policy; advisory mode applies to coverage adoption, not data validity.

### Blocking

A blocking policy makes every applicable unsatisfied obligation determine the blocking exit class unless the same policy explicitly permits a valid active waiver for that obligation. The obligation remains unsatisfied and visible even when process continuation is authorized.

### Promotion

The normal adoption path is:

1. add an advisory policy and debt report;
2. eliminate undecided, uncovered, stale, or unselected obligations;
3. review a policy revision changing `advisory` to `blocking`; and
4. retain the same ratchet ID and scope.

Promotion does not erase advisory history. A blocking policy may be introduced directly when all required fixtures and obligations already exist.

## Ratchet comparison

Base-to-head change analysis compares active policies with the same stable ID and ratchet ID. It materializes both finite obligation sets before classifying the revision.

### Strengthening changes

The following are strengthening or neutral when they do not remove another obligation:

- adding scope subjects;
- adding required evidence roles;
- `advisory -> blocking`;
- `optional -> required` expected authority;
- adding a derivation requirement;
- narrowing an accepted waiver class;
- replacing a scope predicate with one that materializes the same obligation set; and
- editorial approval-record movement that preserves all semantic digests.

Strengthening changes still create new impacts and may need debt during staged adoption. They do not require a weakening migration.

### Weakening changes

The following are weakening impacts:

- removing any previously generated obligation or scope subject;
- removing a required role;
- `blocking -> advisory`;
- `required -> optional` or `forbidden -> optional` authority expectation;
- removing or narrowing a decision-surface domain so obligations disappear;
- removing a derivation/consistency requirement;
- changing a surface member from `normative` or `prohibited` to `deferred` or unauthorised `not-applicable`;
- replacing a current atom citation with no authority or a proposed atom where ratification was required;
- expanding a waiver class;
- changing the ratchet ID to avoid base-to-head comparison;
- terminalizing a policy while any obligation would disappear without equivalent replacement policy; or
- deleting a policy object or terminal history record.

A semantically equivalent scope rewrite is not weakening only if independent materialization produces the exact same obligation set.

### Accepted weakening protocol

Weakening never passes silently. It requires all of:

1. an explicit policy-ratchet impact in the neutral change report;
2. one `revised`, `superseded`, or `retired` disposition for the policy;
3. approval by the policy's configured authority;
4. a repository-owned migration record stating rationale, affected obligation set, and replacement/discharge behavior;
5. debt records for every obligation intentionally left unsatisfied or unowned; and
6. a valid active waiver if a blocking gate is allowed to proceed while any removed obligation remains relevant.

The report continues to show that coverage was weakened. Approval authorizes the governance action; it does not establish technical correctness.

## Policy supersession and retirement

An active policy may become:

- `superseded` only with one or more active replacement policies whose combined materialized obligations cover every still-applicable base obligation, or with explicit accepted weakening records for the difference; or
- `retired` only when all base obligations are no longer applicable under an approved domain/lifecycle change and the terminal approval binds that evidence.

Terminal policy IDs remain reserved. Reactivation and ID reuse are invalid.

## Debt lifecycle and discharge

### Active debt

An active `DebtRecord` identifies exact obligations, affected object revisions, owner, rationale, remediation, and a machine-readable discharge condition. It explains missing work but grants no process exception by itself.

A debt record cannot use a live issue, due date, assignee, or provider label as its discharge condition. Such links may be informational. A discharge condition uses repository facts such as:

- named obligation becomes satisfied;
- replacement object/policy is ratified and current;
- specified oracle obtains `pass` under exact bindings;
- terminal lifecycle transition is accepted; or
- named active replacement debt takes ownership.

### Discharged debt

Debt becomes `discharged` only in a head graph that includes:

- terminal approval by the debt/policy authority;
- evidence references proving every condition;
- every affected obligation satisfied, retired, or no longer applicable under approved evolution; and
- no active replacement debt.

The terminal record remains in the graph and in reports. Discharge is review evidence about closure, not retroactive assurance for earlier revisions.

### Superseded debt

Debt becomes `superseded` when one or more active replacement debt records cover every still-unsatisfied obligation. The terminal approval and replacement set are mandatory. Splitting and merging are allowed only when set comparison proves no obligation was dropped.

Debt cannot return to active or be deleted.

## Waiver lifecycle and discharge

### Active waiver

A waiver is valid only when:

- it names exact unsatisfied obligations and object revisions;
- each affected blocking policy explicitly permits waiver for that obligation kind;
- its approval authority matches every affected policy or a configured joint authority;
- rationale and repository-evaluable discharge condition are nonempty;
- no broader wildcard scope is used; and
- it is reported alongside the unchanged unsatisfied facts.

A waiver permits process continuation only. It cannot satisfy, freshen, select, pass, prove, or structurally enforce anything.

### Discharged waiver

A waiver becomes `discharged` only when all waived obligations are satisfied, retired, or no longer applicable and exact discharge evidence plus terminal approval is present. It has no active process effect thereafter.

### Revoked waiver

A waiver may become `revoked` under terminal approval even while obligations remain unsatisfied. Process authorization ends immediately; blocking policy fails until obligations pass or another independently approved waiver applies.

### Superseded waiver

A waiver becomes `superseded` only when active replacement waivers explicitly cover each continued exception. Any dropped exception reverts to normal blocking behavior. Replacement cannot broaden scope without a new weakening impact and approval.

Terminal waiver IDs remain visible and cannot reactivate or be reused.

## Time and expiry

Canonical governance does not read the clock. A calendar date may appear as informational text but cannot automatically activate, expire, or discharge policy, debt, or waiver state.

Repositories that want time-based review materialize an explicit reviewed lifecycle transition. The wall clock can trigger shell-side automation to propose that change, but the committed transition and approval are the canonical facts.

## Reaffirmation protocol

A reaffirmation is required when change policy classifies an affected object as deliberately retaining the same claim role across changed semantic bindings rather than revising or retiring it.

A valid reaffirmation:

- binds one exact impacted object kind and stable ID;
- binds exact, different base and head digests;
- contains a nonempty statement that the new authority/claim was reread;
- uses the approval authority configured for that object namespace and change action;
- is repository-owned and digest-bound;
- matches exactly one `reaffirmed` change disposition; and
- does not use a wildcard, range, directory, or “all carriers” batch scope.

A tool may generate draft reaffirmation records, but the records remain individually reviewable. Mechanical digest replacement without the matching records fails impact closure.

Reaffirmation is `review-only`. The head carrier or object must independently bind current digests and satisfy validation. Reaffirmation cannot make stale data fresh or turn review into testing/proof.

## Approval authority

Policy activation, weakening, terminal transitions, waivers, debt terminal transitions, and reaffirmations use repository-configured approval authorities. Provider reviews and user IDs are not canonical authority.

A provider adapter may materialize a candidate repository review record containing the decision. Canonical validation uses only the committed record path, digest, authority ID, and configured authority rules. Provider state can disappear without changing historical canonical facts.

Where separation of duty is configured, the approval record contains stable repository authority IDs and a pinned rule payload; the core validates the payload without contacting an identity provider. This is process evidence, not cryptographic identity proof unless a later schema binds signatures.

## Reporting and no-laundering invariants

For every obligation, reports retain independent:

- satisfaction;
- active/terminal debt;
- active/terminal waiver;
- change disposition;
- reaffirmation review fact;
- execution verdict; and
- assurance classes.

The following invariants always hold:

- `waived && unsatisfied` remains unsatisfied;
- debt never authorizes continuation;
- reaffirmed does not imply fresh;
- discharged debt/waiver does not imply earlier revisions were compliant;
- passing siblings cannot hide a missing required role;
- a terminal policy cannot erase its history; and
- advisory visibility precedes blocking adoption but never weakens graph validity.

## Rejected alternatives

- **Provider labels as waivers:** mutable, external, and unavailable offline.
- **Issue closure as debt discharge:** issue state is not canonical evidence of obligation satisfaction.
- **Automatic date expiry:** introduces clock state and ambiguous failure during offline checks.
- **Delete resolved records:** permits ID reuse and erases governance history.
- **One bulk reaffirmation for a directory:** allows mechanical rev-bumping without reviewing each affected claim.
- **Treat waiver as satisfied:** launders the exact gap the waiver is meant to expose.
