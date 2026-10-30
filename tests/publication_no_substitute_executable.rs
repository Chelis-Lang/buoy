#![doc = "Task 12.5: proves no publication code path can invoke a substitute executable."]
#![allow(missing_docs, clippy::expect_used)]

use std::fs;

const SCANNED_FILES: [&str; 4] = [
    "src/publication.rs",
    "src/publication_fs.rs",
    "src/external_link_generation.rs",
    "src/publication_models.rs",
];
const FORBIDDEN_MARKERS: [&str; 4] = [
    "Command::new",
    "process::Command",
    "std::process::",
    "exec(",
];

/// Every module the publication command family calls into is scanned
/// directly, rather than trusting a behavioral proxy: `buoy publication`
/// runs entirely in-process (see `src/cli.rs`'s `publication_command`, which
/// calls only `crate::publication`, `crate::external_link_generation`, and
/// `crate::publication_fs` functions). None of these modules may spawn a
/// process, so a caller-controlled `PATH` or any other executable-selection
/// environment variable can never substitute the code that generates or
/// approves publication bytes.
#[test]
fn no_publication_module_can_spawn_a_substitute_process() {
    for path in SCANNED_FILES {
        let text = fs::read_to_string(path).expect(path);
        for marker in FORBIDDEN_MARKERS {
            assert!(
                !text.contains(marker),
                "{path} contains `{marker}`; publication code must remain in-process"
            );
        }
    }
}

/// `src/cli.rs`'s `publication_command` function itself (the sole dispatch
/// point for `buoy publication check|generate|evidence`) must also contain
/// no process-spawning marker, proving the CLI boundary does not delegate
/// to an external Buoy executable either.
#[test]
fn publication_command_dispatch_function_spawns_no_process() {
    let text = fs::read_to_string("src/cli.rs").expect("src/cli.rs");
    let start = text
        .find("fn publication_command(")
        .expect("publication_command must exist");
    // The next top-level `fn ` after the opening brace marks the end of
    // this function's body for a simple textual scan.
    let body_start = text[start..].find('{').expect("function body") + start;
    let rest = &text[body_start..];
    let next_fn = rest[1..]
        .find("\nfn ")
        .map_or(rest.len(), |offset| offset + 1);
    let body = &rest[..next_fn];
    for marker in FORBIDDEN_MARKERS {
        assert!(
            !body.contains(marker),
            "publication_command contains `{marker}`"
        );
    }
}
