# Logical mounts and opaque artifact bindings

Status: active schema v3 protocol.

## Logical snapshot model

A version 3 configuration declares one or more logical mounts. Each mount has a stable ID and a canonical logical prefix.

The command supplies one physical directory for each mount ID. Physical paths are operational inputs and never enter canonical output.

Buoy opens all mount descriptors in canonical mount-ID order. It then projects selected files into one closed logical namespace.

The projected path has this form:

```text
<mount-prefix>/<selector-root>/<relative-path>
```

Buoy aggregates all selected products before reference resolution. No mount receives a separate acceptance result.

## Version 3 configuration

Use this configuration shape:

```toml
schema = "buoy/repository-config/v3"
digest_algorithms = ["blake3-256"]

[[mounts]]
id = "authority"
prefix = "authority"

[[mounts]]
id = "project"
prefix = "project"

[[selectors]]
mount = "authority"
root = "openspec/specs"
adapter = "openspec/v1"
include = ["**/spec.md"]
exclude = []
authority_expectation = "required"

[[selectors]]
mount = "project"
root = "flake.nix"
adapter = "artifact/v1"
include = ["flake.nix"]
exclude = []
authority_expectation = "forbidden"
```

Mount IDs and prefixes must be unique. Prefixes must not overlap under canonical or portable path comparison.

Each selector must name one declared mount. An `artifact/v1` selector must use `authority_expectation = "forbidden"`.

Custom `artifact/v1` configuration is invalid. The adapter does not accept a language, parser, media override, or text normalization rule.

## Physical mount arguments

For static analysis, supply one `--mount ID=PATH` value for each declaration:

```text
buoy static \
  --mount project=/work/barnacle \
  --mount authority=/work/openspec \
  --config buoy.toml \
  --format machine
```

For oracle execution, use the same complete mount set:

```text
buoy execute \
  --mount project=/work/barnacle \
  --mount authority=/work/openspec \
  --config buoy.toml \
  --format machine
```

Version 3 rejects `--root`. Version 2 rejects `--mount` and keeps the existing `--root` behavior.

`buoy change` retains its version 2 `--base` and `--head` contract. This release does not compare version 3 mount sets.

Buoy rejects missing, extra, duplicate, or malformed mount mappings before it selects a file.

## Current requirement subjects

A static machine report contains `requirement_subjects` when current OpenSpec requirements exist. Each row contains these exact current facts:

- Stable requirement ID.
- Requirement source-node digest.
- Atom revision.
- Logical standing-spec path.
- Ordered authority scopes.

Rows use canonical order and complete digests. They exclude physical roots and planning documents outside configured standing authority selectors.

Use one unambiguous row's `subject` value as `buoy bind --requirement`. The inventory supplies navigation data and no assurance class.

## Aggregate resource limits

One budget account covers the complete mount set. A new mount does not multiply any limit.

The account includes these limits:

- Path depth.
- Directory entries.
- Selected files.
- File bytes.
- Total selected bytes.
- Graph objects and references.
- Diagnostics.
- Source nodes, edges, depth, and visited nodes.

One mount failure rejects the complete snapshot. Buoy retains no partial accepted graph.

## Oracle mount selection

Each version 3 oracle uses one declared mount ID in `working_root`:

```toml
[[oracles]]
id = "ORC-BUILD"
program = "./tools/check"
working_root = "project"
timeout_ms = 30000
```

The physical mount path selects the process directory only. It does not enter graph identity or machine output.

Oracle output does not alter source identities, graph identities, or binding freshness.

## Opaque artifact adapter

The built-in adapter ID is `artifact/v1`. Its descriptor declares these identities:

- Capability: `artifact.opaque-bytes/v1`.
- Source-item kind: `artifact/file/v1`.
- Media type: `application/octet-stream`.
- Representation: `saved-bytes/v1`.

The adapter creates one childless source node for each selected regular file. The source span covers every byte from offset zero.

The identity binds the full logical path, fixed metadata, exact saved bytes, and active digest member.

The adapter does not decode UTF-8. Empty files and invalid UTF-8 are valid inputs.

The adapter emits no atom, authority, semantic object, oracle result, coverage fact, or assurance claim.

## Artifact attachment sidecar

A sidecar selects one immutable binding record for one stable relation ID.

The mount-relative path has this form:

```text
buoy/attachments/v1/<algorithm>/<relation-key-digest>.toml
```

The projected canonical path includes the mount prefix. For example, the `project` mount produces `project/buoy/attachments/v1/...`.

The sidecar uses this schema:

```toml
schema = "buoy/binding-attachment/v1"
relation = "BARNACLE-FLAKE-BINDING"
artifact = "project/flake.nix"
subject = "blake3-256:<artifact-source-node>"
record = "blake3-256:<implementation-binding-record>"
```

The relation-key digest uses domain `buoy/binding-attachment-key/v1`. Its preimage contains the stable relation ID.

The sidecar path does not change when the selected record changes. Old immutable records remain in `buoy/objects/v1/`.

Static validation requires exact agreement between these values:

- The sidecar path key.
- The sidecar relation and subject.
- The current artifact path and source node.
- The immutable record locator, relation, and subject.
- The current requirement, atom revision, and authority.

Unequal sidecars for one relation produce `CAB-ATTACHMENT-DUPLICATE`. Equal sidecars can collapse to one current attachment.

## Artifact bind command

Select one current opaque artifact by its complete logical path:

```text
buoy bind \
  --mount project=/work/barnacle \
  --mount authority=/work/openspec \
  --config buoy.toml \
  --requirement blake3-256:<requirement-node> \
  --artifact project/flake.nix \
  --relation BARNACLE-FLAKE-BINDING \
  --format machine
```

`--artifact` and `--implementation` are exclusive. The requirement, relation, and navigation inputs remain explicit.

The selected artifact mount owns the object and sidecar. Physical writes use `<mount-root>/buoy/...`.

The command validates every selector before its first write. It leaves the selected artifact bytes unchanged.

The transaction performs these actions:

1. Stage and synchronize the immutable object.
2. Replace the immutable object destination.
3. Stage and synchronize the sidecar.
4. Replace the sidecar atomically.

If a precommit operation fails, the command removes temporary files and the uncommitted object. The prior sidecar remains unchanged.

A retry can reuse an equal immutable object. A different object at the selected path causes an input failure.

The finite transaction model injects one fault and then permits cleanup retries. Persistent filesystem refusal remains outside the finite model.

## Barnacle pilot layout

The repository fixture models this layout:

```text
project mount
├── README.md
├── flake.nix
├── src/lib.rs
└── buoy/
    ├── objects/v1/
    └── attachments/v1/

authority mount
└── openspec/specs/demo/spec.md
```

The pilot joins both roots into one static snapshot. This repository does not modify Barnacle.

## Assurance limits

A fresh artifact binding supplies navigation and impact information only.

It does not establish these facts:

- Coverage.
- Execution.
- Approval.
- Proof.
- Authenticity.
- Authorization.
- Language meaning.
- Build use.
- Runtime behavior.
- Implementation correctness.

Buoy trusts the caller to map each mount ID to the intended physical directory. Buoy verifies selected bytes, not repository ownership.
