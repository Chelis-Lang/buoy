# Identifier construction boundaries

Status: active v2 engineering contract.

## Typed identifiers

`StableId` represents the uppercase stable object ID grammar.
`ScopeId` represents lowercase scope, authority, owner, namespace, tool, and ratchet IDs.
`SchemaId` represents schema IDs.
`NamespacePrefix` represents uppercase namespace prefixes.
Each type stores a private `String`.

Each `new` constructor applies NFC normalization and its frozen grammar.
Invalid input returns the role-specific construction code and no typed value.
Each `as_str` method returns canonical text.
Canonical encoders use this text without a second parse.

## Parse boundary

Shell code, tools, adapters, and fixture builders parse raw identifiers at input boundaries.
Core model constructors accept typed identifiers for every identifier role.
Core model code does not run an identifier grammar again.
Core model code does not accept raw strings for identifier roles.

The namespace-prefix comparison accepts `&NamespacePrefix`.
Stable-ID lookups accept `&StableId`.
The compatibility facade re-exports all four identifier types.
The functional core remains `no_std`.

## Repository gate

Run `devenv tasks run buoy:identifier-policy` to check the typed boundary.
The gate reports these conditions:

- an untyped identifier parameter
- a string bypass constructor
- an identifier grammar helper outside `crates/buoy-core/src/identifier.rs`
- a stale exemption

The gate sorts and combines all findings.
Any finding causes a failure.
