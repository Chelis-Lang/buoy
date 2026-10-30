#![doc = "Behavioral contracts for content-addressed source nodes, edges, and bounded traversal."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::{
    ActiveAlgorithm, DigestAlgorithm, DigestTextError, ParsedItemIdentity, RepositoryPath,
    SchemaId, SourceBudgets, SourceChildRef, SourceDepthBudget, SourceEdge, SourceEdgeBudget,
    SourceEdgeKind, SourceGraph, SourceGraphError, SourceItemKind, SourceNode, SourceNodeBudget,
    SourceNodeRef, SourceVisitedBudget, require_current_source_reference, source_root_closure,
    source_traversal,
};

fn budgets() -> SourceBudgets {
    exact_budgets(64, 64, 16, 64)
}

fn exact_budgets(nodes: u32, edges: u32, depth: u32, visited: u32) -> SourceBudgets {
    SourceBudgets::new(
        SourceNodeBudget::new(nodes).expect("positive node budget"),
        SourceEdgeBudget::new(edges).expect("positive edge budget"),
        SourceDepthBudget::new(depth).expect("positive depth budget"),
        SourceVisitedBudget::new(visited).expect("positive visited budget"),
    )
}

fn item(path: &str, kind: &str, payload: &[u8]) -> ParsedItemIdentity {
    ParsedItemIdentity::new(
        SchemaId::new("openspec/v1").expect("valid schema"),
        RepositoryPath::new(path).expect("valid path"),
        SourceItemKind::new(kind).expect("valid source item kind"),
        payload.to_vec(),
        None,
    )
    .expect("valid parsed item")
}

fn leaf(path: &str, kind: &str, payload: &[u8]) -> SourceNode {
    SourceNode::new(item(path, kind, payload), Vec::new()).expect("valid leaf node")
}

#[cfg(feature = "coverage-controls")]
#[test]
fn unequal_source_nodes_under_one_digest_fail_closed() {
    let honest = leaf("src/lib.rs", "rust/item/v1", b"honest");
    let forged = honest.digest_collision_control(item("src/lib.rs", "rust/item/v1", b"forged"));
    assert_eq!(
        SourceGraph::new(vec![honest, forged], Vec::new(), Vec::new(), budgets()),
        Err(SourceGraphError::NodeDigestCollision)
    );
}

fn parent(path: &str, kind: &str, payload: &[u8], children: &[&SourceNode]) -> SourceNode {
    let references = children
        .iter()
        .enumerate()
        .map(|(position, child)| {
            SourceChildRef::new(
                SourceEdgeKind::Contains,
                u32::try_from(position).expect("bounded child position"),
                child.reference(),
            )
        })
        .collect();
    SourceNode::new(item(path, kind, payload), references).expect("valid parent node")
}

#[test]
fn source_graph_canonical_byte_accessors_encode_every_registered_source_value() {
    let tree = spec_tree(b"THEN b");
    let child = tree.scenario.children().first().expect("scenario child");
    let edge = SourceEdge::new(
        tree.scenario.reference(),
        SourceEdgeKind::Contains,
        Some(child.position()),
        child.target(),
    )
    .expect("materialized containment edge");
    let graph = SourceGraph::new(
        vec![
            tree.document.clone(),
            tree.requirement.clone(),
            tree.scenario.clone(),
            tree.given.clone(),
            tree.then.clone(),
        ],
        Vec::new(),
        vec![tree.document.reference()],
        budgets(),
    )
    .expect("source graph");

    assert!(!child.canonical_bytes().expect("child bytes").is_empty());
    assert!(
        !tree
            .scenario
            .canonical_bytes()
            .expect("node bytes")
            .is_empty()
    );
    assert!(!edge.canonical_bytes().expect("edge bytes").is_empty());
    assert!(
        !budgets()
            .canonical_bytes()
            .expect("budget bytes")
            .is_empty()
    );
    assert!(!graph.canonical_bytes().expect("graph bytes").is_empty());
}

#[cfg(all(feature = "blake3-256", feature = "sha-256"))]
#[test]
fn explicit_member_constructors_reject_every_foreign_source_reference_position() {
    let active = ActiveAlgorithm::new(DigestAlgorithm::Blake3_256).expect("BLAKE3 member");
    let foreign = ActiveAlgorithm::new(DigestAlgorithm::Sha256).expect("SHA-256 member");
    let foreign_item = ParsedItemIdentity::new_with(
        foreign,
        SchemaId::new("openspec/v1").expect("schema"),
        RepositoryPath::new("openspec/specs/demo/spec.md").expect("path"),
        SourceItemKind::new("openspec/requirement/v1").expect("kind"),
        b"foreign".to_vec(),
        None,
    )
    .expect("foreign item");
    let foreign_node =
        SourceNode::new_with(foreign, foreign_item.clone(), Vec::new()).expect("foreign node");
    assert_eq!(
        SourceNode::new_with(active, foreign_item, Vec::new()),
        Err(SourceGraphError::NodeDigestMember)
    );

    let local_item = ParsedItemIdentity::new_with(
        active,
        SchemaId::new("openspec/v1").expect("schema"),
        RepositoryPath::new("openspec/specs/demo/spec.md").expect("path"),
        SourceItemKind::new("openspec/document/v1").expect("kind"),
        b"local".to_vec(),
        None,
    )
    .expect("local item");
    assert_eq!(
        SourceNode::new_with(
            active,
            local_item.clone(),
            vec![
                SourceChildRef::new(SourceEdgeKind::Contains, 0, foreign_node.reference()),
                SourceChildRef::new(SourceEdgeKind::Contains, 0, foreign_node.reference()),
            ],
        ),
        Err(SourceGraphError::DuplicateChildPosition)
    );
    assert_eq!(
        SourceNode::new_with(
            active,
            local_item,
            vec![SourceChildRef::new(
                SourceEdgeKind::Contains,
                0,
                foreign_node.reference(),
            )],
        ),
        Err(SourceGraphError::NodeDigestMember)
    );

    let local = SourceNode::new_with(
        active,
        ParsedItemIdentity::new_with(
            active,
            SchemaId::new("rust/v1").expect("schema"),
            RepositoryPath::new("src/lib.rs").expect("path"),
            SourceItemKind::new("rust/item/v1").expect("kind"),
            b"local".to_vec(),
            None,
        )
        .expect("local item"),
        Vec::new(),
    )
    .expect("local node");
    let edge = SourceEdge::new(
        local.reference(),
        SourceEdgeKind::Implements,
        None,
        foreign_node.reference(),
    )
    .expect("navigation edge");
    assert_eq!(
        SourceGraph::new_with(
            active,
            vec![local.clone()],
            vec![edge],
            Vec::new(),
            budgets(),
        ),
        Err(SourceGraphError::NodeDigestMember)
    );
    let foreign_source = SourceEdge::new(
        foreign_node.reference(),
        SourceEdgeKind::Implements,
        None,
        local.reference(),
    )
    .expect("foreign-source navigation edge");
    assert_eq!(
        SourceGraph::new_with(
            active,
            vec![local],
            vec![foreign_source],
            Vec::new(),
            budgets(),
        ),
        Err(SourceGraphError::NodeDigestMember)
    );
    assert_eq!(
        SourceGraph::new_with(
            active,
            vec![foreign_node.clone()],
            Vec::new(),
            Vec::new(),
            budgets(),
        ),
        Err(SourceGraphError::NodeDigestMember)
    );
    assert_eq!(
        SourceGraph::new_with(
            active,
            Vec::new(),
            Vec::new(),
            vec![foreign_node.reference()],
            budgets(),
        ),
        Err(SourceGraphError::NodeDigestMember)
    );
}

fn scenario_item() -> ParsedItemIdentity {
    item(
        "openspec/specs/demo/spec.md",
        "openspec/scenario/v1",
        b"positive",
    )
}

/// One document, one requirement, one scenario, and two ordered steps.
struct SpecTree {
    document: SourceNode,
    requirement: SourceNode,
    scenario: SourceNode,
    given: SourceNode,
    then: SourceNode,
}

fn spec_tree(then_payload: &[u8]) -> SpecTree {
    let given = leaf(
        "openspec/specs/demo/spec.md",
        "openspec/step/v1",
        b"GIVEN a",
    );
    let then = leaf(
        "openspec/specs/demo/spec.md",
        "openspec/step/v1",
        then_payload,
    );
    let scenario = parent(
        "openspec/specs/demo/spec.md",
        "openspec/scenario/v1",
        b"positive",
        &[&given, &then],
    );
    let requirement = parent(
        "openspec/specs/demo/spec.md",
        "openspec/requirement/v1",
        b"DEMO-001",
        &[&scenario],
    );
    let document = parent(
        "openspec/specs/demo/spec.md",
        "openspec/document/v1",
        b"demo",
        &[&requirement],
    );
    SpecTree {
        document,
        requirement,
        scenario,
        given,
        then,
    }
}

impl SpecTree {
    fn nodes(&self) -> Vec<SourceNode> {
        vec![
            self.document.clone(),
            self.requirement.clone(),
            self.scenario.clone(),
            self.given.clone(),
            self.then.clone(),
        ]
    }
}

#[test]
fn source_node_digests_bind_every_canonical_field_and_ignore_input_order() {
    let first = spec_tree(b"THEN b");
    let second = spec_tree(b"THEN b");
    assert_eq!(first.document.digest(), second.document.digest());
    assert_eq!(
        first.document.reference(),
        SourceNodeRef::new(first.document.digest())
    );

    let changed_step = spec_tree(b"THEN c");
    assert_ne!(first.then.digest(), changed_step.then.digest());
    assert_ne!(first.scenario.digest(), changed_step.scenario.digest());
    assert_ne!(
        first.requirement.digest(),
        changed_step.requirement.digest()
    );
    assert_ne!(first.document.digest(), changed_step.document.digest());
    assert_eq!(
        first.given.digest(),
        changed_step.given.digest(),
        "an unchanged sibling keeps its identity"
    );

    let reordered = SourceNode::new(
        scenario_item(),
        vec![
            SourceChildRef::new(SourceEdgeKind::Contains, 1, first.then.reference()),
            SourceChildRef::new(SourceEdgeKind::Contains, 0, first.given.reference()),
        ],
    )
    .expect("child input order is canonicalized");
    assert_eq!(reordered.digest(), first.scenario.digest());
    assert_eq!(reordered.item(), &scenario_item());
    assert_eq!(reordered.children().len(), 2);
    assert_eq!(reordered.children()[0].relation(), SourceEdgeKind::Contains);
    assert_eq!(reordered.children()[0].position(), 0);
    assert_eq!(reordered.children()[0].target(), first.given.reference());

    let swapped = SourceNode::new(
        scenario_item(),
        vec![
            SourceChildRef::new(SourceEdgeKind::Contains, 0, first.then.reference()),
            SourceChildRef::new(SourceEdgeKind::Contains, 1, first.given.reference()),
        ],
    )
    .expect("swapped child positions still construct");
    assert_ne!(
        swapped.digest(),
        first.scenario.digest(),
        "child position binds the node digest"
    );
}

#[test]
fn source_node_rejects_duplicate_and_noncontiguous_child_positions() {
    let child = leaf(
        "openspec/specs/demo/spec.md",
        "openspec/step/v1",
        b"GIVEN a",
    );
    let other = leaf("openspec/specs/demo/spec.md", "openspec/step/v1", b"THEN b");

    assert_eq!(
        SourceNode::new(
            scenario_item(),
            vec![
                SourceChildRef::new(SourceEdgeKind::Contains, 0, child.reference()),
                SourceChildRef::new(SourceEdgeKind::Contains, 0, other.reference()),
            ],
        ),
        Err(SourceGraphError::DuplicateChildPosition)
    );
    assert_eq!(
        SourceNode::new(
            scenario_item(),
            vec![
                SourceChildRef::new(SourceEdgeKind::Contains, 0, child.reference()),
                SourceChildRef::new(SourceEdgeKind::Contains, 2, other.reference()),
            ],
        ),
        Err(SourceGraphError::ChildPositionGap)
    );
    assert_eq!(
        SourceNode::new(
            scenario_item(),
            vec![SourceChildRef::new(
                SourceEdgeKind::Contains,
                1,
                child.reference(),
            )],
        ),
        Err(SourceGraphError::ChildPositionGap)
    );
    for relation in [SourceEdgeKind::Implements, SourceEdgeKind::Authority] {
        assert_eq!(
            SourceNode::new(
                scenario_item(),
                vec![
                    SourceChildRef::new(SourceEdgeKind::Contains, 0, child.reference()),
                    SourceChildRef::new(relation, 0, other.reference()),
                ],
            ),
            Err(SourceGraphError::PositionedNavigation),
            "a position orders containment only, so {relation:?} never reaches a preimage"
        );
    }
}

#[test]
fn source_node_containment_is_exactly_the_aggregate_merkle_preimage() {
    let tree = spec_tree(b"THEN b");
    let subject = leaf("src/lib.rs", "rust/item/v1", b"fn demo() {}");
    let implements = SourceEdge::new(
        subject.reference(),
        SourceEdgeKind::Implements,
        None,
        tree.requirement.reference(),
    )
    .expect("navigation edge");
    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = SourceGraph::new(
        nodes,
        vec![implements],
        vec![tree.document.reference(), subject.reference()],
        budgets(),
    )
    .expect("closed graph");

    let preimage = graph
        .nodes()
        .iter()
        .flat_map(|node| {
            node.children().iter().map(|child| {
                (
                    node.reference(),
                    child.relation(),
                    Some(child.position()),
                    child.target(),
                )
            })
        })
        .collect::<Vec<_>>();
    let containment = graph
        .edges()
        .iter()
        .filter(|edge| edge.relation() == SourceEdgeKind::Contains)
        .map(|edge| {
            (
                edge.source(),
                edge.relation(),
                edge.position(),
                edge.target(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        containment, preimage,
        "aggregate containment is exactly the union of every node preimage"
    );
    assert_eq!(preimage.len(), 4);
    assert!(
        graph
            .edges()
            .iter()
            .all(|edge| edge.position().is_some() == (edge.relation() == SourceEdgeKind::Contains)),
        "a position marks containment and only containment"
    );
}

#[test]
fn source_node_references_parse_only_canonical_digest_text() {
    let node = leaf("src/lib.rs", "rust/item/v1", b"fn demo() {}");
    let text = node.reference().canonical_text();
    assert_eq!(SourceNodeRef::parse(&text), Ok(node.reference()));
    assert_eq!(
        SourceNodeRef::parse(&text).expect("round trip").digest(),
        node.digest()
    );

    for (text, expected) in [
        ("nothing", DigestTextError::AlgorithmOrCase),
        ("blake4-512:00", DigestTextError::AlgorithmOrCase),
        ("xxh3-128:00", DigestTextError::Length),
        (
            "xxh3-128:zz000000000000000000000000000000",
            DigestTextError::Hex,
        ),
    ] {
        assert_eq!(
            SourceNodeRef::parse(text),
            Err(SourceGraphError::NodeReferenceText(expected)),
            "reference text {text} must fail closed"
        );
    }
}

#[test]
fn source_edge_kinds_parse_exactly_the_frozen_vocabulary() {
    for (text, kind) in [
        ("contains", SourceEdgeKind::Contains),
        ("implements", SourceEdgeKind::Implements),
        ("authority", SourceEdgeKind::Authority),
    ] {
        assert_eq!(SourceEdgeKind::parse(text), Ok(kind));
        assert_eq!(kind.as_str(), text);
    }
    for text in [
        "Contains",
        "contains ",
        "",
        "derives",
        "conta\u{69}\u{301}ns",
    ] {
        assert_eq!(
            SourceEdgeKind::parse(text),
            Err(SourceGraphError::EdgeKindGrammar),
            "relation {text} is outside the frozen vocabulary"
        );
    }
}

#[test]
fn source_edge_requires_a_child_position_for_containment_only() {
    let document = leaf(
        "openspec/specs/demo/spec.md",
        "openspec/document/v1",
        b"demo",
    );
    let child = leaf(
        "openspec/specs/demo/spec.md",
        "openspec/step/v1",
        b"GIVEN a",
    );
    assert_eq!(
        SourceEdge::new(
            document.reference(),
            SourceEdgeKind::Contains,
            None,
            child.reference(),
        ),
        Err(SourceGraphError::MissingChildPosition)
    );

    let edge = SourceEdge::new(
        document.reference(),
        SourceEdgeKind::Contains,
        Some(0),
        child.reference(),
    )
    .expect("positioned containment");
    assert_eq!(edge.source(), document.reference());
    assert_eq!(edge.relation(), SourceEdgeKind::Contains);
    assert_eq!(edge.position(), Some(0));
    assert_eq!(edge.target(), child.reference());

    let navigation = SourceEdge::new(
        child.reference(),
        SourceEdgeKind::Implements,
        None,
        document.reference(),
    )
    .expect("unpositioned navigation");
    assert_eq!(navigation.position(), None);

    for relation in [SourceEdgeKind::Implements, SourceEdgeKind::Authority] {
        for position in [0, 1, u32::MAX] {
            assert_eq!(
                SourceEdge::new(
                    child.reference(),
                    relation,
                    Some(position),
                    document.reference(),
                ),
                Err(SourceGraphError::PositionedNavigation),
                "a positioned {relation:?} edge would duplicate the canonical unpositioned form"
            );
        }
    }
}

#[test]
fn source_edge_aggregation_closes_over_declared_nodes_and_positions() {
    let tree = spec_tree(b"THEN b");
    let subject = leaf("src/lib.rs", "rust/item/v1", b"fn demo() {}");
    let absent = leaf("src/lib.rs", "rust/item/v1", b"fn absent() {}");
    let implements = SourceEdge::new(
        subject.reference(),
        SourceEdgeKind::Implements,
        None,
        tree.requirement.reference(),
    )
    .expect("navigation edge");

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = SourceGraph::new(
        nodes.clone(),
        vec![implements],
        vec![tree.document.reference()],
        budgets(),
    )
    .expect("declared edges join one closed graph");
    assert_eq!(graph.nodes().len(), 6);
    assert_eq!(graph.edges().len(), 5);
    assert_eq!(graph.roots(), &[tree.document.reference()]);
    assert_eq!(
        graph.node(tree.requirement.reference()),
        Some(&tree.requirement)
    );
    assert_eq!(graph.node(absent.reference()), None);
    assert_eq!(graph.budgets(), budgets());

    let dangling = SourceEdge::new(
        subject.reference(),
        SourceEdgeKind::Implements,
        None,
        absent.reference(),
    )
    .expect("edge value");
    assert_eq!(
        SourceGraph::new(
            nodes.clone(),
            vec![dangling],
            vec![tree.document.reference()],
            budgets(),
        ),
        Err(SourceGraphError::DanglingEdge)
    );
    let dangling_source = SourceEdge::new(
        absent.reference(),
        SourceEdgeKind::Implements,
        None,
        subject.reference(),
    )
    .expect("dangling-source edge value");
    assert_eq!(
        SourceGraph::new(
            nodes.clone(),
            vec![dangling_source],
            vec![tree.document.reference()],
            budgets(),
        ),
        Err(SourceGraphError::DanglingEdge)
    );
    assert_eq!(
        SourceGraph::new(
            nodes.clone(),
            Vec::new(),
            vec![absent.reference()],
            budgets(),
        ),
        Err(SourceGraphError::DanglingRoot)
    );
    let occupied = SourceEdge::new(
        tree.scenario.reference(),
        SourceEdgeKind::Contains,
        Some(0),
        subject.reference(),
    )
    .expect("edge value");
    assert_eq!(
        SourceGraph::new(
            nodes.clone(),
            vec![occupied],
            vec![tree.document.reference()],
            budgets(),
        ),
        Err(SourceGraphError::DuplicateChildPosition)
    );

    let gap = SourceEdge::new(
        tree.scenario.reference(),
        SourceEdgeKind::Contains,
        Some(7),
        subject.reference(),
    )
    .expect("edge value");
    assert_eq!(
        SourceGraph::new(nodes, vec![gap], vec![tree.document.reference()], budgets(),),
        Err(SourceGraphError::ChildPositionGap)
    );
}

#[test]
fn source_edge_aggregation_rejects_declared_containment_outside_every_node_preimage() {
    let tree = spec_tree(b"THEN b");
    let subject = leaf("src/lib.rs", "rust/item/v1", b"fn demo() {}");
    let mut nodes = tree.nodes();
    nodes.push(subject.clone());

    // The scenario preimage binds children 0 and 1, and the leaf steps bind
    // none, so both positions here are duplicate free and contiguous. Only the
    // node preimage separates declared containment from injected containment.
    for (parent, position, target) in [
        (&tree.scenario, 2, &subject),
        (&tree.given, 0, &subject),
        (&tree.then, 0, &tree.given),
        (&tree.document, 1, &tree.scenario),
    ] {
        let injected = SourceEdge::new(
            parent.reference(),
            SourceEdgeKind::Contains,
            Some(position),
            target.reference(),
        )
        .expect("edge value");
        assert_eq!(
            SourceGraph::new(
                nodes.clone(),
                vec![injected],
                vec![tree.document.reference()],
                budgets(),
            ),
            Err(SourceGraphError::UnbackedContainment),
            "the next contiguous position never admits unbacked containment"
        );
    }

    let backed = SourceGraph::new(
        nodes.clone(),
        Vec::new(),
        vec![tree.document.reference()],
        budgets(),
    )
    .expect("closed graph");
    let repeated = tree
        .scenario
        .children()
        .iter()
        .map(|child| {
            SourceEdge::new(
                tree.scenario.reference(),
                child.relation(),
                Some(child.position()),
                child.target(),
            )
            .expect("edge value")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        SourceGraph::new(nodes, repeated, vec![tree.document.reference()], budgets(),),
        Ok(backed),
        "one declared containment edge repeating its preimage deduplicates"
    );
}

#[test]
fn source_edge_aggregation_is_idempotent_for_equal_nodes_edges_and_roots() {
    let tree = spec_tree(b"THEN b");
    let subject = leaf("src/lib.rs", "rust/item/v1", b"fn demo() {}");
    let implements = SourceEdge::new(
        subject.reference(),
        SourceEdgeKind::Implements,
        None,
        tree.requirement.reference(),
    )
    .expect("navigation edge");
    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let roots = vec![tree.document.reference(), subject.reference()];

    let once = SourceGraph::new(nodes.clone(), vec![implements], roots.clone(), budgets())
        .expect("closed graph");
    let twice = SourceGraph::new(
        nodes.iter().chain(nodes.iter()).cloned().collect(),
        vec![implements, implements, implements],
        roots.iter().chain(roots.iter()).copied().collect(),
        budgets(),
    )
    .expect("equal parts aggregate idempotently");

    assert_eq!(once, twice);
    assert_eq!(once.digest(), twice.digest());
    assert_eq!(twice.nodes().len(), 6);
    assert_eq!(twice.edges().len(), 5);
    assert_eq!(twice.roots().len(), 2);

    let exact = exact_budgets(6, 5, 16, 64);
    assert!(
        SourceGraph::new(
            nodes.iter().chain(nodes.iter()).cloned().collect(),
            vec![implements, implements],
            roots.iter().chain(roots.iter()).copied().collect(),
            exact,
        )
        .is_ok(),
        "a repeated part never consumes a second unit of any budget"
    );
}

#[test]
fn source_graph_diagnostics_ignore_every_input_permutation() {
    let tree = spec_tree(b"THEN b");
    let absent = leaf("src/lib.rs", "rust/item/v1", b"fn absent() {}");
    let nodes = tree.nodes();
    let present = tree.document.reference();
    let missing = absent.reference();

    for roots in [
        vec![present, present, missing],
        vec![missing, present, present],
        vec![present, missing, present],
        vec![missing, missing, present],
    ] {
        assert_eq!(
            SourceGraph::new(nodes.clone(), Vec::new(), roots.clone(), budgets()),
            Err(SourceGraphError::DanglingRoot),
            "root order never selects the reported diagnostic: {roots:?}"
        );
    }

    let mut permuted = nodes.clone();
    permuted.reverse();
    for order in [nodes, permuted] {
        assert_eq!(
            SourceGraph::new(
                order,
                Vec::new(),
                vec![present, missing],
                exact_budgets(1, 64, 16, 64),
            ),
            Err(SourceGraphError::NodeBudget),
            "one fixed precedence reports the node budget before every later check"
        );
    }
}

#[test]
fn source_graph_digest_binds_every_part_and_ignores_input_order() {
    let tree = spec_tree(b"THEN b");
    let subject = leaf("src/lib.rs", "rust/item/v1", b"fn demo() {}");
    let implements = SourceEdge::new(
        subject.reference(),
        SourceEdgeKind::Implements,
        None,
        tree.requirement.reference(),
    )
    .expect("navigation edge");
    let mut nodes = tree.nodes();
    nodes.push(subject.clone());

    let forward = SourceGraph::new(
        nodes.clone(),
        vec![implements],
        vec![tree.document.reference(), subject.reference()],
        budgets(),
    )
    .expect("closed graph");
    let mut reversed_nodes = nodes.clone();
    reversed_nodes.reverse();
    let reversed = SourceGraph::new(
        reversed_nodes,
        vec![implements],
        vec![subject.reference(), tree.document.reference()],
        budgets(),
    )
    .expect("closed graph");
    assert_eq!(forward.digest(), reversed.digest());
    assert_eq!(forward, reversed);
    assert_eq!(forward.nodes(), reversed.nodes());
    assert_eq!(forward.edges(), reversed.edges());
    assert_eq!(forward.roots(), reversed.roots());

    for changed in [
        SourceGraph::new(
            nodes.clone(),
            vec![implements],
            vec![tree.document.reference()],
            budgets(),
        ),
        SourceGraph::new(
            nodes.clone(),
            Vec::new(),
            vec![tree.document.reference(), subject.reference()],
            budgets(),
        ),
        SourceGraph::new(
            nodes.clone(),
            vec![implements],
            vec![tree.document.reference(), subject.reference()],
            exact_budgets(65, 64, 16, 64),
        ),
    ] {
        assert_ne!(
            forward.digest(),
            changed.expect("closed graph").digest(),
            "every graph part binds the graph digest"
        );
    }
}

#[test]
fn source_reachability_is_exactly_the_current_root_closure() {
    let tree = spec_tree(b"THEN b");
    let historical = spec_tree(b"THEN historical");
    let mut nodes = tree.nodes();
    nodes.push(historical.then.clone());
    nodes.push(historical.scenario.clone());
    nodes.push(historical.requirement.clone());

    let graph = SourceGraph::new(
        nodes,
        Vec::new(),
        vec![tree.document.reference()],
        budgets(),
    )
    .expect("historical nodes remain stored");
    let closure = source_root_closure(&graph).expect("acyclic bounded closure");

    assert_eq!(closure.visited(), 5);
    assert_eq!(closure.depth(), 3);
    assert_eq!(
        closure.order(),
        &[
            tree.document.reference(),
            tree.requirement.reference(),
            tree.scenario.reference(),
            tree.given.reference(),
            tree.then.reference(),
        ]
    );
    assert!(closure.contains(tree.requirement.reference()));
    assert!(
        !closure.contains(historical.requirement.reference()),
        "a stored historical node is never current"
    );
    assert_eq!(
        require_current_source_reference(&closure, tree.requirement.reference()),
        Ok(())
    );
    assert_eq!(
        require_current_source_reference(&closure, historical.requirement.reference()),
        Err(SourceGraphError::StaleReference)
    );

    let unrooted = SourceGraph::new(tree.nodes(), Vec::new(), Vec::new(), budgets())
        .expect("an empty current root set is representable");
    let empty = source_root_closure(&unrooted).expect("an empty closure is bounded");
    assert_eq!(empty.visited(), 0);
    assert_eq!(empty.depth(), 0);
    assert!(empty.order().is_empty());
    assert_eq!(
        require_current_source_reference(&empty, tree.document.reference()),
        Err(SourceGraphError::StaleReference)
    );
}

#[test]
fn source_reachability_rejects_every_cycle_at_construction() {
    let tree = spec_tree(b"THEN b");
    let back_edge = SourceEdge::new(
        tree.then.reference(),
        SourceEdgeKind::Implements,
        None,
        tree.document.reference(),
    )
    .expect("edge value");
    assert_eq!(
        SourceGraph::new(
            tree.nodes(),
            vec![back_edge],
            vec![tree.document.reference()],
            budgets(),
        ),
        Err(SourceGraphError::Cycle),
        "acyclicity is one construction invariant, not one traversal outcome"
    );

    let self_loop = SourceEdge::new(
        tree.given.reference(),
        SourceEdgeKind::Implements,
        None,
        tree.given.reference(),
    )
    .expect("edge value");
    assert_eq!(
        SourceGraph::new(
            tree.nodes(),
            vec![self_loop],
            vec![tree.document.reference()],
            budgets(),
        ),
        Err(SourceGraphError::Cycle)
    );

    let first = leaf("src/lib.rs", "rust/item/v1", b"fn first() {}");
    let second = leaf("src/lib.rs", "rust/item/v1", b"fn second() {}");
    let unreachable = [(&first, &second), (&second, &first)]
        .into_iter()
        .map(|(source, target)| {
            SourceEdge::new(
                source.reference(),
                SourceEdgeKind::Implements,
                None,
                target.reference(),
            )
            .expect("edge value")
        })
        .collect::<Vec<_>>();
    let mut nodes = tree.nodes();
    nodes.push(first);
    nodes.push(second);
    assert_eq!(
        SourceGraph::new(
            nodes.clone(),
            unreachable.clone(),
            vec![tree.document.reference()],
            budgets(),
        ),
        Err(SourceGraphError::Cycle),
        "a cycle outside every current root closure still fails closed"
    );
    assert_eq!(
        SourceGraph::new(nodes.clone(), unreachable, Vec::new(), budgets()),
        Err(SourceGraphError::Cycle),
        "an empty current root set never hides a cycle"
    );

    assert!(
        SourceGraph::new(
            nodes,
            Vec::new(),
            vec![tree.document.reference()],
            budgets()
        )
        .is_ok(),
        "unreachable acyclic nodes remain representable"
    );
}

#[test]
fn source_traversal_over_the_empty_graph_is_closed_and_bounded() {
    let empty = SourceGraph::new(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        exact_budgets(1, 1, 1, 1),
    )
    .expect("the empty graph is closed, acyclic, and inside every budget");
    assert!(empty.nodes().is_empty());
    assert!(empty.edges().is_empty());
    assert!(empty.roots().is_empty());

    let closure = source_root_closure(&empty).expect("an empty closure is bounded");
    assert!(closure.order().is_empty());
    assert_eq!(closure.visited(), 0);
    assert_eq!(closure.depth(), 0);

    let absent = leaf("src/lib.rs", "rust/item/v1", b"fn absent() {}");
    assert_eq!(
        source_traversal(&empty, absent.reference()),
        Err(SourceGraphError::UnknownNode)
    );
    assert_eq!(
        require_current_source_reference(&closure, absent.reference()),
        Err(SourceGraphError::StaleReference)
    );
    assert_eq!(
        empty.digest(),
        SourceGraph::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            exact_budgets(1, 1, 1, 1)
        )
        .expect("closed graph")
        .digest()
    );
}

#[test]
fn source_traversal_returns_canonical_order_from_one_binding_subject() {
    let tree = spec_tree(b"THEN b");
    let subject = leaf("src/lib.rs", "rust/item/v1", b"fn demo() {}");
    let atom = leaf("openspec/specs/demo/spec.md", "openspec/atom/v1", b"ATOM-1");
    let implements = SourceEdge::new(
        subject.reference(),
        SourceEdgeKind::Implements,
        None,
        tree.requirement.reference(),
    )
    .expect("edge value");
    let authority = SourceEdge::new(
        tree.requirement.reference(),
        SourceEdgeKind::Authority,
        None,
        atom.reference(),
    )
    .expect("edge value");
    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    nodes.push(atom.clone());
    let graph = SourceGraph::new(
        nodes,
        vec![implements, authority],
        vec![tree.document.reference(), subject.reference()],
        budgets(),
    )
    .expect("closed binding graph");

    let traversal = source_traversal(&graph, subject.reference()).expect("bounded traversal");
    assert_eq!(
        traversal.order(),
        &[
            subject.reference(),
            tree.requirement.reference(),
            tree.scenario.reference(),
            tree.given.reference(),
            tree.then.reference(),
            atom.reference(),
        ],
        "containment precedes authority in canonical relation order"
    );
    assert_eq!(traversal.visited(), 6);
    assert_eq!(traversal.depth(), 3);
    assert!(traversal.contains(atom.reference()));

    let step = source_traversal(&graph, tree.given.reference()).expect("bounded traversal");
    assert_eq!(step.order(), &[tree.given.reference()]);
    assert_eq!(step.depth(), 0);
    assert_eq!(step.visited(), 1);

    let absent = leaf("src/lib.rs", "rust/item/v1", b"fn absent() {}");
    assert_eq!(
        source_traversal(&graph, absent.reference()),
        Err(SourceGraphError::UnknownNode)
    );
}

#[test]
fn source_traversal_fails_closed_on_every_exhausted_budget() {
    let tree = spec_tree(b"THEN b");
    let build = |budgets| {
        SourceGraph::new(
            tree.nodes(),
            Vec::new(),
            vec![tree.document.reference()],
            budgets,
        )
    };

    let graph = build(exact_budgets(5, 4, 3, 5)).expect("exact budgets admit the graph");
    let closure = source_root_closure(&graph).expect("exact budgets admit the closure");
    assert_eq!(closure.visited(), 5);
    assert_eq!(closure.depth(), 3);

    assert_eq!(
        build(exact_budgets(4, 4, 3, 5)),
        Err(SourceGraphError::NodeBudget)
    );
    assert_eq!(
        build(exact_budgets(5, 3, 3, 5)),
        Err(SourceGraphError::EdgeBudget)
    );

    let shallow = build(exact_budgets(5, 4, 2, 5)).expect("depth is a traversal budget");
    assert_eq!(
        source_root_closure(&shallow),
        Err(SourceGraphError::DepthBudget)
    );
    assert_eq!(
        source_traversal(&shallow, tree.document.reference()),
        Err(SourceGraphError::DepthBudget)
    );

    let narrow = build(exact_budgets(5, 4, 3, 4)).expect("visited is a traversal budget");
    assert_eq!(
        source_root_closure(&narrow),
        Err(SourceGraphError::VisitedBudget)
    );
    assert_eq!(
        source_traversal(&narrow, tree.document.reference()),
        Err(SourceGraphError::VisitedBudget)
    );
    assert!(
        source_traversal(&narrow, tree.scenario.reference()).is_ok(),
        "a narrower start stays inside the same visited budget"
    );

    let starved = build(exact_budgets(5, 4, 1, 1)).expect("both traversal budgets bind traversal");
    assert_eq!(
        source_root_closure(&starved),
        Err(SourceGraphError::VisitedBudget),
        "the walk stops at the crossed visited budget, before any depth relaxation"
    );
    assert_eq!(
        source_traversal(&starved, tree.then.reference()),
        Ok(source_traversal(&graph, tree.then.reference()).expect("bounded traversal")),
        "one exhausted closure never narrows an unrelated bounded traversal"
    );
}

#[test]
fn source_graph_budgets_reject_every_nonpositive_limit() {
    for value in [
        SourceNodeBudget::new(0).err(),
        SourceEdgeBudget::new(0).err(),
        SourceDepthBudget::new(0).err(),
        SourceVisitedBudget::new(0).err(),
    ] {
        assert_eq!(value, Some(SourceGraphError::NonPositiveBudget));
    }
    assert_eq!(SourceNodeBudget::new(7).expect("positive").get(), 7);
    assert_eq!(SourceEdgeBudget::new(7).expect("positive").get(), 7);
    assert_eq!(SourceDepthBudget::new(7).expect("positive").get(), 7);
    assert_eq!(SourceVisitedBudget::new(7).expect("positive").get(), 7);

    let configured = exact_budgets(1, 2, 3, 4);
    assert_eq!(configured.nodes().get(), 1);
    assert_eq!(configured.edges().get(), 2);
    assert_eq!(configured.depth().get(), 3);
    assert_eq!(configured.visited().get(), 4);
}

#[test]
fn source_graph_diagnostics_use_one_stable_code_each() {
    for (error, expected) in [
        (SourceGraphError::EdgeKindGrammar, "SRG-EDGE-KIND"),
        (
            SourceGraphError::NonPositiveBudget,
            "SRG-BUDGET-NONPOSITIVE",
        ),
        (
            SourceGraphError::DuplicateChildPosition,
            "SRG-CHILD-POSITION-DUPLICATE",
        ),
        (SourceGraphError::ChildPositionGap, "SRG-CHILD-POSITION-GAP"),
        (
            SourceGraphError::MissingChildPosition,
            "SRG-CHILD-POSITION-MISSING",
        ),
        (
            SourceGraphError::PositionedNavigation,
            "SRG-CHILD-POSITION-NAVIGATION",
        ),
        (SourceGraphError::NodeDigestCollision, "SRG-NODE-COLLISION"),
        (SourceGraphError::NodeDigestMember, "SRG-NODE-MEMBER"),
        (SourceGraphError::UnbackedContainment, "SRG-EDGE-UNBACKED"),
        (SourceGraphError::DanglingEdge, "SRG-EDGE-DANGLING"),
        (SourceGraphError::DanglingRoot, "SRG-ROOT-DANGLING"),
        (SourceGraphError::UnknownNode, "SRG-NODE-UNKNOWN"),
        (SourceGraphError::Cycle, "SRG-SOURCE-CYCLE"),
        (SourceGraphError::StaleReference, "SRG-SOURCE-STALE"),
        (SourceGraphError::NodeBudget, "SRG-BUDGET-NODES"),
        (SourceGraphError::EdgeBudget, "SRG-BUDGET-EDGES"),
        (SourceGraphError::VisitedBudget, "SRG-BUDGET-VISITED"),
        (SourceGraphError::DepthBudget, "SRG-BUDGET-DEPTH"),
    ] {
        assert_eq!(error.code(), expected);
    }
    assert_eq!(
        SourceGraphError::NodeReferenceText(DigestTextError::Hex).code(),
        DigestTextError::Hex.code()
    );
}
