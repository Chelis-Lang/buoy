# Source-Item Formal Spikes

## Scope

The Verus spike proves two structural claims. A version of zero is invalid, and the parsed-item field tuple binds the source-item kind.

The AeneasVerif spike supplies the equivalent safe Rust subset. It contains the same version predicate and parsed-item field structure.

## Files

- `proofs/spikes/verus/source_item_kind.rs`
- `proofs/spikes/aeneas/rust/source_item_kind.rs`

## Assumptions

The spikes model a parsed path as a positive component count. They model a source-item kind as one distinct field value.

The production parser supplies the lower-case byte grammar. The production canonical encoder supplies field tags, lengths, and byte encoding.

## Exclusions

The spikes do not prove UTF-8 parsing, NFC normalization, parser correctness, hash correctness, or collision resistance.

The spikes do not prove correspondence between an adapter claim and host syntax. Parser-backed fixtures and review supply that evidence.

## Translator Limits

The selected Aeneas subset excludes `String`, iterators, trait objects, and the adapter registry. These features exceed the selected pure comparison scope.

The Verus subset uses mathematical tuples instead of the production encoder. Existing canonical vectors cover the production byte format.

## Accepted Obligation

These files are architecture spikes and translation evidence. They do not create a `formally-proved` assurance fact for production code.

Production acceptance uses exhaustive kind controls, Hegel-rs properties, compile-fail boundaries, canonical vectors, and strict schema checks.
