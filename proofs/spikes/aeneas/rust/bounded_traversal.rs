#![forbid(unsafe_code)]

//! Safe sequential comparison subset for the bounded source-traversal kernel.
//!
//! A four-node source graph is one bitmask row per node. The reachability
//! fixed point is unrolled to the node bound, so this subset contains no loop,
//! no allocation, no iterator, and no collection.

/// Successors of every node currently in the frontier mask.
pub fn successors(from0: u8, from1: u8, from2: u8, from3: u8, frontier: u8) -> u8 {
    let mut reached = 0u8;
    if frontier & 1 != 0 {
        reached |= from0;
    }
    if frontier & 2 != 0 {
        reached |= from1;
    }
    if frontier & 4 != 0 {
        reached |= from2;
    }
    if frontier & 8 != 0 {
        reached |= from3;
    }
    reached
}

/// One reachability step. The step is extensive: it never drops a frontier node.
pub fn reachable_step(from0: u8, from1: u8, from2: u8, from3: u8, frontier: u8) -> u8 {
    frontier | successors(from0, from1, from2, from3, frontier)
}

/// The reachability fixed point, unrolled to the four-node bound.
pub fn reachable_closure(from0: u8, from1: u8, from2: u8, from3: u8, roots: u8) -> u8 {
    let first = reachable_step(from0, from1, from2, from3, roots);
    let second = reachable_step(from0, from1, from2, from3, first);
    let third = reachable_step(from0, from1, from2, from3, second);
    reachable_step(from0, from1, from2, from3, third)
}

/// True when one more step adds nothing, which is the fixed-point condition.
pub fn is_fixed_point(from0: u8, from1: u8, from2: u8, from3: u8, reached: u8) -> bool {
    reachable_step(from0, from1, from2, from3, reached) == reached
}

/// A node on a self loop is the smallest rejected cycle.
pub fn has_self_loop(from0: u8, from1: u8, from2: u8, from3: u8) -> bool {
    from0 & 1 != 0 || from1 & 2 != 0 || from2 & 4 != 0 || from3 & 8 != 0
}

/// Nodes reachable from one node in at least one edge.
pub fn strict_closure(from0: u8, from1: u8, from2: u8, from3: u8, node: u8) -> u8 {
    let first = successors(from0, from1, from2, from3, node);
    reachable_closure(from0, from1, from2, from3, first)
}

/// True when a node reachable from the current roots reaches itself.
pub fn reachable_cycle(from0: u8, from1: u8, from2: u8, from3: u8, roots: u8) -> bool {
    let reached = reachable_closure(from0, from1, from2, from3, roots);
    (reached & 1 != 0 && strict_closure(from0, from1, from2, from3, 1) & 1 != 0)
        || (reached & 2 != 0 && strict_closure(from0, from1, from2, from3, 2) & 2 != 0)
        || (reached & 4 != 0 && strict_closure(from0, from1, from2, from3, 4) & 4 != 0)
        || (reached & 8 != 0 && strict_closure(from0, from1, from2, from3, 8) & 8 != 0)
}

/// Visited-node count for a four-node mask.
pub fn visited_count(mask: u8) -> u32 {
    let mut count = 0u32;
    if mask & 1 != 0 {
        count += 1;
    }
    if mask & 2 != 0 {
        count += 1;
    }
    if mask & 4 != 0 {
        count += 1;
    }
    if mask & 8 != 0 {
        count += 1;
    }
    count
}

/// Measured cost of one traversal.
pub struct Measures {
    pub nodes: u32,
    pub edges: u32,
    pub depth: u32,
    pub visited: u32,
}

/// Positive resource limits for one traversal.
pub struct Budgets {
    pub nodes: u32,
    pub edges: u32,
    pub depth: u32,
    pub visited: u32,
}

/// Every resource limit that one bounded traversal must respect.
pub fn within_budgets(measures: Measures, budgets: Budgets) -> bool {
    measures.nodes <= budgets.nodes
        && measures.edges <= budgets.edges
        && measures.depth <= budgets.depth
        && measures.visited <= budgets.visited
}
