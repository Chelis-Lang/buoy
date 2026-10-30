use vstd::prelude::*;
use vstd::set_lib::lemma_len_subset;

verus! {

/// One reachability step over a current-root set and an edge set.
///
/// The pinned `vstd` reserves `Set` for finite sets, so the closure lemmas use
/// `ISet` and the cardinality bound uses the finite `Set`.
spec fn reachable_step(nodes: ISet<nat>, edges: ISet<(nat, nat)>) -> ISet<nat> {
    ISet::new(
        |child: nat|
            nodes.contains(child) || exists|parent: nat|
                #![trigger edges.contains((parent, child))]
                nodes.contains(parent) && edges.contains((parent, child)),
    )
}

/// A traversal is acyclic when no reachable node reaches itself.
spec fn acyclic(nodes: ISet<nat>, reaches: ISet<(nat, nat)>) -> bool {
    forall|node: nat|
        #![trigger reaches.contains((node, node))]
        nodes.contains(node) ==> !reaches.contains((node, node))
}

spec fn transitive(reaches: ISet<(nat, nat)>) -> bool {
    forall|first: nat, second: nat, third: nat|
        #![trigger reaches.contains((first, second)), reaches.contains((second, third))]
        reaches.contains((first, second)) && reaches.contains((second, third))
            ==> reaches.contains((first, third))
}

/// Every resource limit that one bounded traversal must respect.
spec fn within_budgets(
    node_count: nat,
    edge_count: nat,
    depth: nat,
    visited: nat,
    node_budget: nat,
    edge_budget: nat,
    depth_budget: nat,
    visited_budget: nat,
) -> bool {
    node_count <= node_budget && edge_count <= edge_budget && depth <= depth_budget && visited
        <= visited_budget
}

// -- Edge-order independence ------------------------------------------------

/// The step result depends only on edge membership, never on presentation order.
proof fn step_depends_only_on_edge_membership(
    nodes: ISet<nat>,
    left: ISet<(nat, nat)>,
    right: ISet<(nat, nat)>,
)
    requires
        forall|edge: (nat, nat)| left.contains(edge) == right.contains(edge),
    ensures
        reachable_step(nodes, left) =~= reachable_step(nodes, right),
{
}

// -- Fixed-point reachability -----------------------------------------------

proof fn step_is_extensive(nodes: ISet<nat>, edges: ISet<(nat, nat)>)
    ensures
        nodes.subset_of(reachable_step(nodes, edges)),
{
}

proof fn step_is_monotone(small: ISet<nat>, large: ISet<nat>, edges: ISet<(nat, nat)>)
    requires
        small.subset_of(large),
    ensures
        reachable_step(small, edges).subset_of(reachable_step(large, edges)),
{
}

/// A fixed point of the step function is closed under every outgoing edge.
proof fn fixed_point_is_closed_under_edges(
    reached: ISet<nat>,
    edges: ISet<(nat, nat)>,
    parent: nat,
    child: nat,
)
    requires
        reachable_step(reached, edges) =~= reached,
        reached.contains(parent),
        edges.contains((parent, child)),
    ensures
        reached.contains(child),
{
    assert(reachable_step(reached, edges).contains(child));
}

// -- Cycle rejection --------------------------------------------------------

proof fn a_self_loop_is_rejected(nodes: ISet<nat>, reaches: ISet<(nat, nat)>, node: nat)
    requires
        nodes.contains(node),
        reaches.contains((node, node)),
    ensures
        !acyclic(nodes, reaches),
{
}

/// An edge back to an ancestor closes a cycle under a transitive reach relation.
proof fn a_back_edge_to_an_ancestor_is_rejected(
    nodes: ISet<nat>,
    reaches: ISet<(nat, nat)>,
    ancestor: nat,
    descendant: nat,
)
    requires
        transitive(reaches),
        nodes.contains(ancestor),
        reaches.contains((ancestor, descendant)),
        reaches.contains((descendant, ancestor)),
    ensures
        !acyclic(nodes, reaches),
{
    assert(reaches.contains((ancestor, ancestor)));
}

// -- Traversal bounds -------------------------------------------------------

/// A traversal never visits more nodes than the source graph contains.
proof fn visited_is_bounded_by_the_node_set(visited: Set<nat>, nodes: Set<nat>)
    requires
        visited.subset_of(nodes),
    ensures
        visited.len() <= nodes.len(),
{
    lemma_len_subset(visited, nodes);
}

proof fn an_admitted_traversal_respects_every_budget(
    node_count: nat,
    edge_count: nat,
    depth: nat,
    visited: nat,
    node_budget: nat,
    edge_budget: nat,
    depth_budget: nat,
    visited_budget: nat,
)
    requires
        within_budgets(
            node_count,
            edge_count,
            depth,
            visited,
            node_budget,
            edge_budget,
            depth_budget,
            visited_budget,
        ),
    ensures
        node_count <= node_budget,
        edge_count <= edge_budget,
        depth <= depth_budget,
        visited <= visited_budget,
{
}

/// Exceeding any single budget rejects the traversal; no budget is optional.
proof fn exceeding_any_budget_is_rejected(
    node_count: nat,
    edge_count: nat,
    depth: nat,
    visited: nat,
    node_budget: nat,
    edge_budget: nat,
    depth_budget: nat,
    visited_budget: nat,
)
    requires
        node_count > node_budget || edge_count > edge_budget || depth > depth_budget || visited
            > visited_budget,
    ensures
        !within_budgets(
            node_count,
            edge_count,
            depth,
            visited,
            node_budget,
            edge_budget,
            depth_budget,
            visited_budget,
        ),
{
}

} // verus!

fn main() {}
