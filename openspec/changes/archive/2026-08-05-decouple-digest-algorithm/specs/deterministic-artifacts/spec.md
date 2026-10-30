## MODIFIED Requirements

### Requirement: DET-004 Versioned unambiguous encoding
Every canonical digest MUST identify its algorithm through the active semantic schema, and every digest preimage MUST include a domain tag, schema version, algorithm identity, and unambiguous field encoding. The active version 3 schema MUST take the algorithm name, numeric tag, output length, and complete parameters of every repository-owned identity from the closed digest-algorithm registry, and MUST use the one declared active member for computation. The canonical text form MUST be `<canonical-name>:<lowercase hexadecimal payload>`, and the binary form MUST carry the member's declared tag and declared byte count in that member's official canonical order. Truncated forms MUST be display-only. A digest under an obsolete tag or an unregistered algorithm MUST NOT be accepted by the active schema, and an obsolete digest MUST be regenerated from the authoritative canonical source under the version 3 schema and domain rather than reinterpreted in place.

#### Scenario: Positive - semantic field changes
- **GIVEN** a canonical version 3 object and one mutation to a digest-bound semantic field
- **WHEN** both domain-separated preimages are encoded and hashed with the declared active member
- **THEN** the encoded preimages and full version 3 digests differ for the exercised mutation

#### Scenario: Negative - ambiguous concatenation boundary
- **GIVEN** two different sequences of field names or values that would collide under raw concatenation
- **WHEN** the canonical encoder processes them
- **THEN** length delimiting or an equivalent encoding produces distinct preimages before hashing

#### Scenario: Negative - obsolete algorithm changes in place
- **GIVEN** an object identity established under an obsolete tag, an unregistered algorithm, or a registered member that is not the declared active member
- **WHEN** a configuration, parser, or migration attempts to relabel or reinterpret its digest bytes under the active member
- **THEN** validation fails with a deterministic digest-schema diagnostic and requires regeneration from authoritative canonical source bytes
