## MODIFIED Requirements

### Requirement: RSC-005 Controlled dependency and tool updates
Updating any semantic dependency or formal tool MUST regenerate its verified lock pins, affected fixtures/goldens, parser/Unicode/path conformance evidence, proof bindings, campaign artifacts, and change impacts before acceptance. `devenv.lock` and `Cargo.lock` hold the verified pins. Historical identities MUST remain addressable and MUST NOT be rewritten in place.

#### Scenario: Positive - parser update is reviewed
- **GIVEN** a new pinned parser revision with changed syntax boundaries
- **WHEN** the update protocol runs
- **THEN** attachment/span fixtures, adapter parity, tool lock, impacts, and compatibility records are updated together

#### Scenario: Negative - lockfile-only verifier update
- **GIVEN** a verifier/solver lock update without refreshed proof records and negative controls
- **WHEN** the final gate runs
- **THEN** formal bindings are stale and `formally-proved` MUST NOT be emitted

## REMOVED Requirements

### Requirement: RSC-004 Reviewable provenance and closure inventory
`devenv.lock`, `Cargo.lock`, and Git provide the immutable pins and provenance. A separate committed inventory copies the narHash and checksum values from the locks, so it adds a synchronization surface without an independent guarantee. Provenance review uses the locks and Git history instead.
