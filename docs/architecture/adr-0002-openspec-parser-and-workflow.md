# ADR-0002: OpenSpec Parser and Workflow

## Status

Accepted.

## Context

Buoy needs exact source ranges for OpenSpec requirements and scenarios. Buoy also needs a project-local workflow that requires traceability planning.

The integration must not depend on provider APIs. Provider state changes without a change to the repository.

## Decision

Buoy uses the pinned `pulldown-cmark` parser for OpenSpec Markdown. The parser emits original byte ranges for headings, paragraphs, lists, and code blocks.

Buoy recognizes only the standing-spec forms that the adapter contract declares. Other Markdown remains nonnormative or produces a stable diagnostic.

Buoy uses a project-local OpenSpec schema named `buoy-bridge`. The schema requires `traceability.md` before `tasks.md`.

The workflow provides planning guidance only. The Buoy checker does not read workflow templates or generated planning artifacts.

## Proof Results

The positive fixture produced one requirement atom and three scenario items. Each item used the exact range from the source file.

The malformed fixtures produced stable diagnostics for an incomplete heading and an invalid scenario level. Repeated runs produced equal outputs.

`openspec schema validate buoy-bridge` accepted the local schema. `openspec schema which buoy-bridge` selected the project-local schema.

A temporary change used the local schema and produced the required artifact order. The traceability artifact preceded the task artifact.

## Upstream Result

No upstream OpenSpec change is necessary. The supported local-schema mechanism provides the required artifact and dependency rules.

## Consequences

The parser dependency remains pinned in `Cargo.lock`. A dependency update requires fixture and span review.

The adapter rejects unsupported standing-spec syntax. This behavior prevents accidental authority from ordinary Markdown.

Provider connectors can consume the command plan. They must not add provider status to the semantic graph.
