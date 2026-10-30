## Why

Buoy stores exact implementation links, but current static output does not present fresh links as a readable, searchable view. Users must inspect opaque attributes and immutable object files across repository mounts.

OpenSpec change artifacts can record link intent, but the current template lacks stable relation, consumer, target, and completion fields. Standing specifications also lack a safe same-file navigation section that remains outside requirement identity.

This change adds deterministic trace projections without moving canonical bindings into OpenSpec authority text.

## What Changes

### Goals

- Add a read-only `buoy trace` command with explicit requirement, relation, implementation, and record selectors.
- Add deterministic human, machine, and Markdown trace views with paths, source spans, freshness, and abbreviated human digests.
- Add implementation-owned Markdown trace pages that derive from current graph data and retain complete machine identities.
- Add an opt-in OpenSpec format that accepts external trace-link fences only in nonauthority level-two sections after requirements.
- Extend `traceability.md` with planned relation data, link review, and a labeled historical completion snapshot.
- Preserve exact requirement, atom, implementation, and binding identities under the existing schema versions.

### Non-goals

- Do not put implementation binding records or current digests inside requirement spans.
- Do not make `buoy bind` modify an authority repository or a change artifact.
- Do not treat a trace projection, external link, annotation, or template as correctness, coverage, execution, approval, or proof.
- Do not fetch provider data, resolve remote titles, or add network state to static output.
- Do not add editor integration, a hosted trace service, or effectful oracle execution.

## Capabilities

### New Capabilities

- `readable-trace-projections`: Query current and historical implementation links and render deterministic human, machine, and Markdown projections.

### Modified Capabilities

- `content-addressed-implementation-bindings`: Expose read-only traversal from requirement IDs, relation IDs, implementation locations, and record digests.
- `openspec-source-adapter`: Add an opt-in standing-spec format with trace-link sections outside requirement and atom identity.
- `external-trace-links`: Accept reviewed trace-link fences from the new OpenSpec nonauthority section without changing the closed relation set.
- `openspec-buoy-workflow`: Record planned link intent and historical completion pointers in `traceability.md` before task creation.

## Impact

- CLI parsing and report rendering gain a read-only trace path with explicit selector types.
- The OpenSpec adapter gains one versioned format while all version 1 authority-node identities remain byte-compatible across format adoption.
- The Markdown trace renderer produces derived output only. Canonical source graphs and binding records remain authoritative.
- Workflow templates, examples, documentation, fixtures, report goldens, and positive and negative controls change.
- Static trace commands remain deterministic and offline. They do not invoke configured oracles or provider APIs.
