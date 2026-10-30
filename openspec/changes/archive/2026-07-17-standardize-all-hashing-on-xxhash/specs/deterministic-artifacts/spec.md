## MODIFIED Requirements

### Requirement: DET-004 Versioned unambiguous encoding
Every canonical digest MUST identify its algorithm through the active semantic schema, and every digest preimage MUST include a domain tag, schema version, algorithm identity, and unambiguous field encoding. The active version 2 schema MUST use full XXH3-128 digests with seed `0` and the standard XXH3 default secret for repository-owned identity, binary algorithm tag `2`, exactly 16 digest bytes in official XXH128 canonical order (high 64-bit half big-endian, then low 64-bit half big-endian), and canonical text `xxh3-128:<32 lowercase hexadecimal digits>`. Truncated forms MUST be display-only. SHA-family and BLAKE3 values MUST NOT be accepted by the active schema, and an obsolete digest MUST be regenerated from the authoritative canonical source under the version 2 schema/domain rather than reinterpreted in place.

#### Scenario: Positive - semantic field changes
- **GIVEN** a canonical version 2 object and one mutation to a digest-bound semantic field
- **WHEN** both domain-separated preimages are encoded and hashed with the required XXH3-128 profile
- **THEN** the encoded preimages and full version 2 digests differ for the exercised mutation

#### Scenario: Negative - ambiguous concatenation boundary
- **GIVEN** two different sequences of field names or values that would collide under raw concatenation
- **WHEN** the canonical encoder processes them
- **THEN** length delimiting or an equivalent encoding produces distinct preimages before hashing

#### Scenario: Negative - obsolete algorithm changes in place
- **GIVEN** an object identity established under an obsolete SHA or BLAKE3 schema
- **WHEN** a configuration, parser, or migration attempts to relabel or reinterpret its digest bytes as XXH3-128
- **THEN** validation fails with a deterministic digest-schema diagnostic and requires regeneration from authoritative canonical source bytes
