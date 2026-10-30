#![doc = "Behavioral contracts for complete schema-edge impact indexing."]
#![allow(clippy::expect_used, clippy::panic)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::{
    SCHEMA_IMPACT_EDGES, SchemaImpactDependency, SchemaImpactEdge, SchemaImpactError,
    SchemaImpactIndex, SchemaImpactKey, SchemaImpactNode, SemanticDigest, schema_change_impacts,
};
use std::collections::{BTreeMap, BTreeSet};

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn key(kind: &str, id: &str) -> SchemaImpactKey {
    SchemaImpactKey::new(kind, id).expect("fixture key must be canonical")
}

fn node(
    kind: &str,
    id: &str,
    value: SemanticDigest,
    dependencies: Vec<SchemaImpactDependency>,
) -> SchemaImpactNode {
    SchemaImpactNode::new(key(kind, id), value, dependencies)
        .expect("fixture node must satisfy the frozen edge catalog")
}

fn edge_index(
    edge: SchemaImpactEdge,
    target_digest: SemanticDigest,
    include_edge: bool,
    reverse_nodes: bool,
) -> SchemaImpactIndex {
    let target = key(edge.target(), "TARGET");
    let dependencies = if include_edge {
        vec![
            SchemaImpactDependency::new(edge.id(), target.clone())
                .expect("catalog edge target must agree"),
        ]
    } else {
        Vec::new()
    };
    let mut nodes = vec![
        node(edge.target(), "TARGET", target_digest, Vec::new()),
        node(edge.source(), "SOURCE", digest(3), dependencies),
    ];
    if reverse_nodes {
        nodes.reverse();
    }
    SchemaImpactIndex::new(nodes).expect("fixture index must close")
}

fn catalog_edge(id: &str) -> SchemaImpactEdge {
    SCHEMA_IMPACT_EDGES
        .iter()
        .copied()
        .find(|edge| edge.id() == id)
        .expect("fixture edge must be inventoried")
}

fn chain_index(edge_ids: &[&str], root_digest: SemanticDigest) -> SchemaImpactIndex {
    let mut dependencies = BTreeMap::<String, Vec<SchemaImpactDependency>>::new();
    for (position, edge_id) in edge_ids.iter().enumerate() {
        let edge = catalog_edge(edge_id);
        let target_kind = if edge.target() == "object-reference" && position > 0 {
            catalog_edge(edge_ids[position - 1]).source()
        } else {
            edge.target()
        };
        let target = key(target_kind, target_kind);
        dependencies.entry(String::from(target_kind)).or_default();
        dependencies
            .entry(String::from(edge.source()))
            .or_default()
            .push(SchemaImpactDependency::new(edge.id(), target).expect("chain target must agree"));
    }
    let root_kind = catalog_edge(edge_ids[0]).target();
    let nodes = dependencies
        .into_iter()
        .map(|(kind, edges)| {
            let value = if kind == root_kind {
                root_digest
            } else {
                digest(6)
            };
            node(&kind, &kind, value, edges)
        })
        .collect();
    SchemaImpactIndex::new(nodes).expect("fixture chain must close")
}

fn impact_shape(impacts: &[buoy_core::SchemaImpact]) -> BTreeSet<(String, &'static str)> {
    impacts
        .iter()
        .map(|impact| {
            (
                String::from(impact.key().kind()),
                if impact.direct() {
                    "direct"
                } else {
                    "transitive"
                },
            )
        })
        .collect()
}

#[test]
fn frozen_catalog_exactly_matches_every_inventory_impact_edge() {
    let inventory = include_str!("../../../schema/v3/inventory.tsv");
    let inventory_edges = inventory
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns = line.split('\t').collect::<Vec<_>>();
            (columns.first().copied() == Some("impact-edge")).then(|| {
                let target = columns[4]
                    .strip_prefix("target:")
                    .expect("impact target shape");
                (
                    format!("{}-{}", columns[1], columns[2]),
                    String::from(columns[1]),
                    String::from(target),
                )
            })
        })
        .collect::<BTreeSet<_>>();
    let production_edges = SCHEMA_IMPACT_EDGES
        .iter()
        .map(|edge| {
            (
                String::from(edge.id()),
                String::from(edge.source()),
                String::from(edge.target()),
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(SCHEMA_IMPACT_EDGES.len(), 116);
    assert_eq!(production_edges.len(), SCHEMA_IMPACT_EDGES.len());
    assert_eq!(production_edges, inventory_edges);
}

#[test]
fn every_frozen_edge_propagates_and_its_omission_is_observable() {
    for edge in SCHEMA_IMPACT_EDGES.iter().copied() {
        let base = edge_index(edge, digest(1), true, false);
        let head = edge_index(edge, digest(2), true, false);
        let impacts = schema_change_impacts(&base, &head);
        assert_eq!(impacts.len(), 2, "edge {}", edge.id());
        let target_impact = impacts
            .iter()
            .find(|impact| impact.key().kind() == edge.target() && impact.key().id() == "TARGET")
            .expect("changed target impact");
        assert!(target_impact.direct());
        assert_eq!(target_impact.base(), Some(digest(1)));
        assert_eq!(target_impact.head(), Some(digest(2)));
        let source_impact = impacts
            .iter()
            .find(|impact| impact.key().kind() == edge.source() && impact.key().id() == "SOURCE")
            .expect("dependent source impact");
        assert!(!source_impact.direct());
        assert_eq!(source_impact.base(), Some(digest(3)));
        assert_eq!(source_impact.head(), Some(digest(3)));

        let omitted_base = edge_index(edge, digest(1), false, false);
        let omitted_head = edge_index(edge, digest(2), false, false);
        let omitted = schema_change_impacts(&omitted_base, &omitted_head);
        assert_eq!(omitted.len(), 1, "edge {} omission control", edge.id());
        assert!(omitted.iter().all(|impact| impact.key().id() != "SOURCE"));
    }
}

#[test]
fn assumption_property_proof_chain_reaches_the_fixed_point_in_canonical_order() {
    let assumption = key("assumption", "ASSUMPTION");
    let property = key("formal-property", "PROPERTY");
    let property_dependency =
        SchemaImpactDependency::new("formal-property-assumptions", assumption.clone())
            .expect("property edge");
    let proof_dependency =
        SchemaImpactDependency::new("proof-record-formal-property", property.clone())
            .expect("proof edge");
    let build = |assumption_digest, reverse: bool| {
        let mut nodes = vec![
            SchemaImpactNode::new(assumption.clone(), assumption_digest, Vec::new())
                .expect("assumption"),
            SchemaImpactNode::new(
                property.clone(),
                digest(4),
                vec![property_dependency.clone()],
            )
            .expect("property"),
            node(
                "proof-record",
                "PROOF",
                digest(5),
                vec![proof_dependency.clone()],
            ),
        ];
        if reverse {
            nodes.reverse();
        }
        SchemaImpactIndex::new(nodes).expect("closed chain")
    };

    let forward = schema_change_impacts(&build(digest(1), false), &build(digest(2), false));
    let reversed = schema_change_impacts(&build(digest(1), true), &build(digest(2), true));
    assert_eq!(forward, reversed);
    assert_eq!(forward.len(), 3);
    assert!(forward[0].direct());
    assert_eq!(forward[0].key().kind(), "assumption");
    assert_eq!(forward[1].key().kind(), "formal-property");
    assert!(!forward[1].direct());
    assert_eq!(forward[2].key().kind(), "proof-record");
    assert!(!forward[2].direct());

    let one_hop = &forward[..2];
    assert_ne!(one_hop, forward.as_slice(), "CHG-IMPACT-FIXED-POINT");
}

#[test]
fn traversal_unions_base_and_head_edges_and_terminates_on_cycles() {
    let assumption = key("assumption", "ASSUMPTION");
    let property_dependency =
        SchemaImpactDependency::new("formal-property-assumptions", assumption.clone())
            .expect("property edge");
    let base = SchemaImpactIndex::new(vec![
        SchemaImpactNode::new(assumption.clone(), digest(1), Vec::new()).expect("assumption"),
        node(
            "formal-property",
            "PROPERTY",
            digest(3),
            vec![property_dependency],
        ),
    ])
    .expect("base index");
    let head = SchemaImpactIndex::new(vec![
        SchemaImpactNode::new(assumption, digest(2), Vec::new()).expect("assumption"),
        node("formal-property", "PROPERTY", digest(3), Vec::new()),
    ])
    .expect("head index");
    let cross_side = schema_change_impacts(&base, &head);
    assert_eq!(cross_side.len(), 2);
    assert!(cross_side.iter().any(|impact| {
        impact.key().kind() == "formal-property" && impact.key().id() == "PROPERTY"
    }));

    let first = key("parsed-item-identity", "FIRST");
    let second = key("parsed-item-identity", "SECOND");
    let cycle = |first_digest| {
        SchemaImpactIndex::new(vec![
            SchemaImpactNode::new(
                first.clone(),
                first_digest,
                vec![
                    SchemaImpactDependency::new(
                        "parsed-item-identity-enclosing-item",
                        second.clone(),
                    )
                    .expect("first cycle edge"),
                ],
            )
            .expect("first item"),
            SchemaImpactNode::new(
                second.clone(),
                digest(4),
                vec![
                    SchemaImpactDependency::new(
                        "parsed-item-identity-enclosing-item",
                        first.clone(),
                    )
                    .expect("second cycle edge"),
                ],
            )
            .expect("second item"),
        ])
        .expect("closed cycle")
    };
    let cycle_impacts = schema_change_impacts(&cycle(digest(1)), &cycle(digest(2)));
    assert_eq!(cycle_impacts.len(), 2);
    assert_eq!(
        cycle_impacts
            .iter()
            .filter(|impact| impact.direct())
            .count(),
        1
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn committed_impact_fixtures_execute_against_production_and_reject_false_controls() {
    let fixtures = include_str!("../../../fixtures/hardening/v3/impact-edge-fixtures.tsv");
    let mut executed = 0_usize;
    for line in fixtures.lines().skip(1) {
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 14, "fixture row shape: {}", columns[1]);
        let edge_ids = columns[3].split('+').collect::<Vec<_>>();
        let (base, head) = if edge_ids.len() == 1 {
            let edge = catalog_edge(edge_ids[0]);
            (
                edge_index(edge, digest(1), true, false),
                edge_index(edge, digest(2), true, false),
            )
        } else {
            (
                chain_index(&edge_ids, digest(1)),
                chain_index(&edge_ids, digest(2)),
            )
        };
        let production = schema_change_impacts(&base, &head);
        let expected = columns[13]
            .split(';')
            .map(|entry| {
                let (kind, classification) = entry.split_once(':').expect("expected impact shape");
                (String::from(kind), classification)
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            impact_shape(&production),
            expected,
            "production fixture {}",
            columns[1]
        );

        match columns[10] {
            "none" => {
                assert_eq!(columns[2], "positive");
                assert_eq!(columns[11], "accept");
                assert_eq!(columns[12], "-");
            }
            "omit-declared-impact-edge" => {
                let edge = catalog_edge(edge_ids[0]);
                let omitted = schema_change_impacts(
                    &edge_index(edge, digest(1), false, false),
                    &edge_index(edge, digest(2), false, false),
                );
                assert_ne!(impact_shape(&omitted), expected, "fixture {}", columns[1]);
                assert_eq!(columns[2], "negative");
                assert_eq!(columns[11], "reject");
                assert_eq!(columns[12], "CHG-IMPACT-EDGE-OMITTED");
            }
            "stop-after-one-hop" => {
                let first_hop = catalog_edge(edge_ids[0]).source();
                let one_hop = production
                    .iter()
                    .filter(|impact| impact.direct() || impact.key().kind() == first_hop)
                    .cloned()
                    .collect::<Vec<_>>();
                assert_ne!(impact_shape(&one_hop), expected, "fixture {}", columns[1]);
                assert_eq!(columns[2], "negative");
                assert_eq!(columns[11], "reject");
                assert_eq!(columns[12], "CHG-IMPACT-FIXED-POINT");
            }
            mutation => panic!("unknown committed mutation {mutation}"),
        }
        executed += 1;
    }
    assert_eq!(executed, 238);
}

#[test]
fn index_construction_rejects_unknown_mismatched_duplicate_and_dangling_edges() {
    assert_eq!(
        SchemaImpactKey::new("not-in-schema", "VALUE"),
        Err(SchemaImpactError::UnknownKind)
    );
    assert_eq!(
        SchemaImpactKey::new("assumption", ""),
        Err(SchemaImpactError::InvalidKey)
    );
    assert_eq!(
        SchemaImpactDependency::new("missing-edge", key("assumption", "A")),
        Err(SchemaImpactError::UnknownEdge)
    );
    assert_eq!(
        SchemaImpactDependency::new("formal-property-assumptions", key("formal-property", "P")),
        Err(SchemaImpactError::TargetKind)
    );

    let assumption = key("assumption", "A");
    let dependency = SchemaImpactDependency::new("formal-property-assumptions", assumption.clone())
        .expect("valid dependency");
    assert_eq!(
        SchemaImpactNode::new(
            key("proof-record", "PROOF"),
            digest(1),
            vec![dependency.clone()]
        ),
        Err(SchemaImpactError::SourceKind)
    );
    assert_eq!(
        SchemaImpactNode::new(
            key("formal-property", "PROPERTY"),
            digest(1),
            vec![dependency.clone(), dependency.clone()]
        ),
        Err(SchemaImpactError::DuplicateDependency)
    );

    let property = SchemaImpactNode::new(
        key("formal-property", "PROPERTY"),
        digest(2),
        vec![dependency],
    )
    .expect("valid property node");
    assert_eq!(
        SchemaImpactIndex::new(vec![property.clone()]),
        Err(SchemaImpactError::DanglingDependency)
    );
    let assumption_node =
        SchemaImpactNode::new(assumption, digest(3), Vec::new()).expect("assumption node");
    assert_eq!(
        SchemaImpactIndex::new(vec![assumption_node.clone(), assumption_node]),
        Err(SchemaImpactError::DuplicateNode)
    );

    for error in [
        SchemaImpactError::UnknownKind,
        SchemaImpactError::InvalidKey,
        SchemaImpactError::UnknownEdge,
        SchemaImpactError::TargetKind,
        SchemaImpactError::SourceKind,
        SchemaImpactError::DuplicateDependency,
        SchemaImpactError::DuplicateNode,
        SchemaImpactError::DanglingDependency,
    ] {
        assert!(error.code().starts_with("CHG-IMPACT-"));
    }
}
