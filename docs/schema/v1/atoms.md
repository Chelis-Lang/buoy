# Version 1 atom authority and identity

Status: frozen schema decision for task 2.1. This document defines the product's atom syntax and authority model. Later v1 schema documents define path/text normalization and canonical byte encoding; they may not change the semantic field set or authority rules frozen here.

## Product boundary

OpenSpec is a planning and reference workflow for this repository. The product MUST NOT parse OpenSpec changes, import OpenSpec lifecycle state, use OpenSpec as an atom source format, or require an OpenSpec library or command at runtime. OpenSpec validation is a development gate only.

The v1 product syntax is owned and versioned by Buoy.

## Authoritative Markdown syntax

A v1 atom authority is exactly one CommonMark fenced code block whose info string is exactly:

```text
spec-provenance-atom-v1
```

The fence body is one strict TOML 1.0 document. It contains one atom and no other object. Key order and TOML layout are nonsemantic. Duplicate keys, unknown keys, unsupported TOML value kinds, and invalid UTF-8 are errors.

An illustrative authority follows. Four outer backticks keep the inner fence non-authoritative in this document.

````markdown
```spec-provenance-atom-v1
id = "SPG-001"
state = "ratified"
kind = "behavioral"
scope = ["spec-provenance.graph"]
statement = """
The graph builder MUST reject two authority declarations for the same live atom identifier.
"""

[approval]
authority = "spec-provenance-maintainers"
record = "reviews/SPG-001-ratification.md"
digest = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
```
````

The fenced block is the complete authority extent. Headings, paragraphs, block quotes, HTML comments, adjacent metadata, link definitions, and enclosing sections do not add semantic fields or authority. An identical-looking fence nested as literal content inside another code fence is not a syntax node and is not authority.

An info string beginning with `spec-provenance-atom-` but naming any version other than `v1` is an unsupported-version error in a selected input, not ordinary prose. Other fenced code blocks are nonnormative unless another product schema explicitly gives their info string semantics.

## Closed field set

### Required fields

- `id`: stable atom identity using the grammar below.
- `state`: exactly `proposed` or `ratified` for a live atom authority.
- `kind`: exactly `behavioral`, `structural`, or `constraint`.
- `scope`: a nonempty array of unique stable scope identifiers. Input order is nonsemantic; canonical order is ascending canonical UTF-8 bytes.
- `statement`: nonempty, self-contained normative UTF-8 text. It MUST contain at least one uppercase RFC 2119 term: `MUST`, `MUST NOT`, `SHALL`, `SHALL NOT`, `SHOULD`, `SHOULD NOT`, or `MAY`.

Scope identifiers use lowercase ASCII and match:

```text
^[a-z][a-z0-9]*(?:[.-][a-z0-9]+)*$
```

A statement may contain Markdown inline syntax. The v1 atom statement is one normative decision; splitting or combining independent decisions is a review concern because no parser can establish semantic atomicity.

### Approval table

`approval` MUST be absent for `proposed` atoms and present for `ratified` atoms. Its closed fields are:

- `authority`: a stable approval-authority ID declared for the atom's ID namespace by repository configuration.
- `record`: a normalized repository-relative path to a repository-owned review record.
- `digest`: the full algorithm-qualified digest of that record under the active schema.

Approval proves only that the configured authority ratified the statement. It is review evidence, not proof that the requirement is correct or implemented. Provider users, labels, pull-request reviews, timestamps, and issue state cannot satisfy this binding unless a shell adapter first materializes a repository-owned record; those provider facts are never canonical inputs themselves.

## Stable ID grammar and allocation

Atom IDs are uppercase ASCII and match:

```text
^[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+$
```

An ID is 3 through 128 bytes, compared byte-for-byte, and never case-folded. Examples include `SPG-001`, `DET-PATH-002`, and `CHELIS-NUM-17`.

A repository-owned append-only v1 ID registry is the allocation authority. Each registered namespace declares its approval authority and each allocated ID exactly once. The registry reserves live and retired IDs permanently. Allocation is an explicit repository edit; the deterministic core never invents an ID from content, a path, a heading, issue state, traversal order, or a counter stored outside the repository.

Moving an authority does not rename its atom. Renaming an atom is a retirement of the old ID plus allocation of a new ID. Deleting authority without a valid terminal tombstone is an error. A retired ID can never return to the live registry.

The ID registry establishes identity and reservation. The atom fence establishes the sole normative authority for the live atom's content. Neither can substitute for the other.

## Authority and lifecycle

Configured authority roots are explicit repository inputs. Across all selected authority roots:

- A live ID MUST have exactly one authoritative v1 fence.
- References, summaries, examples, carriers, implementation links, reports, and registry entries MUST NOT create or redefine normative authority.
- Duplicate authority fences fail; source order, path order, and lifecycle state do not choose a winner.
- Only `proposed -> ratified`, `proposed -> withdrawn`, `ratified -> superseded`, and `ratified -> withdrawn` transitions are valid.
- `withdrawn` and `superseded` are represented only by tombstones, not live atom fences.
- Ratification requires a valid approval binding. Proposed atoms cannot satisfy a policy that requires ratified authority.

## Semantic revision boundary

The atom revision digest covers exactly these semantic fields under the v1 canonical encoding:

1. `kind`;
2. the canonical set of `scope` identifiers; and
3. the canonical `statement` text.

The revision digest excludes:

- `id`, which denotes stable identity rather than revision;
- lifecycle `state`;
- the approval binding;
- authority path and source span;
- TOML key order, quoting choice, comments, and other syntax trivia declared nonsemantic by the v1 text policy.

Changing `kind`, `scope`, or `statement` changes the revision. Moving or reformatting an otherwise semantically identical authority preserves the revision while changing source-map and complete-graph identity. Ratifying unchanged proposed content preserves its atom revision but changes lifecycle and complete-object identity.

A wording change that could invalidate a carrier SHOULD retire and replace the atom rather than merely update its revision; the configured approval authority makes that review decision. In either case, change-impact checking exposes every affected binding.

## Rejected alternatives

- **Markdown blockquote atoms:** readable, but metadata and extent become dependent on formatting and adjacency.
- **Heading plus HTML-comment metadata:** splits authority across syntax nodes and makes attachment ambiguous.
- **YAML front matter:** naturally file-scoped, so it does not support multiple local authorities without another attachment convention.
- **Sidecar-only JSON or TOML atoms:** mechanically simple but separates normative text from the specification context and encourages parallel stale copies.
- **OpenSpec requirement headings:** useful planning references, but coupling product authority and lifecycle to OpenSpec violates the repository-independent boundary.
