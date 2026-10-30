# Version 1 repository input and text policy

Status: frozen schema decision for task 2.2. This document defines the portable repository snapshot, selected-root, path, link, Unicode, line-ending, terminal-newline, and Markdown-trivia rules used by every v1 adapter and canonical report.

## Explicit snapshot boundary

The imperative shell constructs a snapshot and passes immutable values into the deterministic core. The core never discovers files or consults the working directory, Git index, `.gitignore`, environment, locale, filesystem metadata, or host path APIs.

A v1 snapshot contains:

- the schema identity;
- the repository configuration bytes;
- one unique canonical path and file-byte value for every selected regular file;
- explicit entry-kind observations needed to reject links and submodules; and
- pinned semantic tool identities required by the active operation.

File discovery order is nonsemantic. The shell sorts only after path normalization and duplicate rejection.

## Canonical repository-relative paths

Input paths MUST be valid UTF-8 and MUST use `/` as their separator. Canonicalization applies Unicode NFC to every segment and rejoins segments with one `/`. Canonical paths are compared and ordered by their normalized UTF-8 bytes, unsigned byte by unsigned byte.

A path is rejected if it:

- is empty or absolute;
- begins with `/`, a drive prefix, or a UNC prefix;
- ends with `/`;
- contains an empty, `.`, or `..` segment;
- contains NUL, an ASCII control, or a backslash;
- contains `<`, `>`, `:`, `"`, `|`, `?`, or `*`;
- has a segment ending in a space or `.`;
- has a segment whose basename before the first `.` is, under ASCII case folding, `CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, or `LPT1` through `LPT9`; or
- changes into another already-present path after NFC normalization.

The shell also computes a portable collision key by applying Unicode NFC followed by full Unicode default case folding to each segment. Two distinct canonical paths with the same portable collision key are rejected. This prevents a snapshot accepted on a case-sensitive host from becoming unrepresentable or aliased on a case-insensitive host.

No percent decoding, URL decoding, tilde expansion, separator conversion, locale collation, or filesystem case folding occurs in the deterministic core.

## Symlinks and other link-like entries

Version 1 does not follow symlinks.

A selected path is rejected if the entry itself or any traversed component below the explicit repository root is a symbolic link, junction, mount-point alias exposed as a link, or another host link type. The shell supplies the observed entry kind so the core can validate the policy without rereading the filesystem.

Hard links are not semantically observable in v1: the snapshot is keyed by canonical path and bytes, not inode identity. Two paths with equal bytes remain two files. Inode and device numbers never enter canonical input.

This fail-closed rule avoids repository escapes, alias-dependent duplicate authority, and host-specific link resolution. A future schema may add explicit content-addressed link support but cannot reinterpret a v1 snapshot.

## Submodules and nested repositories

Version 1 does not recurse into Git submodules or repository links. A selected gitlink/submodule entry is rejected with a deterministic diagnostic. A selector may explicitly exclude the entry itself, but it cannot select files beneath it as part of the parent snapshot.

A separately snapshotted nested repository can be checked as an independent repository with its own configuration and graph. Cross-repository references are unsupported in v1 unless a later schema defines an external reference class.

Ordinary nested `.git` directories are never selected by implicit convention; selection is controlled only by repository configuration. Git state does not determine canonical meaning.

## Selector and expected-authority policy

Repository configuration contains a closed list of selectors. Each selector declares:

- one canonical root path;
- a syntax adapter and exact schema version;
- explicit include and exclude patterns under the product's versioned portable glob grammar; and
- `authority_expectation`, exactly one of `forbidden`, `optional`, or `required`.

Selectors are a set ordered canonically by root, adapter ID, and pattern bytes. Two selectors MUST NOT select the same path. Ambiguous overlap is an error rather than first-match or last-match behavior.

The shell MUST NOT treat `.gitignore`, `.ignore`, editor settings, current-directory conventions, or provider configuration as implicit selectors. If a repository wants equivalent exclusions, it records them in product configuration.

Expectation semantics are:

- `forbidden`: any authority produced in the selector is an error;
- `optional`: zero authorities is valid, but every selected input still receives a semantic, nonnormative, or diagnostic disposition; and
- `required`: the selector as a whole MUST produce at least one valid authority, or validation fails instead of accepting an empty graph.

A selected path that matches no supported syntax version or cannot be decoded as required by its adapter produces a deterministic diagnostic. It is never silently dropped.

## Resource-budget boundary

Repository configuration carries explicit positive budgets for path depth, directory entries, selected files, per-file bytes, aggregate bytes, materialized graph objects, direct graph references, and retained stable diagnostics. Defaults are respectively `64`, `100000`, `10000`, `16777216`, `536870912`, `200000`, `2000000`, and `100000`; closed upper bounds prevent a configuration from disabling the boundary.

Discovery enforces path and entry limits while walking, before retaining selected content. Snapshot construction checks per-file and aggregate byte limits before reads or aggregate retention. Shared adapter checking accounts for objects, direct references, and diagnostics before extending the aggregate graph or diagnostic set. Exceeding a limit fails closed with its stable `REH-BUDGET-*` code; no partial graph or report is accepted.

## Unicode policy

Paths and semantic text MUST be valid UTF-8. V1 canonical semantic strings use Unicode NFC. The adapter retains raw source bytes and byte spans for diagnostics, but canonical semantic values use their NFC form.

If two distinct semantic keys become equal after NFC normalization, validation fails as a collision. The system never chooses one spelling. Case is preserved for semantic text and stable IDs except where a schema defines an ASCII-only grammar.

Unicode version is a semantic toolchain input. The pinned normalization and full-case-fold data version is recorded by the active v1 schema/tool lock; changing it requires a schema/toolchain migration and cannot silently change existing canonical identities.

Invalid UTF-8 in a text-selected file is an error. A future binary adapter may accept arbitrary bytes, but Markdown and Rust adapters do not.

## Line endings and terminal newlines

For Markdown, TOML metadata, and other v1 text syntax:

- LF and CRLF line endings are accepted;
- CRLF is normalized to LF for parsed semantic text;
- a lone CR is invalid;
- raw file bytes remain available for byte-bound artifact digests; and
- source byte spans are measured in the original bytes before semantic newline normalization.

The presence or absence of one terminal file newline is syntax trivia for parser-extracted semantic objects. It does not alter an atom revision or another parsed object digest. It remains significant to a raw file-byte digest and therefore to any derivation or proof binding that explicitly binds the complete file bytes.

For an atom `statement`, TOML decoding occurs first. The semantic text then undergoes CRLF-to-LF conversion, NFC normalization, removal of trailing spaces and tabs on each line, and removal of leading and trailing blank lines. Internal line breaks, indentation, spaces, and Markdown punctuation remain significant. Reflowing normative prose can therefore change its revision; the system does not guess that two differently wrapped statements mean the same thing.

## Markdown syntax trivia

For an authoritative fenced-TOML object:

- fence marker choice, marker length, indentation accepted by CommonMark, TOML key order, TOML quoting form, and TOML comments are nonsemantic;
- the exact decoded field values are semantic according to the object's schema;
- surrounding headings, prose, link definitions, block quotes, and HTML comments are nonsemantic to the object;
- a fence nested as literal text inside another code block is not an authority node;
- malformed authority-like fences produce diagnostics rather than fallback prose; and
- raw regular-expression matches can support advisory migration reports only.

CommonMark parser identity and version are pinned semantic tool inputs. A parser upgrade that changes syntax-node boundaries requires fixture review and a schema/toolchain migration.

## Diagnostic path and span policy

Canonical diagnostics contain only:

- the canonical repository-relative path;
- the schema-defined byte-span convention over original file bytes;
- a stable diagnostic code;
- structured canonical fields; and
- schema-defined deterministic rendering data.

They exclude absolute roots, symlink targets, inode data, usernames, temporary paths, terminal width, and raw host error strings. When the shell cannot safely open or classify an entry, it maps the failure to a stable diagnostic category and keeps raw operating-system text in a noncanonical channel.

## Rejected alternatives

- **Raw platform paths:** preserve host quirks and make cross-root and cross-platform identity unreliable.
- **Implicit Git ignore semantics:** convenient, but couples meaning to nested ignore discovery and tool-version behavior.
- **Following in-repository symlinks:** permits aliasing, escapes, and duplicate authorities whose meaning depends on host resolution.
- **Recursive submodule snapshots:** imports mutable repository boundaries and complicates offline closure.
- **NFKC for semantic content:** folds compatibility distinctions that authors may intend. NFC is the narrower normalization.
- **Whitespace collapse for normative text:** avoids editorial invalidations but can also merge semantically distinct Markdown and code fragments. V1 uses conservative syntax-aware normalization.
