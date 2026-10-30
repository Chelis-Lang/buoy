#![doc = "Construct and validate one typed Buoy graph."]

use buoy::core::{
    Atom, AtomKind, AtomRevisionRef, ExternalTraceLink, ExternalTraceRelation, ExternalTraceTarget,
    LifecycleState, ScopeId, SpecGraph, StableId, validate_graph,
};
use serde::Serialize;
use std::process::ExitCode;

#[derive(Serialize)]
struct ExampleReport {
    diagnostics: Vec<String>,
    graph: &'static str,
}

fn atom(statement: &str) -> Result<Atom, &'static str> {
    let id = StableId::new("EXAMPLE-ATOM").map_err(|_| "EXAMPLE-STABLE-ID")?;
    let scope = ScopeId::new("example.core").map_err(|_| "EXAMPLE-SCOPE-ID")?;
    Atom::new(
        id,
        LifecycleState::Proposed,
        AtomKind::Behavioral,
        vec![scope],
        statement,
        None,
    )
    .map_err(|_| "EXAMPLE-ATOM")
}

fn graph(invalid: bool) -> Result<SpecGraph, &'static str> {
    let original = atom("The example MUST preserve exact atom revisions.")?;
    let atom_id = original.meta().id().clone();
    let link = ExternalTraceLink::new(
        StableId::new("EXAMPLE-TRACE").map_err(|_| "EXAMPLE-TRACE-ID")?,
        vec![AtomRevisionRef::new(atom_id, original.revision())],
        ExternalTraceRelation::Documents,
        ExternalTraceTarget::new("urn:buoy:example:v1").map_err(|_| "EXAMPLE-TARGET")?,
    )
    .map_err(|_| "EXAMPLE-TRACE")?;
    let current = if invalid {
        atom("The revised example MUST preserve exact atom revisions.")?
    } else {
        original
    };
    Ok(SpecGraph {
        atoms: vec![current],
        external_trace_links: vec![link],
        ..SpecGraph::default()
    })
}

fn run(invalid: bool) -> Result<(String, bool), &'static str> {
    let diagnostics = validate_graph(&graph(invalid)?)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    let valid = diagnostics.is_empty();
    let report = ExampleReport {
        diagnostics,
        graph: if valid { "valid" } else { "invalid" },
    };
    serde_json::to_string(&report)
        .map(|output| (output, valid))
        .map_err(|_| "EXAMPLE-REPORT")
}

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let invalid = match arguments.as_slice() {
        [] => false,
        [flag] if flag == "--invalid" => true,
        _ => {
            eprintln!("usage: cargo run --example core_graph [-- --invalid]");
            return ExitCode::from(64);
        }
    };
    match run(invalid) {
        Ok((output, valid)) => {
            println!("{output}");
            if valid {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
        Err(code) => {
            eprintln!("{code}");
            ExitCode::from(70)
        }
    }
}
