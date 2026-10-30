# Semantic Schema v3

Schema v3 is the active semantic profile. It uses versioned text for each parsed source-item kind.

The profile does not read numeric item-kind tags. It does not migrate or preserve a schema-v1 or schema-v2 parsed-item identity.

## Schema v2 fixture disposition

The schema v2 fixture rows remain as historical cutover controls. Schema v3 acceptance does not claim execution evidence for these rows.

The assurance ledger records each schema v2 fixture operation and mutation as `not-applicable`. This disposition supplies no schema v3 evidence.

Each adapter descriptor declares its complete source-item kind set. The registry rejects an undeclared kind before graph materialization.

Schema v3 adds `ExternalTraceLink` as a navigation object. This object binds exact atom revisions but supplies no assurance class.

Schema v3 also adds repository source graphs and compact implementation bindings. These values supply navigation and impact information only.

Logical mounts join separate physical roots into one canonical snapshot. The opaque artifact adapter supplies language-neutral implementation subjects.

See [`logical-mount-artifacts.md`](logical-mount-artifacts.md) for version 3 configuration, sidecars, commands, and the Barnacle pilot layout.

See [`content-addressed-bindings.md`](content-addressed-bindings.md) for the Rust syntax, immutable store, traversal, and stale diagnostics.

Schema v3 adds one derived shell projection for stored relations. The `buoy/trace-report/v1` JSON report is rendered output, not a canonical digest domain, so the machine inventory lists no canonical row for it.

See [`readable-trace-projections.md`](readable-trace-projections.md) for the report fields and the derived-view boundary.

The OpenSpec adapter converts selected standing requirements into atoms. Repository configuration supplies all capability and lifecycle authority.

The adapter accepts `openspec/standing-spec/v1` and `openspec/standing-spec/v2`. Both formats keep the version 1 source-item schema for authority nodes.

The complete machine inventory is [`schema/v3/inventory.tsv`](../../../schema/v3/inventory.tsv). Decoder budgets are in [`schema/v3/resource-budgets.toml`](../../../schema/v3/resource-budgets.toml).

See [`openspec-traceability.md`](openspec-traceability.md) for configuration and operating procedures.
