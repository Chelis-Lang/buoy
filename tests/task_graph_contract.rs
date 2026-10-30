#![doc = "Contract tests for the resolved Devenv task graph's blocking and advisory edges."]
#![allow(missing_docs, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// One parsed task's declared `after` and `before` edges.
struct TaskEdges {
    after: Vec<String>,
    before: Vec<String>,
}

/// Parses every `"name" = { ... after = [ ... ]; before = [ ... ]; ... };`
/// task block from the Nix task-graph source into a name-to-edges map.
///
/// This is a deliberately narrow textual parser: it reads only the `after`
/// and `before` list literals inside each quoted task block, which is
/// exactly the graph-shape surface this contract needs.
fn parse_tasks(source: &str) -> BTreeMap<String, TaskEdges> {
    let mut tasks = BTreeMap::new();
    let mut search_from = 0_usize;
    while let Some(relative) = source[search_from..].find("\"buoy:") {
        let start = search_from + relative;
        let Some(name_end) = source[start + 1..].find('"') else {
            break;
        };
        let name = source[start + 1..start + 1 + name_end].to_owned();
        let after_quote = start + 1 + name_end + 1;
        // Only a task *definition* line — `"buoy:name" = {` — starts a new
        // block. A quoted `"buoy:name"` reference inside another task's own
        // `after`/`before` dependency list must never be mistaken for a
        // definition: matching it as one corrupts the following block's
        // extracted fields, since the scan then resumes mid-list rather
        // than at the next real task header.
        if !source[after_quote..].trim_start().starts_with("= {") {
            search_from = start + 1;
            continue;
        }
        let block_start = start;
        let block_end = source[block_start + 1..]
            .find("\n    \"")
            .map_or(source.len(), |relative| block_start + 1 + relative);
        let block = &source[block_start..block_end];
        tasks.insert(
            name,
            TaskEdges {
                after: extract_list(block, "after"),
                before: extract_list(block, "before"),
            },
        );
        search_from = block_end;
    }
    tasks
}

fn extract_list(block: &str, key: &str) -> Vec<String> {
    let marker = format!("{key} = [");
    let Some(start) = block.find(&marker) else {
        return Vec::new();
    };
    let content_start = start + marker.len();
    let Some(end) = block[content_start..].find(']') else {
        return Vec::new();
    };
    block[content_start..content_start + end]
        .split_whitespace()
        .map(|token| token.trim_matches('"').to_owned())
        .collect()
}

/// Every task that can transitively reach `devenv:enterTest` through a
/// `before` edge, or is reached transitively via an `after` edge from a task
/// that itself reaches `devenv:enterTest`.
fn ancestors_of_enter_test(tasks: &BTreeMap<String, TaskEdges>) -> BTreeSet<String> {
    // A task X blocks enterTest when: X declares `before = [ ... "devenv:enterTest" ... ]`,
    // X declares `before = [Y]` for some other blocking task Y, or X is one
    // of some blocking task Y's own `after` dependencies (Y depends on X, so
    // X necessarily runs before Y, and therefore before enterTest too).
    let mut blocking: BTreeSet<String> = tasks
        .iter()
        .filter(|(_, edges)| edges.before.iter().any(|value| value == "devenv:enterTest"))
        .map(|(name, _)| name.clone())
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for (name, edges) in tasks {
            if !blocking.contains(name) {
                // X declares `before = [Y]` for some already-blocking Y.
                if edges.before.iter().any(|target| blocking.contains(target))
                    && blocking.insert(name.clone())
                {
                    changed = true;
                }
                continue;
            }
            // `name` is blocking: everything it depends on through its own
            // `after` list necessarily runs before it, so those
            // dependencies are on the path to enterTest too.
            for dependency in &edges.after {
                if blocking.insert(dependency.clone()) {
                    changed = true;
                }
            }
        }
    }
    blocking
}

fn read_tasks_source() -> String {
    fs::read_to_string("nix/devenv/tasks.nix").expect("devenv task graph source")
}

/// The single registered owner of every consumer-package task body.
const PACKAGE_TASK_OWNER: &str = "nix/tools/package-contract.sh";

/// Extracts one task block's full text, including its `exec` body.
fn task_block<'a>(source: &'a str, name: &str) -> &'a str {
    let marker = format!("\"{name}\" = {{");
    let Some(start) = source.find(&marker) else {
        return "";
    };
    let end = source[start..]
        .find("\n    };")
        .map_or(source.len(), |relative| start + relative);
    &source[start..end]
}

/// Audit M17, tasks 10.11/10.12: exactly one registered owner implements the
/// consumer-package task body. The blocking internal task and the advisory
/// downstream task each invoke that one owner; neither restates a step
/// inline, so a task body cannot drift from the gate it claims to run.
#[test]
fn both_package_tasks_invoke_exactly_one_registered_owner() {
    let source = read_tasks_source();
    assert!(
        std::path::Path::new(PACKAGE_TASK_OWNER).exists(),
        "the single registered task owner must be a real committed file"
    );

    let blocking = task_block(&source, "buoy:package-contract");
    let advisory = task_block(&source, "buoy:consumer-package");
    assert!(!blocking.is_empty() && !advisory.is_empty());

    for (label, block, mode) in [
        ("buoy:package-contract", blocking, "--mode blocking"),
        ("buoy:consumer-package", advisory, "--mode advisory"),
    ] {
        assert!(
            block.contains(&format!("{PACKAGE_TASK_OWNER} {mode}")),
            "{label} must invoke the single registered owner with {mode}"
        );
        // No duplicated step: the owner alone may name the underlying gate
        // binaries and scripts.
        for duplicated in [
            "check-package-contract",
            "check-consumer-package",
            "verify-offline-realization",
            "check-derivation-contract",
            "derive-source-content-digest",
        ] {
            assert!(
                !block.contains(duplicated),
                "{label} duplicates `{duplicated}` instead of delegating to {PACKAGE_TASK_OWNER}"
            );
        }
    }

    // The devenv gate script for the same control delegates to the same
    // owner rather than restating the steps a third time.
    let core_scripts =
        fs::read_to_string("nix/devenv/scripts/core.nix").expect("devenv script source");
    let script_start = core_scripts
        .find("scripts.buoy-package-contract")
        .expect("the registered gate script");
    let script_block = &core_scripts[script_start..script_start + 600];
    assert!(
        script_block.contains(PACKAGE_TASK_OWNER),
        "the buoy-package-contract gate script must delegate to the single owner"
    );
    assert!(
        !script_block.contains("verify-offline-realization"),
        "the buoy-package-contract gate script must not duplicate the owner's steps"
    );
}

/// The single owner really implements both modes, and the blocking mode --
/// the only one bound into `devenv test` -- is the only one that realizes.
#[test]
fn the_registered_owner_implements_a_blocking_and_an_advisory_mode() {
    let owner = fs::read_to_string(PACKAGE_TASK_OWNER).expect("registered task owner");
    for required in [
        "check-package-contract",
        "verify-offline-realization.sh",
        "check-derivation-contract.sh",
        "check-consumer-package",
    ] {
        assert!(owner.contains(required), "the owner must run {required}");
    }
    let blocking_start = owner.find("\nblocking)").expect("blocking mode");
    let advisory_start = owner.find("\nadvisory)").expect("advisory mode");
    let blocking_body = &owner[blocking_start..advisory_start];
    let advisory_body = &owner[advisory_start..];
    assert!(
        blocking_body.contains("verify-offline-realization.sh"),
        "the blocking mode must realize offline"
    );
    assert!(
        !advisory_body.contains("verify-offline-realization.sh"),
        "the advisory mode must not enter the blocking realization path"
    );
    assert!(
        advisory_body.contains("check-consumer-package"),
        "the advisory mode owns the private git+ssh acquisition check"
    );
}

#[test]
fn package_contract_task_is_declared_blocking() {
    let source = read_tasks_source();
    let tasks = parse_tasks(&source);
    assert!(tasks.contains_key("buoy:package-contract"));
    let ancestors = ancestors_of_enter_test(&tasks);
    assert!(
        ancestors.contains("buoy:package-contract"),
        "buoy:package-contract must have a path to devenv:enterTest"
    );
}

#[test]
fn consumer_package_task_is_advisory_and_unreachable() {
    let source = read_tasks_source();
    let tasks = parse_tasks(&source);
    let edges = tasks
        .get("buoy:consumer-package")
        .expect("buoy:consumer-package must be declared");
    assert!(
        edges.after.is_empty() && edges.before.is_empty(),
        "the advisory task must declare no after/before edge"
    );
    let ancestors = ancestors_of_enter_test(&tasks);
    assert!(
        !ancestors.contains("buoy:consumer-package"),
        "buoy:consumer-package must have no path to devenv:enterTest"
    );
}

#[test]
fn gate_omission_negative_control_detects_a_missing_blocking_edge() {
    // A minimal graph that omits the blocking edge must fail this same
    // reachability check, proving the positive assertion above is not
    // vacuously true.
    let mut tasks = BTreeMap::new();
    tasks.insert(
        String::from("buoy:package-contract"),
        TaskEdges {
            after: vec![String::from("buoy:specs")],
            before: Vec::new(),
        },
    );
    tasks.insert(
        String::from("buoy:gate"),
        TaskEdges {
            after: vec![String::from("buoy:specs")],
            before: vec![String::from("devenv:enterTest")],
        },
    );
    let ancestors = ancestors_of_enter_test(&tasks);
    assert!(
        !ancestors.contains("buoy:package-contract"),
        "an omitted blocking edge must be detected as unreachable, not silently accepted"
    );
}

#[test]
fn every_downstream_publication_task_stays_outside_the_blocking_graph() {
    let source = read_tasks_source();
    let tasks = parse_tasks(&source);
    // Buoy's own resolved task graph declares no downstream publication
    // task at all: a consumer repository's `buoy publication check`,
    // `generate`, or `evidence` invocation is never a devenv task defined
    // here, so this loop has nothing to iterate over today. Assert that
    // fact explicitly rather than let an empty loop pass vacuously.
    let publication_tasks: Vec<&String> = tasks
        .keys()
        .filter(|name| name.contains("publication"))
        .collect();
    assert!(
        publication_tasks.is_empty(),
        "Buoy's own task graph currently declares no downstream publication task; \
         if one is added, DEV-010 requires it stay outside the blocking graph"
    );
    let ancestors = ancestors_of_enter_test(&tasks);
    for name in &publication_tasks {
        assert!(
            !ancestors.contains(*name),
            "{name} is a downstream publication task and must remain advisory"
        );
    }
}

#[test]
fn a_hypothetical_blocking_publication_task_would_be_detected() {
    // The positive assertion above is not vacuously true only because no
    // publication task currently exists: this proves the same detection
    // logic actually catches a publication task that *is* wired into the
    // blocking chain, exactly like the package-contract gate-omission
    // control above.
    let mut tasks = BTreeMap::new();
    tasks.insert(
        String::from("buoy:publication-check"),
        TaskEdges {
            after: vec![String::from("buoy:specs")],
            before: Vec::new(),
        },
    );
    tasks.insert(
        String::from("buoy:gate"),
        TaskEdges {
            after: vec![String::from("buoy:publication-check")],
            before: vec![String::from("devenv:enterTest")],
        },
    );
    let ancestors = ancestors_of_enter_test(&tasks);
    assert!(
        ancestors.contains("buoy:publication-check"),
        "a publication task wired into the blocking chain must be detected as blocking"
    );
}
