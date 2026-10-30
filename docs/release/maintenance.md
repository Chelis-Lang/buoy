# Buoy maintenance

This guide covers source installation and routine maintenance.
It adds no release approval and removes none.
The [release checklist](buoy-rename-checklist.md) keeps all release approvals.

## Source installation

The source installation uses a source checkout or a prepared source export.
Install Rust with rustup and a native C toolchain.
The installation needs no Nix, Devenv, Atoll access, or private policy store.
The first build downloads dependencies from public registries.

Run this command in the source root:

```text
cargo install --locked --path . --bin buoy --features blake3-256,sha-256 --root PREFIX
```

- `--root PREFIX` is optional. It puts the command at `PREFIX/bin/buoy`. Without it, Cargo uses its default installation root.
- `--bin buoy` installs only the product command. Without it, Cargo installs every binary target of the root package. These targets include repository-only tools.
- `--locked` uses the committed `Cargo.lock`.
- In the source root, rustup uses the toolchain that `rust-toolchain.toml` pins.
- By default, Cargo builds in the `target/` directory of the source root. Add `--target-dir DIR` to use a different build directory.
- Cargo installs the executable and its own installation records. It does not install the license notices. For redistribution, retain the license notices that the release checklist lists.
- Add `PREFIX/bin` to `PATH` when you use a custom installation root.

Check the installed command with the README example:

```text
buoy static \
  --root examples/repositories/openspec-authority \
  --config examples/repositories/openspec-authority/buoy.toml \
  --format machine
```

The command returns exit code `0`.
Its output is byte-equal to `examples/expected/openspec-static.json`.

## Nix consumer package

The Nix consumer package installs only buoy, buoy-migrate, and chelis-provenance.
Repository checkers, generators, and policy tools remain available through Cargo and Devenv.
All three digest variants retain the same commands and third-party notices.

The build maps compiler source paths to `/rustc` and rejects compiler store references in its output.
Runtime libraries and compiler identity metadata stay intact.
The package smoke scans executable bytes for host paths and credential markers.

Offline package smoke uses temporary copies of the OpenSpec demo.
Each copy binds its external trace references to the atom revision from that variant's authority report.
The committed examples remain unchanged.
Migration smoke selects the installed variant as its target algorithm.

## Routine checks

The full contributor Devenv input needs read access to the private Atoll repository.
Nix needs a GitHub access token for that input, as `devenv.yaml` states.
Version checks depend on the existing task that creates the pinned Rustup aliases.

For routine compilation, run the existing task:

```text
devenv tasks run buoy:check
```

The workspace check needs a C compiler and a C++17 compiler.
It includes the native Chelis grammar build.

Without Atoll access, run the Cargo check directly:

```text
cargo check --locked --workspace --all-targets --all-features
```

After a behavior change, run the existing focused tests for that behavior.
This test checks the public static and change examples against their committed outputs and exit codes:

```text
cargo test --locked --all-features --test usage_examples command_examples_match_exact_outputs_and_exit_classes
```

The two Cargo commands need no Atoll access.
Routine checks do not replace release acceptance.
`devenv test` stays the only full acceptance command.

## Pins and evidence

Keep the dependency and toolchain pins.
The Cargo manifests, `Cargo.lock`, `rust-toolchain.toml`, `devenv.yaml`, and `devenv.lock` hold them.
Change a pin only in a reviewed patch.
A pin or source change can make recorded evidence stale.
Run the real check or campaign before a new record replaces stale evidence.
The [golden and mutation policy](../testing/golden-and-mutation-policy.md) gives the renewal rules.

## Patch workflow

Maintenance is manual.
This workflow starts no task on a schedule. It does not publish releases.

1. Collect related dependency, toolchain, and documentation changes in one patch group.
2. Run the routine checks and the focused behavior tests for the group.
3. Renew each affected evidence record with a real run.
4. After the last source change, refresh the reviewed source digest one time with `nix/tools/derive-source-content-digest.sh --update`. This step needs Nix. Review the new value, then run the script with `--verify`.
5. Run `devenv test` for release acceptance.
6. Follow the approvals in the release checklist. This guide authorizes no commit, push, merge, visibility change, history change, or release.

## Support and security

Use [SECURITY.md](../../SECURITY.md) for private vulnerability reports.
Maintainers must enable and verify the report form during an approved public-visibility change.
Do not place vulnerability details in public issues or pull requests.
