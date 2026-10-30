# Buoy

Buoy checks links between software requirements, source code, tests, and proof records.
Use it to find a requirement's code links and detect references that need review after a change.

Requirements have stable IDs and content digests.
A saved binding names an exact requirement revision and source item.
Buoy compares those references with current files and reports missing or stale records.
It does not decide whether the code meets the meaning of a requirement.

Static checks read local Markdown, Rust, OpenSpec, and binding files.
They do not contact GitHub or other external services.

## Install from source

Install [Rust with rustup](https://rustup.rs/) and a native C toolchain.
The checkout selects its Rust version through [`rust-toolchain.toml`](rust-toolchain.toml).

Run this command from the repository root:

```sh
cargo install --locked --path . --bin buoy --features blake3-256,sha-256
```

This enables all three digest algorithms, including the BLAKE3 digest that the trace example uses.
If you set `CARGO_HOME`, Cargo installs the command in `$CARGO_HOME/bin`.
Otherwise, it uses `$HOME/.cargo/bin`.
Add that directory to `PATH`.
You do not need Nix or the OpenSpec CLI to use Buoy.

## Example: connect a requirement to code

Copy [the sample repository](fixtures/readable-trace-projections/v1/repository/) to a directory named `demo`.
It contains an OpenSpec requirement, a Rust source file, configuration, and a saved binding.

The requirement in `demo/openspec/specs/demo/spec.md` includes:

```markdown
### Requirement: BGLC-003 Render a deterministic and complete catalog

The generator MUST render every discovered library and registered lint in stable order.
```

The function in `demo/src/lib.rs` is:

```rust
pub fn render_catalog(libraries: &[&str]) -> String {
    libraries.join("\n")
}
```

The saved binding connects `BGLC-003` to `src/lib.rs::render_catalog`.
Inspect that link:

```sh
buoy trace \
  --root demo \
  --config demo/buoy.toml \
  --relation-id BGLC-RENDER-CATALOG \
  --format human
```

The command returns exit code `0`.
Its output includes:

```text
relation: BGLC-RENDER-CATALOG (implements)
state: current
requirement: BGLC-003 openspec/specs/demo/spec.md:93-608
  Render a deterministic and complete catalog
implementation: src/lib.rs:0-98 rust/item/v1
  render_catalog
```

This result means the saved references match the current source items.
It does not test the function or prove that its output meets the requirement.

### Detect a stale requirement

In `demo/openspec/specs/demo/spec.md`, replace `in stable order.` with `in name order.`.
Keep the binding unchanged, then run:

```sh
buoy static --root demo --config demo/buoy.toml --format human
```

The command returns exit code `2` and reports:

```text
CAB-REQUIREMENT-STALE src/lib.rs:0-98 BGLC-RENDER-CATALOG
```

The requirement keeps its ID, but its revision changes.
The binding still names the old revision.
Review the change before you create a new binding.
See the [binding guide](docs/schema/v3/content-addressed-bindings.md) for that process.

## Example: check a proposed change

The [change example](examples/repositories/change-check/) contains separate base and head directories.
The base requirement says `preserve saved base inputs`.
The head changes it to `preserve saved base and head inputs`.

A change manifest records how each changed requirement is handled.
This incomplete manifest omits that decision:

```sh
buoy change \
  --base examples/repositories/change-check/base \
  --head examples/repositories/change-check/head \
  --config examples/repositories/change-check/buoy.toml \
  --manifest examples/repositories/change-check/incomplete.toml \
  --format human
```

The command returns exit code `3` with `CHG-DISPOSITION-MISSING`.
The complete manifest records the requirement as `revised`:

```sh
buoy change \
  --base examples/repositories/change-check/base \
  --head examples/repositories/change-check/head \
  --config examples/repositories/change-check/buoy.toml \
  --manifest examples/repositories/change-check/complete.toml \
  --format human
```

This command returns exit code `0` with no diagnostics.
Buoy checks the recorded decision against the selected revisions. It does not grant review approval.

## Configure your own project

Save this configuration as `buoy.toml`:

```toml
schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "required"
```

This selects Markdown files below `spec/`.
Use [TOML blocks](examples/repositories/change-check/base/spec/requirements.md) to declare Markdown requirements.
From your project root, run:

```sh
buoy static --root . --config buoy.toml --format human
```

The [adapter guide](docs/engineering/adapter-sdk.md) describes other source formats.
Selected roots cannot contain symlinks or submodules.

## Commands

| Command | Purpose |
| --- | --- |
| `static` | Check requirements, links, and saved evidence without execution. |
| `bind` | Create a binding record and attach it to a selected source item. |
| `trace` | Inspect current or historical requirement and implementation links. |
| `change` | Check a change against its base version and change manifest. |
| `execute` | Run configured checks and report their results. |

Use `--format machine` for JSON or `--format human` for text.
The `trace` command also supports `--format markdown`.
Trace reports go to standard output and do not change source files.

### Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success. |
| `2` | A static check found a blocking problem. |
| `3` | The change check is incomplete or invalid. |
| `4` | An executable check failed. |
| `64` | Command usage error. |
| `70` | Input or configuration error. |

## Limits

Repository checks need a Unix system. Buoy does not support Windows snapshots.
A current link does not count as test coverage or proof.
Test and proof records apply only to their declared inputs and scope.
XXH3-128, the default digest, is not a cryptographic hash.

See the [trust limits](docs/assurance/trust-boundaries.md) for details.

## Documentation

- [More examples and Rust APIs](examples/README.md)
- [Binding records and updates](docs/schema/v3/content-addressed-bindings.md)
- [Trace reports](docs/schema/v3/readable-trace-projections.md)
- [OpenSpec support](docs/schema/v3/openspec-traceability.md)
- [Multiple source roots and file bindings](docs/schema/v3/logical-mount-artifacts.md)

## Development

The [`buoy-core`](crates/buoy-core/) crate contains the deterministic `no_std` core.
The root `buoy` crate handles files, parsers, commands, and reports.

Contributor checks use Nix and devenv.
See the [development guide](docs/engineering/enforcement-order.md) for commands.
See [source installation and maintenance](docs/release/maintenance.md) for package details and routine checks.
Known Miri and coverage check failures appear in the [release checklist](docs/release/buoy-rename-checklist.md#source-only-publication-scope).

## Security and license

Report security defects through the private process in [SECURITY.md](SECURITY.md).
Do not submit security details through public issues or pull requests.

Repository-owned code uses the [MIT license](LICENSE), copyright 2026 2nd Set AI Corp.
Third-party code and data retain their own license notices.
