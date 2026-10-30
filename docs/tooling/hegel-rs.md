# Hegel-rs upstream decision

Status: accepted tooling decision for task 2.4, recorded 2026-07-16.

## Selected upstream

The project name “Hegel-rs” means exactly:

- repository: `https://github.com/hegeldev/hegel-rust`;
- package: `hegeltest` version `0.28.0` from crates.io;
- Rust library crate name: `hegel`;
- immutable source commit from the published crate's `.cargo_vcs_info.json`: `634ea934be19dbb86a5792b4c3d35d6d4a54f7ad`;
- Git tree at that commit: `dc6a62465fb3f6b8d34942c60637d2d1aaca4347`; and
- crates.io archive SHA-256/checksum: `f12603ff7e94c463c26e97565d725010e92139f68fc93edcacf7cb800cfd422b`.

The exact Cargo pin for the active property-test suites is:

```toml
[dev-dependencies]
hegeltest = "=0.28.0"
```

`hegeltest` is test-shell tooling and MUST NOT enter `buoy-core` or its `no_std` dependency closure.

The published package requires Rust 1.86 and edition 2024. The repository's pinned nightly toolchain satisfies that requirement. The production crates may continue to declare Rust 1.85 as their minimum because Hegel is a development dependency, but running the property suite requires Rust 1.86 or newer and the pinned project environment remains authoritative.

## Exact companion packages

`hegeltest 0.28.0` pins two packages from the same upstream repository:

| package | version | source commit | path in source | crates.io SHA-256 |
|---|---:|---|---|---|
| `hegeltest-macros` | `0.28.0` | `634ea934be19dbb86a5792b4c3d35d6d4a54f7ad` | `hegel-macros` | `c3fe342530584e9e0ac8144d40a746fff47c63ad1a2725c8793fbc883f7a6f9e` |
| `hegeltest-c` | `0.29.0` | `b8e2fdcf7046b55fdc84d9b7be68250324654f74` | `hegel-c` | `dc2ab96fd030ce7ad37081c1f6ab3186ba794b9894cefeef2644d9dc1ecc1df7` |

Commit `b8e2fdcf7046b55fdc84d9b7be68250324654f74` has Git tree `3736ecbe3cd96918c3d1aa19c815eb8ba6bcc4d2` and is the commit referenced by tag `v0.29.0`. The root `hegeltest 0.28.0` commit is twelve commits later and is identified by the published crate metadata rather than that tag.

All three package archive checksums were independently reproduced from the downloaded `.crate` files. A generated Cargo lock probe reproduced the `hegeltest` checksum from the crates.io index. These are field-specific Cargo supply-chain checksums retained opaquely under Cargo's trust model; they are not Buoy semantic digests and cannot be parsed, relabeled, or promoted as v2 identity.

The two selected source commits are unsigned according to the GitHub commit-verification API. Therefore this decision claims archive and commit identity, not author-signature provenance. Cargo's registry checksum and the pinned Nix/Cargo lock closure are the acceptance mechanism.

## License

The repository and all selected packages declare the MIT License. The selected source and published package contain:

```text
MIT License
Copyright (c) 2026 Antithesis, LLC
```

The full license grants use, copy, modification, distribution, sublicensing, and sale subject to retaining the copyright and permission notice. This is compatible with development-tool use in this repository. The license text remains part of the pinned package archive; this decision does not replace it.

## Approved API surface

Task 4.1 may use only the documented public API needed for property generation and deterministic execution:

- `hegel::TestCase` and `TestCase::draw`;
- `hegel::Generator`;
- `hegel::generators`, especially integer, boolean, binary, text, vector, tuple, set, map, option, sampled, and composition generators;
- `hegel::Hegel` as the explicit runner builder;
- `hegel::Settings`;
- `hegel::Backend::Default`;
- `Settings::test_cases`, `seed`, `derandomize`, `database`, and `phases`;
- `TestCase::assume`, `reject`, and `note` where a property requires them;
- `#[hegel::test]`, `#[hegel::composite]`, `#[derive(hegel::DefaultGenerator)]`, and failure-reproduction macros only when their expansion is covered by a focused compile fixture; and
- `hegel::stateful` only for a declared state-machine property, never as a substitute for the independent exhaustive models required by task 4.2.

A standalone compile probe against the exact registry package successfully type-checked this load-bearing API:

```rust
use hegel::generators as gs;
use hegel::{Backend, Hegel, Settings, TestCase};

Hegel::new(|tc: TestCase| {
    let values = tc.draw(gs::vecs(gs::integers::<u8>()).max_size(8));
    assert_eq!(values, values.clone());
})
.settings(
    Settings::new()
        .backend(Backend::Default)
        .test_cases(8)
        .seed(Some(0x5350_5631))
        .derandomize(true)
        .database(None),
)
.run();
```

The probe compiled `hegeltest 0.28.0`, `hegeltest-macros 0.28.0`, `hegeltest-c 0.29.0`, and the probe crate successfully under the pinned project shell.

## Deterministic property-run policy

Hegel's defaults inspect CI environment variables, choose a backend automatically, may enable a local failure database, and may select randomness behavior from ambient context. Those defaults are unsuitable as canonical semantics.

Every authoritative project property runner MUST use `tests/support/hegel_settings.rs` and explicitly set:

- `Backend::Default`, never automatic or `Urandom`;
- exactly 256 cases;
- one committed property-specific seed under `0x5350_5631_0000_0000`;
- `derandomize(true)`;
- `database(None)`;
- the exact ordered phases `Explicit`, `Reuse`, `Generate`, `Target`, `Shrink`; and
- no environment-derived setting.

The runner configuration, package checksums, seed/derandomization rule, test-case count, enabled phases, generator-domain declaration, and property source digest are evidence bindings. Hegel output, shrinking traces, timing, and failure databases are noncanonical raw logs. Only the normalized oracle verdict enters a deterministic execution report.

A passing Hegel run establishes sampled property evidence within its declared generator domain and configuration. It is not exhaustive checking or formal proof. A minimized counterexample is evidence of failure, not a canonical semantic identity unless committed as a separately digest-bound fixture.

## Upstream maturity and upgrade rule

The upstream README labels Hegel beta and permits breaking changes. Version 0.28.0 itself contains public-API corrections discovered by a documentation-versus-behavior audit. Exact version and checksum pins are therefore mandatory.

An upgrade requires:

1. a new upstream decision recording package versions, source commits, checksums, license, Rust requirement, and API delta;
2. rerunning the API compile probe and all Hegel positive/negative fixtures;
3. reviewing generator, shrinking, replay, database, seed, and backend behavior;
4. updating every property tool binding and invalidating prior property execution evidence; and
5. no silent substitution with `proptest`, `quickcheck`, the unrelated crates.io package `hegel`, or another property framework.

## Rejected identities

- crates.io `hegel = 0.1.1`: an AWS API Gateway payload crate, not the property framework;
- `hegeltest-c` alone: the native engine/FFI package, not the Rust test API;
- a live `main` branch or moving release tag: not immutable;
- any similarly named Hypothesis, QuickCheck, or Proptest wrapper: not the user-selected upstream; and
- an unchecksummed Git dependency: weaker than the selected registry archive plus source-commit binding.
