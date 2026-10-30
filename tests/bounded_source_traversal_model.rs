#![doc = "Bounded source-graph traversal kernel, seeded properties, and an exhaustive bounded model."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use std::collections::BTreeSet;

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

/// The `AeneasVerif` comparison subset, checked here against the same model.
///
/// The subset passes structs by value because Charon and Aeneas translate owned
/// values more directly than borrows, so the borrow lint is relaxed for it here.
#[allow(clippy::needless_pass_by_value)]
#[path = "../proofs/spikes/aeneas/rust/bounded_traversal.rs"]
mod aeneas_subset;

/// Widest node count the bitmask reference model represents.
const REFERENCE_CAPACITY: usize = 8;

/// Positive resource limits that bound one source-graph traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Budgets {
    nodes: u32,
    edges: u32,
    depth: u32,
    visited: u32,
}

impl Budgets {
    fn unbounded() -> Self {
        Self {
            nodes: u32::MAX,
            edges: u32::MAX,
            depth: u32::MAX,
            visited: u32::MAX,
        }
    }
}

/// Deterministic fail-closed traversal rejections, in kernel decision order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TraversalError {
    DanglingEdge,
    NodeBudget,
    EdgeBudget,
    Cycle,
    VisitedBudget,
    DepthBudget,
}

/// One directed source graph over dense node indices.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Graph {
    node_count: usize,
    edges: Vec<(usize, usize)>,
    roots: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Candidate production kernel: worklist closure, Kahn acyclicity, bounded DFS.
// ---------------------------------------------------------------------------

/// Canonical child order for a source node: ascending target, duplicates merged.
fn kernel_children(graph: &Graph, from: usize) -> Vec<usize> {
    let mut targets = graph
        .edges
        .iter()
        .filter(|(source, _)| *source == from)
        .map(|(_, target)| *target)
        .collect::<Vec<_>>();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn kernel_roots(graph: &Graph) -> Vec<usize> {
    let mut roots = graph.roots.clone();
    roots.sort_unstable();
    roots.dedup();
    roots
}

fn kernel_closed(graph: &Graph) -> Result<(), TraversalError> {
    let out_of_range = graph
        .edges
        .iter()
        .flat_map(|(source, target)| [*source, *target])
        .chain(graph.roots.iter().copied())
        .any(|node| node >= graph.node_count);
    if out_of_range {
        return Err(TraversalError::DanglingEdge);
    }
    Ok(())
}

fn kernel_reachable(graph: &Graph) -> BTreeSet<usize> {
    let mut reached = BTreeSet::new();
    let mut pending = kernel_roots(graph);
    while let Some(node) = pending.pop() {
        if !reached.insert(node) {
            continue;
        }
        pending.extend(kernel_children(graph, node));
    }
    reached
}

fn kernel_acyclic(graph: &Graph) -> bool {
    let mut remaining = (0..graph.node_count)
        .map(|node| {
            graph
                .edges
                .iter()
                .filter(|(_, target)| *target == node)
                .map(|(source, _)| *source)
                .collect::<BTreeSet<_>>()
                .len()
        })
        .collect::<Vec<_>>();
    let mut ready = (0..graph.node_count)
        .filter(|node| remaining[*node] == 0)
        .collect::<Vec<_>>();
    let mut removed = 0_usize;
    while let Some(node) = ready.pop() {
        removed += 1;
        for child in kernel_children(graph, node) {
            remaining[child] -= 1;
            if remaining[child] == 0 {
                ready.push(child);
            }
        }
    }
    removed == graph.node_count
}

/// Longest reachable path length in edges.
///
/// Each round settles at least one node when the reachable subgraph is acyclic,
/// so input that has not settled after `node_count` rounds contains a reachable
/// cycle. The round cap keeps the kernel total: it never diverges on a cycle.
fn kernel_depth(graph: &Graph) -> Result<u32, TraversalError> {
    let reachable = kernel_reachable(graph);
    let mut longest = vec![0_u32; graph.node_count];
    let mut settled = BTreeSet::new();
    for _ in 0..graph.node_count {
        if settled.len() == reachable.len() {
            break;
        }
        for node in &reachable {
            if settled.contains(node) {
                continue;
            }
            let parents = reachable
                .iter()
                .filter(|parent| kernel_children(graph, **parent).contains(node))
                .copied()
                .collect::<Vec<_>>();
            if parents.iter().all(|parent| settled.contains(parent)) {
                longest[*node] = parents
                    .iter()
                    .map(|parent| longest[*parent] + 1)
                    .max()
                    .unwrap_or(0);
                settled.insert(*node);
            }
        }
    }
    if settled.len() != reachable.len() {
        return Err(TraversalError::Cycle);
    }
    Ok(reachable
        .iter()
        .map(|node| longest[*node])
        .max()
        .unwrap_or(0))
}

/// Bounded canonical traversal. Every rejection is total: no partial output.
fn kernel_traverse(graph: &Graph, budgets: Budgets) -> Result<Vec<usize>, TraversalError> {
    kernel_closed(graph)?;
    let node_count = u32::try_from(graph.node_count).map_err(|_| TraversalError::NodeBudget)?;
    if node_count > budgets.nodes {
        return Err(TraversalError::NodeBudget);
    }
    let edge_count = u32::try_from(graph.edges.len()).map_err(|_| TraversalError::EdgeBudget)?;
    if edge_count > budgets.edges {
        return Err(TraversalError::EdgeBudget);
    }
    if !kernel_acyclic(graph) {
        return Err(TraversalError::Cycle);
    }
    let reachable = kernel_reachable(graph);
    let visited_count =
        u32::try_from(reachable.len()).map_err(|_| TraversalError::VisitedBudget)?;
    if visited_count > budgets.visited {
        return Err(TraversalError::VisitedBudget);
    }
    if kernel_depth(graph)? > budgets.depth {
        return Err(TraversalError::DepthBudget);
    }

    let mut visited = BTreeSet::new();
    let mut output = Vec::new();
    let mut pending = kernel_roots(graph);
    pending.reverse();
    while let Some(node) = pending.pop() {
        if !visited.insert(node) {
            continue;
        }
        output.push(node);
        let mut children = kernel_children(graph, node);
        children.reverse();
        pending.extend(children);
    }
    Ok(output)
}

// ---------------------------------------------------------------------------
// Independent reference model: dense bitmask adjacency and transitive closure.
// ---------------------------------------------------------------------------

fn reference_matrix(graph: &Graph) -> [u16; REFERENCE_CAPACITY] {
    let mut rows = [0_u16; REFERENCE_CAPACITY];
    for (source, target) in &graph.edges {
        rows[*source] |= 1 << *target;
    }
    rows
}

fn reference_transitive_closure(graph: &Graph) -> [u16; REFERENCE_CAPACITY] {
    let mut rows = reference_matrix(graph);
    for pivot in 0..graph.node_count {
        let pivot_row = rows[pivot];
        for row in rows.iter_mut().take(graph.node_count) {
            if *row & (1 << pivot) != 0 {
                *row |= pivot_row;
            }
        }
    }
    rows
}

fn reference_cyclic(graph: &Graph) -> bool {
    let closure = reference_transitive_closure(graph);
    (0..graph.node_count).any(|node| closure[node] & (1 << node) != 0)
}

/// True when some node reachable from a current root lies on a cycle.
fn reference_reachable_cyclic(graph: &Graph) -> bool {
    let closure = reference_transitive_closure(graph);
    reference_reachable(graph)
        .iter()
        .any(|node| closure[*node] & (1 << *node) != 0)
}

fn reference_reachable(graph: &Graph) -> BTreeSet<usize> {
    let closure = reference_transitive_closure(graph);
    let mut reached = BTreeSet::new();
    for root in &graph.roots {
        reached.insert(*root);
        for node in 0..graph.node_count {
            if closure[*root] & (1 << node) != 0 {
                reached.insert(node);
            }
        }
    }
    reached
}

/// Longest path in edges from any root, by explicit relaxation over the reachable set.
fn reference_depth(graph: &Graph) -> u32 {
    let matrix = reference_matrix(graph);
    let reachable = reference_reachable(graph);
    let mut longest = vec![0_u32; graph.node_count];
    for _ in 0..graph.node_count.max(1) {
        for parent in reachable.iter().copied() {
            for child in 0..graph.node_count {
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
fn reference_traverse(graph: &Graph) -> Vec<usize> {
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

    let matrix = reference_matrix(graph);
    let mut roots = graph.roots.clone();
    roots.sort_unstable();
    roots.dedup();
    let mut visited = Vec::new();
    for root in roots {
        visit(&matrix, graph.node_count, root, &mut visited);
    }
    visited
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn exact_budgets(graph: &Graph) -> Budgets {
    Budgets {
        nodes: u32::try_from(graph.node_count).expect("bounded node count"),
        edges: u32::try_from(graph.edges.len()).expect("bounded edge count"),
        depth: kernel_depth(graph).expect("an acyclic graph has a defined depth"),
        visited: u32::try_from(kernel_reachable(graph).len()).expect("bounded visited count"),
    }
}

fn draw_graph(test_case: &TestCase, capacity: usize) -> Graph {
    let node_count = test_case.draw(gs::integers::<usize>().min_value(1).max_value(capacity));
    let edges = test_case.draw(
        gs::vecs(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(capacity * capacity - 1),
        )
        .max_size(2 * capacity),
    );
    let edges = edges
        .into_iter()
        .map(|packed| (packed / capacity % node_count, packed % node_count))
        .collect();
    let roots = test_case.draw(
        gs::vecs(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(node_count - 1),
        )
        .min_size(1)
        .max_size(node_count),
    );
    Graph {
        node_count,
        edges,
        roots,
    }
}

/// Reorders edges without changing the edge set, using drawn sort keys.
fn permute_edges(graph: &Graph, keys: &[u8]) -> Graph {
    let mut ordered = graph
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| (keys.get(index).copied().unwrap_or(0), index, *edge))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(key, index, _)| (*key, *index));
    Graph {
        node_count: graph.node_count,
        edges: ordered.into_iter().map(|(_, _, edge)| edge).collect(),
        roots: graph.roots.clone(),
    }
}

// ---------------------------------------------------------------------------
// Seeded properties
// ---------------------------------------------------------------------------

#[test]
fn hegel_traversal_output_is_edge_and_root_order_independent() {
    Hegel::new(|test_case: TestCase| {
        let graph = draw_graph(&test_case, REFERENCE_CAPACITY);
        let keys = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(2 * REFERENCE_CAPACITY));
        let permuted = permute_edges(&graph, &keys);

        assert_eq!(kernel_reachable(&graph), kernel_reachable(&permuted));
        assert_eq!(kernel_acyclic(&graph), kernel_acyclic(&permuted));
        assert_eq!(
            kernel_traverse(&graph, Budgets::unbounded()),
            kernel_traverse(&permuted, Budgets::unbounded())
        );

        let mut duplicated = graph.clone();
        duplicated.edges.extend(graph.edges.iter().copied());
        duplicated.roots.extend(graph.roots.iter().copied());
        assert_eq!(kernel_reachable(&graph), kernel_reachable(&duplicated));
        assert_eq!(
            kernel_traverse(&graph, Budgets::unbounded()),
            kernel_traverse(&duplicated, Budgets::unbounded())
        );

        let mut reversed_roots = graph.clone();
        reversed_roots.roots.reverse();
        assert_eq!(
            kernel_traverse(&graph, Budgets::unbounded()),
            kernel_traverse(&reversed_roots, Budgets::unbounded())
        );
    })
    .settings(settings(401))
    .run();
}

#[test]
fn hegel_reachability_is_the_least_fixed_point_over_current_roots() {
    Hegel::new(|test_case: TestCase| {
        let graph = draw_graph(&test_case, REFERENCE_CAPACITY);
        let reachable = kernel_reachable(&graph);

        assert_eq!(reachable, reference_reachable(&graph));
        for root in &graph.roots {
            assert!(reachable.contains(root));
        }

        let stepped = reachable
            .iter()
            .flat_map(|node| kernel_children(&graph, *node))
            .chain(reachable.iter().copied())
            .collect::<BTreeSet<_>>();
        assert_eq!(stepped, reachable, "one more step must add nothing");

        let mut iterated = graph.roots.iter().copied().collect::<BTreeSet<_>>();
        for _ in 0..graph.node_count {
            iterated = iterated
                .iter()
                .flat_map(|node| kernel_children(&graph, *node))
                .chain(iterated.iter().copied())
                .collect();
        }
        assert_eq!(iterated, reachable, "the fixed point converges in n steps");

        let mut narrowed = graph.clone();
        narrowed.roots.truncate(1);
        assert!(kernel_reachable(&narrowed).is_subset(&reachable));
    })
    .settings(settings(402))
    .run();
}

#[test]
fn hegel_cycles_are_rejected_and_never_traversed() {
    Hegel::new(|test_case: TestCase| {
        let graph = draw_graph(&test_case, REFERENCE_CAPACITY);
        assert_eq!(kernel_acyclic(&graph), !reference_cyclic(&graph));

        if kernel_acyclic(&graph) {
            assert!(kernel_traverse(&graph, Budgets::unbounded()).is_ok());
            assert!(kernel_depth(&graph).is_ok());
        } else {
            assert_eq!(
                kernel_traverse(&graph, Budgets::unbounded()),
                Err(TraversalError::Cycle)
            );
        }
        // The depth round cap must terminate on a cyclic reachable subgraph
        // rather than diverge, and must agree with the independent model.
        assert_eq!(
            kernel_depth(&graph).is_ok(),
            !reference_reachable_cyclic(&graph)
        );

        let node = test_case.draw(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(graph.node_count - 1),
        );
        let mut self_loop = graph.clone();
        self_loop.edges.push((node, node));
        assert!(!kernel_acyclic(&self_loop));
        assert_eq!(
            kernel_traverse(&self_loop, Budgets::unbounded()),
            Err(TraversalError::Cycle)
        );

        let closure = reference_transitive_closure(&graph);
        let ancestor = test_case.draw(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(graph.node_count - 1),
        );
        let descendant = test_case.draw(
            gs::integers::<usize>()
                .min_value(0)
                .max_value(graph.node_count - 1),
        );
        if closure[ancestor] & (1 << descendant) != 0 {
            let mut closed = graph.clone();
            closed.edges.push((descendant, ancestor));
            assert!(
                !kernel_acyclic(&closed),
                "an edge back to an ancestor must close a cycle"
            );
        }
    })
    .settings(settings(403))
    .run();
}

#[test]
fn hegel_traversal_respects_every_budget_and_fails_closed() {
    Hegel::new(|test_case: TestCase| {
        let graph = draw_graph(&test_case, REFERENCE_CAPACITY);
        if !kernel_acyclic(&graph) {
            return;
        }
        let reachable = kernel_reachable(&graph);
        let exact = exact_budgets(&graph);

        let visited = kernel_traverse(&graph, exact).expect("exact budgets admit the traversal");
        assert_eq!(visited.iter().copied().collect::<BTreeSet<_>>(), reachable);
        assert_eq!(visited.len(), reachable.len(), "no node is visited twice");
        assert!(visited.len() <= graph.node_count);
        assert_eq!(visited, reference_traverse(&graph));
        assert_eq!(exact.depth, reference_depth(&graph));

        for (reduced, expected) in [
            (
                Budgets {
                    nodes: exact.nodes - 1,
                    ..exact
                },
                TraversalError::NodeBudget,
            ),
            (
                Budgets {
                    edges: exact.edges.saturating_sub(1),
                    ..exact
                },
                TraversalError::EdgeBudget,
            ),
            (
                Budgets {
                    visited: exact.visited - 1,
                    ..exact
                },
                TraversalError::VisitedBudget,
            ),
            (
                Budgets {
                    depth: exact.depth.saturating_sub(1),
                    ..exact
                },
                TraversalError::DepthBudget,
            ),
        ] {
            if reduced == exact {
                continue;
            }
            assert_eq!(kernel_traverse(&graph, reduced), Err(expected));
        }

        let mut dangling = graph.clone();
        dangling.edges.push((0, graph.node_count));
        assert_eq!(
            kernel_traverse(&dangling, Budgets::unbounded()),
            Err(TraversalError::DanglingEdge)
        );
    })
    .settings(settings(404))
    .run();
}

// ---------------------------------------------------------------------------
// Exhaustive bounded model
// ---------------------------------------------------------------------------

/// Compares the kernel with the independent model over every directed graph and
/// every current-root set on up to four nodes.
#[test]
fn exhaustive_bounded_model_agrees_for_every_graph_up_to_four_nodes() {
    let mut graphs = 0_u64;
    let mut cases = 0_u64;
    let mut acyclic_cases = 0_u64;
    for node_count in 0_usize..=4 {
        let edge_slots = node_count * node_count;
        for edge_mask in 0_u32..(1_u32 << edge_slots) {
            graphs += 1;
            let edges = (0..edge_slots)
                .filter(|slot| edge_mask & (1 << slot) != 0)
                .map(|slot| (slot / node_count, slot % node_count))
                .collect::<Vec<_>>();
            for root_mask in 0_u32..(1_u32 << node_count) {
                cases += 1;
                let graph = Graph {
                    node_count,
                    edges: edges.clone(),
                    roots: (0..node_count)
                        .filter(|node| root_mask & (1 << node) != 0)
                        .collect(),
                };

                assert_eq!(
                    kernel_acyclic(&graph),
                    !reference_cyclic(&graph),
                    "acyclicity disagrees for {graph:?}"
                );
                assert_eq!(
                    kernel_reachable(&graph),
                    reference_reachable(&graph),
                    "reachability disagrees for {graph:?}"
                );
                assert_eq!(kernel_closed(&graph), Ok(()));

                if !kernel_acyclic(&graph) {
                    assert_eq!(
                        kernel_traverse(&graph, Budgets::unbounded()),
                        Err(TraversalError::Cycle),
                        "a cyclic graph must be rejected: {graph:?}"
                    );
                    continue;
                }
                acyclic_cases += 1;

                assert_eq!(
                    kernel_depth(&graph),
                    Ok(reference_depth(&graph)),
                    "depth disagrees for {graph:?}"
                );
                let exact = exact_budgets(&graph);
                let visited = kernel_traverse(&graph, exact)
                    .expect("exact budgets admit an acyclic traversal");
                assert_eq!(
                    visited,
                    reference_traverse(&graph),
                    "traversal order disagrees for {graph:?}"
                );
                // Slot enumeration presents edges already sorted, so the
                // reversed presentation is what exercises order independence.
                let reversed = Graph {
                    node_count,
                    edges: graph.edges.iter().rev().copied().collect(),
                    roots: graph.roots.iter().rev().copied().collect(),
                };
                assert_eq!(
                    kernel_traverse(&reversed, exact),
                    Ok(visited.clone()),
                    "edge presentation order changed the traversal for {graph:?}"
                );
                assert_eq!(
                    visited.iter().copied().collect::<BTreeSet<_>>(),
                    kernel_reachable(&graph)
                );

                for (reduced, expected) in [
                    (
                        Budgets {
                            nodes: exact.nodes.saturating_sub(1),
                            ..exact
                        },
                        TraversalError::NodeBudget,
                    ),
                    (
                        Budgets {
                            edges: exact.edges.saturating_sub(1),
                            ..exact
                        },
                        TraversalError::EdgeBudget,
                    ),
                    (
                        Budgets {
                            visited: exact.visited.saturating_sub(1),
                            ..exact
                        },
                        TraversalError::VisitedBudget,
                    ),
                    (
                        Budgets {
                            depth: exact.depth.saturating_sub(1),
                            ..exact
                        },
                        TraversalError::DepthBudget,
                    ),
                ] {
                    if reduced == exact {
                        continue;
                    }
                    assert_eq!(
                        kernel_traverse(&graph, reduced),
                        Err(expected),
                        "budget reduction must fail closed for {graph:?}"
                    );
                }
            }
        }
    }

    assert_eq!(graphs, 1 + 2 + 16 + 512 + 65_536);
    assert_eq!(cases, 1 + 4 + 64 + 4_096 + 1_048_576);
    assert!(acyclic_cases > 0 && acyclic_cases < cases);
}

/// Binds the `AeneasVerif` comparison subset to the exhaustive four-node model.
///
/// The subset unrolls the fixed point to the node bound instead of iterating a
/// worklist, so this check is what makes the unrolling honest.
#[test]
fn aeneas_comparison_subset_agrees_with_the_exhaustive_four_node_model() {
    let node_count = 4_usize;
    let mut checked = 0_u64;
    for edge_mask in 0_u32..(1_u32 << 16) {
        let edges = (0..16)
            .filter(|slot| edge_mask & (1 << slot) != 0)
            .map(|slot| (slot / node_count, slot % node_count))
            .collect::<Vec<_>>();
        let rows = [0_usize, 1, 2, 3].map(|node| {
            u8::try_from(
                (0..node_count)
                    .filter(|child| edges.contains(&(node, *child)))
                    .fold(0_usize, |mask, child| mask | (1 << child)),
            )
            .expect("a four-node row fits one nibble")
        });
        for root_mask in 0_u8..16 {
            checked += 1;
            let graph = Graph {
                node_count,
                edges: edges.clone(),
                roots: (0..node_count)
                    .filter(|node| root_mask & (1 << node) != 0)
                    .collect(),
            };

            let reached =
                aeneas_subset::reachable_closure(rows[0], rows[1], rows[2], rows[3], root_mask);
            let expected = kernel_reachable(&graph)
                .iter()
                .fold(0_u8, |mask, node| mask | (1 << node));
            assert_eq!(reached, expected, "closure disagrees for {graph:?}");
            assert!(
                aeneas_subset::is_fixed_point(rows[0], rows[1], rows[2], rows[3], reached),
                "the unrolled closure must be a fixed point for {graph:?}"
            );
            assert_eq!(
                u64::from(aeneas_subset::visited_count(reached)),
                kernel_reachable(&graph).len() as u64
            );
            assert_eq!(
                aeneas_subset::reachable_cycle(rows[0], rows[1], rows[2], rows[3], root_mask),
                reference_reachable_cyclic(&graph),
                "cycle verdict disagrees for {graph:?}"
            );
            assert_eq!(
                aeneas_subset::has_self_loop(rows[0], rows[1], rows[2], rows[3]),
                (0..node_count).any(|node| edges.contains(&(node, node))),
                "self-loop verdict disagrees for {graph:?}"
            );

            let Ok(exact) = kernel_depth(&graph) else {
                continue;
            };
            let budgets = || aeneas_subset::Budgets {
                nodes: u32::try_from(node_count).expect("four nodes"),
                edges: u32::try_from(edges.len()).expect("bounded edge count"),
                depth: exact,
                visited: aeneas_subset::visited_count(reached),
            };
            let measures = |depth| aeneas_subset::Measures {
                nodes: u32::try_from(node_count).expect("four nodes"),
                edges: u32::try_from(edges.len()).expect("bounded edge count"),
                depth,
                visited: aeneas_subset::visited_count(reached),
            };
            assert!(aeneas_subset::within_budgets(measures(exact), budgets()));
            assert!(!aeneas_subset::within_budgets(
                measures(exact + 1),
                budgets()
            ));
        }
    }
    assert_eq!(checked, 1_048_576);
}
