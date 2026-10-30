use buoy_core::{SpecGraph, validate_graph};

fn validate_saved_graph(graph: &SpecGraph) {
    drop(validate_graph(graph));
}

fn main() {
    let effectful_graph_loader = || {
        let _ = std::fs::read("ambient-repository-state");
        SpecGraph::default()
    };
    validate_saved_graph(effectful_graph_loader);
}
