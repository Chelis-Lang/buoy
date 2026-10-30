# Readable trace projection fixtures

Status: test-first fixtures for the active `add-readable-buoy-trace-projections` change.

## Repository

`repository/` holds one complete snapshot with one fresh implementation relation. Its bytes are a copy of the reviewed content-addressed binding fixture.

The snapshot binds requirement `BGLC-003` to the Rust item `render_catalog` through relation `BGLC-RENDER-CATALOG`.

Tests copy this snapshot into a temporary directory. A test then edits the copy to create a stale or historical case. No test writes into this directory.

## Case tables

Each table lists one case for each row. The tables are the reviewed input for the focused tests.

- `selectors/positive.tsv` names each accepted typed selector.
- `selectors/negative.tsv` names each rejected selector and its exact exit class.
- `relations/cases.tsv` names each current, historical, and stale relation state.
- `collisions/cases.tsv` names each abbreviated-digest display case.

## Output contracts

`contracts/` states the required and forbidden content of each rendered view.

- `machine.tsv` lists required machine fields, forbidden fields, and canonical row order.
- `human.tsv` lists required human lines.
- `markdown.tsv` lists required Markdown sections.

Exact output goldens arrive with the renderer implementation. The contracts state the reviewed shape before that implementation exists.

## Cross-adapter fences

`cross-adapter/` holds equal `buoy-external-trace-v1` fence bytes for the Markdown adapter and the OpenSpec version 2 adapter. Both adapters must produce equal link values from these bytes.

## Workflow artifacts

`workflow/` holds accepted and rejected `traceability.md` sections for planned relation rows and historical completion snapshots.

## Assurance boundary

These fixtures supply navigation and impact data only. They do not establish coverage, execution, proof, approval, correctness, or prose-to-code correspondence.
