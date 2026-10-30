#![doc = "Seeded properties and an exhaustive bounded model for production source traversal."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use buoy_core::{
    ParsedItemIdentity, RepositoryPath, SchemaId, SourceBudgets, SourceDepthBudget, SourceEdge,
    SourceEdgeBudget, SourceEdgeKind, SourceGraph, SourceGraphError, SourceItemKind, SourceNode,
    SourceNodeBudget, SourceNodeRef, SourceVisitedBudget, source_root_closure,
};
use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use std::collections::BTreeSet;

#[path = "../../../tests/support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

/// Widest node count the bitmask reference model represents.
const REFERENCE_CAPACITY: usize = 8;

/// One directed source graph over dense node indices, before production values.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Model {
    node_count: usize,
    edges: Vec<(usize, usize)>,
    roots: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Production graph construction over dense node indices
// ---------------------------------------------------------------------------

/// Builds one distinct leaf node per index.
fn model_nodes(node_count: usize) -> Vec<SourceNode> {
    (0..node_count)
        .map(|index| {
            let item = ParsedItemIdentity::new(
                SchemaId::new("model/v1").expect("valid schema"),
                RepositoryPath::new("src/lib.rs").expect("valid path"),
                SourceItemKind::new("model/node/v1").expect("valid source item kind"),
                vec![u8::try_from(index).expect("bounded model index")],
                None,
            )
            .expect("valid parsed item");
            SourceNode::new(item, Vec::new()).expect("valid leaf node")
        })
        .collect()
}

/// Orders node references so that index order equals canonical digest order.
fn model_references(nodes: &[SourceNode]) -> Vec<SourceNodeRef> {
    let mut references = nodes.iter().map(SourceNode::reference).collect::<Vec<_>>();
    references.sort_unstable();
    references
}

fn model_budgets(nodes: u32, edges: u32, depth: u32, visited: u32) -> SourceBudgets {
    SourceBudgets::new(
        SourceNodeBudget::new(nodes.max(1)).expect("positive node budget"),
        SourceEdgeBudget::new(edges.max(1)).expect("positive edge budget"),
        SourceDepthBudget::new(depth.max(1)).expect("positive depth budget"),
        SourceVisitedBudget::new(visited.max(1)).expect("positive visited budget"),
    )
}

fn generous_budgets() -> SourceBudgets {
    model_budgets(64, 64, 64, 64)
}

/// Materializes one production graph from dense indices, or reports its rejection.
fn build(
    nodes: &[SourceNode],
    references: &[SourceNodeRef],
    model: &Model,
    budgets: SourceBudgets,
) -> Result<SourceGraph, SourceGraphError> {
    let edges = model
        .edges
        .iter()
        .map(|(source, target)| {
            SourceEdge::new(
                references[*source],
                SourceEdgeKind::Implements,
                None,
                references[*target],
            )
            .expect("a navigation edge needs no child position")
        })
        .collect();
    SourceGraph::new(
        nodes.to_vec(),
        edges,
        model.roots.iter().map(|root| references[*root]).collect(),
        budgets,
    )
}

/// Maps one canonical traversal back to dense node indices.
fn indices(order: &[SourceNodeRef], references: &[SourceNodeRef]) -> Vec<usize> {
    order
        .iter()
        .map(|node| {
            references
                .iter()
                .position(|candidate| candidate == node)
                .expect("every traversed node is indexed")
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Independent reference model: dense bitmask adjacency and transitive closure
// ---------------------------------------------------------------------------

fn reference_matrix(model: &Model) -> [u16; REFERENCE_CAPACITY] {
    let mut rows = [0_u16; REFERENCE_CAPACITY];
    for (source, target) in &model.edges {
        rows[*source] |= 1 << *target;
    }
    rows
}

fn reference_transitive_closure(model: &Model) -> [u16; REFERENCE_CAPACITY] {
    let mut rows = reference_matrix(model);
    for pivot in 0..model.node_count {
        let pivot_row = rows[pivot];
        for row in rows.iter_mut().take(model.node_count) {
            if *row & (1 << pivot) != 0 {
                *row |= pivot_row;
            }
        }
    }
    rows
}

fn reference_cyclic(model: &Model) -> bool {
    let closure = reference_transitive_closure(model);
    (0..model.node_count).any(|node| closure[node] & (1 << node) != 0)
}

fn reference_reachable(model: &Model) -> BTreeSet<usize> {
    let closure = reference_transitive_closure(model);
    let mut reached = BTreeSet::new();
    for root in &model.roots {
        reached.insert(*root);
        for node in 0..model.node_count {
            if closure[*root] & (1 << node) != 0 {
                reached.insert(node);
            }
        }
    }
    reached
}

/// Longest path in edges from any root, by explicit relaxation.
fn reference_depth(model: &Model) -> u32 {
    let matrix = reference_matrix(model);
    let reachable = reference_reachable(model);
    let mut longest = vec![0_u32; model.node_count];
    for _ in 0..model.node_count.max(1) {
        for parent in reachable.iter().copied() {
            for child in 0..model.node_count {
                if matrix[parent] & (1 << child) != 0 && longest[child] < longest[parent] + 1 {
                    longest[child] = longest[parent] + 1;
                }
            }
        }
    }
    reachable
        .iter()
        .map(|node| longest[*node])
        .max()
        .unwrap_or(0)
}

/// Independent canonical preorder over the bitmask adjacency.
fn reference_traverse(model: &Model) -> Vec<usize> {
    fn visit(
        matrix: &[u16; REFERENCE_CAPACITY],
        node_count: usize,
        node: usize,
        visited: &mut Vec<usize>,
    ) {
        if visited.contains(&node) {
            return;
        }
        visited.push(node);
        for child in 0..node_count {
            if matrix[node] & (1 << child) != 0 {
                visit(matrix, node_count, child, visited);
            }
        }
    }

    let matrix = reference_matrix(model);
    let mut roots = model.roots.clone();
    roots.sort_unstable();
    roots.dedup();
    let mut visited = Vec::new();
    for root in roots {
        visit(&matrix, model.node_count, root, &mut visited);
    }
    visited
}

// ---------------------------------------------------------------------------
// Seeded properties
// ---------------------------------------------------------------------------

fn draw_model(test_case: &TestCase, capacity: usize) -> Model {
    let node_count = test_case.draw(gs::integers::<usize>().min_value(1).max_value(capacity));
    let drawn = test_case.draw(
        gs::vecs(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(capacity * capacity - 1),
        )
        .max_size(2 * capacity),
    );
    let mut edges = drawn
        .into_iter()
        .map(|packed| (packed / capacity % node_count, packed % node_count))
        .collect::<Vec<_>>();
    edges.sort_unstable();
    edges.dedup();
    let mut roots = test_case.draw(
        gs::vecs(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(node_count - 1),
        )
        .min_size(1)
        .max_size(node_count),
    );
    roots.sort_unstable();
    roots.dedup();
    Model {
        node_count,
        edges,
        roots,
    }
}

fn permuted(model: &Model, keys: &[u8]) -> Model {
    let mut ordered = model
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| (keys.get(index).copied().unwrap_or(0), index, *edge))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(key, index, _)| (*key, *index));
    let mut roots = model.roots.clone();
    roots.reverse();
    Model {
        node_count: model.node_count,
        edges: ordered.into_iter().map(|(_, _, edge)| edge).collect(),
        roots,
    }
}

#[test]
fn hegel_source_traversal_is_input_order_independent() {
    Hegel::new(|test_case: TestCase| {
        let model = draw_model(&test_case, REFERENCE_CAPACITY);
        let keys = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(2 * REFERENCE_CAPACITY));
        let nodes = model_nodes(model.node_count);
        let references = model_references(&nodes);

        let forward = build(&nodes, &references, &model, generous_budgets());
        let mut reordered_nodes = nodes.clone();
        reordered_nodes.reverse();
        let reversed = build(
            &reordered_nodes,
            &references,
            &permuted(&model, &keys),
            generous_budgets(),
        );
        assert_eq!(
            forward, reversed,
            "input order changes neither the graph nor its diagnostic"
        );

        let (Ok(forward), Ok(reversed)) = (forward, reversed) else {
            return;
        };
        assert_eq!(forward.digest(), reversed.digest());
        assert_eq!(forward.nodes(), reversed.nodes());
        assert_eq!(forward.edges(), reversed.edges());
        assert_eq!(forward.roots(), reversed.roots());
        assert_eq!(
            source_root_closure(&forward).map(|closure| closure.order().to_vec()),
            source_root_closure(&reversed).map(|closure| closure.order().to_vec()),
        );
    })
    .settings(settings(405))
    .run();
}

#[test]
fn hegel_source_reachability_is_the_least_fixed_point_over_current_roots() {
    Hegel::new(|test_case: TestCase| {
        let model = draw_model(&test_case, REFERENCE_CAPACITY);
        if reference_cyclic(&model) {
            return;
        }
        let nodes = model_nodes(model.node_count);
        let references = model_references(&nodes);
        let graph = build(&nodes, &references, &model, generous_budgets())
            .expect("a bounded model graph closes");
        let closure = source_root_closure(&graph).expect("an acyclic model graph is bounded");
        let reached = indices(closure.order(), &references)
            .into_iter()
            .collect::<BTreeSet<_>>();

        assert_eq!(reached, reference_reachable(&model));
        assert_eq!(closure.visited() as usize, reached.len());
        for root in &model.roots {
            assert!(closure.contains(references[*root]));
        }

        let stepped = reached
            .iter()
            .flat_map(|node| {
                model
                    .edges
                    .iter()
                    .filter(move |(source, _)| source == node)
                    .map(|(_, target)| *target)
            })
            .chain(reached.iter().copied())
            .collect::<BTreeSet<_>>();
        assert_eq!(stepped, reached, "one more step must add nothing");

        let mut narrowed = model.clone();
        narrowed.roots.truncate(1);
        let narrow_graph = build(&nodes, &references, &narrowed, generous_budgets())
            .expect("a narrowed model graph closes");
        let narrow_closure =
            source_root_closure(&narrow_graph).expect("an acyclic model graph is bounded");
        assert!(
            indices(narrow_closure.order(), &references)
                .into_iter()
                .collect::<BTreeSet<_>>()
                .is_subset(&reached)
        );
    })
    .settings(settings(406))
    .run();
}

#[test]
fn hegel_source_cycles_are_rejected_at_construction() {
    Hegel::new(|test_case: TestCase| {
        let model = draw_model(&test_case, REFERENCE_CAPACITY);
        let nodes = model_nodes(model.node_count);
        let references = model_references(&nodes);
        let graph = build(&nodes, &references, &model, generous_budgets());

        if reference_cyclic(&model) {
            assert_eq!(graph, Err(SourceGraphError::Cycle));
        } else {
            assert!(
                source_root_closure(&graph.expect("an acyclic model graph closes")).is_ok(),
                "an acyclic bounded graph always traverses"
            );
        }

        let node = test_case.draw(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(model.node_count - 1),
        );
        let mut self_loop = model.clone();
        self_loop.edges.push((node, node));
        self_loop.edges.sort_unstable();
        self_loop.edges.dedup();
        assert_eq!(
            build(&nodes, &references, &self_loop, generous_budgets()),
            Err(SourceGraphError::Cycle),
            "a self loop never constructs"
        );

        let closure = reference_transitive_closure(&model);
        let ancestor = test_case.draw(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(model.node_count - 1),
        );
        let descendant = test_case.draw(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(model.node_count - 1),
        );
        if closure[ancestor] & (1 << descendant) != 0 {
            let mut closed = model.clone();
            closed.edges.push((descendant, ancestor));
            closed.edges.sort_unstable();
            closed.edges.dedup();
            assert_eq!(
                build(&nodes, &references, &closed, generous_budgets()),
                Err(SourceGraphError::Cycle),
                "an edge back to an ancestor closes a cycle"
            );
        }

        let mut unrooted = model.clone();
        unrooted.roots.clear();
        unrooted.roots.push(0);
        assert_eq!(
            build(&nodes, &references, &unrooted, generous_budgets()).is_err(),
            reference_cyclic(&model),
            "a narrower current root set never hides a cycle"
        );
    })
    .settings(settings(407))
    .run();
}

#[test]
fn hegel_source_traversal_respects_every_budget_and_fails_closed() {
    Hegel::new(|test_case: TestCase| {
        let model = draw_model(&test_case, REFERENCE_CAPACITY);
        if reference_cyclic(&model) {
            return;
        }
        let nodes = model_nodes(model.node_count);
        let references = model_references(&nodes);
        let node_count = u32::try_from(model.node_count).expect("bounded node count");
        let edge_count = u32::try_from(model.edges.len()).expect("bounded edge count");
        let visited = u32::try_from(reference_reachable(&model).len()).expect("bounded visited");
        let depth = reference_depth(&model);
        let exact = model_budgets(node_count, edge_count, depth, visited);

        let graph =
            build(&nodes, &references, &model, exact).expect("exact budgets admit the graph");
        let closure = source_root_closure(&graph).expect("exact budgets admit the closure");
        assert_eq!(closure.visited(), visited);
        assert_eq!(closure.depth(), depth);
        assert_eq!(
            indices(closure.order(), &references),
            reference_traverse(&model)
        );

        if node_count > 1 {
            assert_eq!(
                build(
                    &nodes,
                    &references,
                    &model,
                    model_budgets(node_count - 1, edge_count, depth, visited),
                ),
                Err(SourceGraphError::NodeBudget)
            );
        }
        if edge_count > 1 {
            assert_eq!(
                build(
                    &nodes,
                    &references,
                    &model,
                    model_budgets(node_count, edge_count - 1, depth, visited),
                ),
                Err(SourceGraphError::EdgeBudget)
            );
        }
        if visited > 1 {
            let narrow = build(
                &nodes,
                &references,
                &model,
                model_budgets(node_count, edge_count, depth, visited - 1),
            )
            .expect("a visited budget binds traversal, not construction");
            assert_eq!(
                source_root_closure(&narrow),
                Err(SourceGraphError::VisitedBudget)
            );
        }
        if depth > 1 {
            let shallow = build(
                &nodes,
                &references,
                &model,
                model_budgets(node_count, edge_count, depth - 1, visited),
            )
            .expect("a depth budget binds traversal, not construction");
            assert_eq!(
                source_root_closure(&shallow),
                Err(SourceGraphError::DepthBudget)
            );
        }
    })
    .settings(settings(408))
    .run();
}

// ---------------------------------------------------------------------------
// Exhaustive bounded model
// ---------------------------------------------------------------------------

/// Compares canonical order, reachability, and depth with the independent model.
fn check_agreement(nodes: &[SourceNode], references: &[SourceNodeRef], model: &Model) -> bool {
    let graph = build(nodes, references, model, generous_budgets());
    if reference_cyclic(model) {
        assert_eq!(
            graph,
            Err(SourceGraphError::Cycle),
            "a cyclic graph must be rejected at construction: {model:?}"
        );
        return false;
    }
    let graph = graph.expect("an acyclic bounded model graph closes");

    let closure = source_root_closure(&graph).expect("an acyclic model graph is bounded");
    assert_eq!(
        indices(closure.order(), references),
        reference_traverse(model),
        "traversal order disagrees for {model:?}"
    );
    assert_eq!(
        usize::try_from(closure.visited()).expect("bounded visited count"),
        reference_reachable(model).len(),
        "visited count disagrees for {model:?}"
    );
    assert_eq!(
        closure.depth(),
        reference_depth(model),
        "depth disagrees for {model:?}"
    );
    true
}

/// Proves that every reduced budget rejects the same bounded model graph.
fn check_budgets(nodes: &[SourceNode], references: &[SourceNodeRef], model: &Model) {
    let graph =
        build(nodes, references, model, generous_budgets()).expect("a bounded model graph closes");
    let closure = source_root_closure(&graph).expect("an acyclic model graph is bounded");
    let node_count = u32::try_from(model.node_count).expect("bounded node count");
    let edge_count = u32::try_from(model.edges.len()).expect("bounded edge count");
    let visited = closure.visited();
    let depth = closure.depth();
    if node_count > 1 {
        assert_eq!(
            build(
                nodes,
                references,
                model,
                model_budgets(node_count - 1, edge_count, depth, visited),
            ),
            Err(SourceGraphError::NodeBudget),
            "the node budget must fail closed for {model:?}"
        );
    }
    if edge_count > 1 {
        assert_eq!(
            build(
                nodes,
                references,
                model,
                model_budgets(node_count, edge_count - 1, depth, visited),
            ),
            Err(SourceGraphError::EdgeBudget),
            "the edge budget must fail closed for {model:?}"
        );
    }
    if visited > 1 {
        let narrow = build(
            nodes,
            references,
            model,
            model_budgets(node_count, edge_count, depth, visited - 1),
        )
        .expect("a visited budget binds traversal");
        assert_eq!(
            source_root_closure(&narrow),
            Err(SourceGraphError::VisitedBudget),
            "the visited budget must fail closed for {model:?}"
        );
    }
    if depth > 1 {
        let shallow = build(
            nodes,
            references,
            model,
            model_budgets(node_count, edge_count, depth - 1, visited),
        )
        .expect("a depth budget binds traversal");
        assert_eq!(
            source_root_closure(&shallow),
            Err(SourceGraphError::DepthBudget),
            "the depth budget must fail closed for {model:?}"
        );
    }
}

/// Compares production with the independent model over every directed graph and
/// every current-root set on up to three nodes.
#[test]
fn exhaustive_bounded_model_agrees_for_every_graph_up_to_three_nodes() {
    let mut cases = 0_u64;
    for node_count in 0_usize..=3 {
        let nodes = model_nodes(node_count);
        let references = model_references(&nodes);
        let edge_slots = node_count * node_count;
        for edge_mask in 0_u32..(1_u32 << edge_slots) {
            let edges = (0..edge_slots)
                .filter(|slot| edge_mask & (1 << slot) != 0)
                .map(|slot| (slot / node_count, slot % node_count))
                .collect::<Vec<_>>();
            for root_mask in 0_u32..(1_u32 << node_count) {
                cases += 1;
                let model = Model {
                    node_count,
                    edges: edges.clone(),
                    roots: (0..node_count)
                        .filter(|node| root_mask & (1 << node) != 0)
                        .collect(),
                };
                if check_agreement(&nodes, &references, &model) {
                    check_budgets(&nodes, &references, &model);
                }
            }
        }
    }
    assert_eq!(cases, 1 + 4 + 64 + 4_096);
}

/// Compares production with the independent model over every four-node edge set
/// with the complete current-root set.
#[test]
fn exhaustive_bounded_model_agrees_for_every_four_node_edge_set() {
    let node_count = 4_usize;
    let nodes = model_nodes(node_count);
    let references = model_references(&nodes);
    let mut cases = 0_u64;
    let mut acyclic = 0_u64;
    for edge_mask in 0_u32..(1_u32 << 16) {
        cases += 1;
        if check_agreement(
            &nodes,
            &references,
            &Model {
                node_count,
                edges: (0..16)
                    .filter(|slot| edge_mask & (1 << slot) != 0)
                    .map(|slot| (slot / node_count, slot % node_count))
                    .collect(),
                roots: (0..node_count).collect(),
            },
        ) {
            acyclic += 1;
        }
    }
    assert_eq!(cases, 65_536);
    assert!(acyclic > 0 && acyclic < cases);
}
