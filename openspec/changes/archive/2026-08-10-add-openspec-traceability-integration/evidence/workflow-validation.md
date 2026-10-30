# Workflow Validation

## Local Schema

`openspec schema validate buoy-bridge` accepted the project schema. `openspec schema which buoy-bridge` selected the project source.

## Resolution Conflict

The control set `XDG_DATA_HOME` to a temporary data directory. That directory contained a conflicting user schema named `buoy-bridge`.

`openspec schema which buoy-bridge --json` selected the project schema. Its output listed the temporary user schema in `shadows`.

This result proves the selected OpenSpec 1.6.0 resolution order for the reviewed command. The project schema precedes a user schema with the same name.

## Artifact Order

The active change metadata selects `buoy-bridge`. The active change contains `traceability.md` before its task artifact.

`openspec status --change add-openspec-traceability-integration --json` reports five complete artifacts in the configured order. `openspec instructions apply --change add-openspec-traceability-integration --json` includes `traceability.md` in its context.

The `tasks` artifact depends on `traceability`. OpenSpec cannot make task instructions before the traceability artifact is complete.

## Enforcement Boundary

The Buoy checker does not read the workflow schema, template files, or `traceability.md`. These artifacts provide planning guidance only.
