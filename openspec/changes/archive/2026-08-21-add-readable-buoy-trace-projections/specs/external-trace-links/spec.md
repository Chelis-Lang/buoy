## MODIFIED Requirements

### Requirement: ETL-003 Repository-owned link syntax

A `buoy-external-trace-v1` Markdown fence MUST contain one strict link document. Unknown, duplicate, or malformed fields MUST fail closed.

The Markdown adapter and the opt-in OpenSpec version 2 nonauthority parser MUST produce equal link values from equal fence bytes.

#### Scenario: Positive - one Markdown fence creates one link

- **GIVEN** a selected Markdown file with one valid `buoy-external-trace-v1` fence
- **WHEN** the Markdown adapter processes the file
- **THEN** the graph contains one link with an exact fence source mapping

#### Scenario: Positive - OpenSpec version 2 reuses the parser

- **GIVEN** equal valid fence bytes in a Markdown file and an OpenSpec version 2 nonauthority section
- **WHEN** both adapters process their files
- **THEN** both outputs contain equal external trace-link values with file-specific source mappings

#### Scenario: Negative - prose URL resembles a link

- **GIVEN** ordinary prose with an issue URL and no authoritative trace-link fence
- **WHEN** either adapter processes the file
- **THEN** no external trace link is created from the prose URL

#### Scenario: Negative - parser behavior differs by host adapter

- **GIVEN** one malformed trace fence supplied through both accepted host adapters
- **WHEN** deterministic validation runs
- **THEN** both paths fail with the same primary trace-document diagnostic
