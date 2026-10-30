#![doc = "Behavioral execution of every frozen policy-ratchet fixture."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::*;
use std::fs;

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
}

fn payload(schema: &str, bytes: impl Into<Vec<u8>>) -> TypedPayload {
    TypedPayload::new(SchemaId::new(schema).expect("valid schema"), bytes.into())
        .expect("fixture payload")
}

fn approval() -> ApprovalBinding {
    ApprovalBinding::new(
        ScopeId::new("maintainers").expect("valid scope"),
        RepositoryPath::new("reviews/policy.md").expect("approval path"),
        digest(90),
    )
    .expect("approval")
}

fn atom(id: &str, statement: &str) -> Atom {
    Atom::new(
        sid(id),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("policy.ratchet").expect("valid scope")],
        statement,
        Some(approval()),
    )
    .expect("fixture atom")
}

fn object_ref(meta: &ObjectMeta) -> ObjectRef {
    ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest())
}

fn selector(index: u8) -> TypedSelector {
    let root = format!("specs/authority-{index}");
    TypedSelector::new(
        RepositoryPath::new(&root).expect("selector root"),
        SchemaId::new("markdown/v1").expect("valid schema"),
        vec![String::from("**/*.md")],
        Vec::new(),
        AuthorityExpectation::Required,
    )
    .expect("authority selector")
}

#[derive(Clone)]
struct GraphConfig {
    subject_count: usize,
    roles: Vec<EvidenceRole>,
    mode: EnforcementMode,
    ratchet: &'static str,
    authority_count: u8,
    derivation: bool,
    scope_byte: u8,
    surface_members: Option<usize>,
    waiver_classes: Vec<&'static str>,
    lifecycle: PolicyLifecycle,
    omit_policy: bool,
    replacement_roles: Option<Vec<EvidenceRole>>,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            subject_count: 1,
            roles: vec![EvidenceRole::Positive],
            mode: EnforcementMode::Blocking,
            ratchet: "ratchet-line-a",
            authority_count: 0,
            derivation: false,
            scope_byte: 1,
            surface_members: None,
            waiver_classes: Vec::new(),
            lifecycle: PolicyLifecycle::Active,
            omit_policy: false,
            replacement_roles: None,
        }
    }
}

fn policy_revision(config: &GraphConfig, roles: Vec<EvidenceRole>) -> CoveragePolicyRevision {
    CoveragePolicyRevision::new(
        payload("policy-scope/v1", vec![config.scope_byte]),
        roles,
        config.mode,
        ScopeId::new(config.ratchet).expect("valid scope"),
        (0..config.authority_count).map(selector).collect(),
        config
            .derivation
            .then(|| payload("derivation-requirement/v1", vec![1])),
    )
    .expect("policy revision")
}

fn surface(authority: &Atom, member_count: usize) -> DecisionSurface {
    let authority_ref = AtomRevisionRef::new(authority.meta().id().clone(), authority.revision());
    let members = (1_u8..)
        .take(member_count)
        .map(|index| payload("surface-member/v1", vec![index]))
        .collect::<Vec<_>>();
    let domain = DecisionSurfaceDomain::new(
        sid("SURFACE-POLICY"),
        payload("surface-key-schema/v1", vec![1]),
        payload("surface-disposition-schema/v1", vec![1]),
        members.clone(),
        1,
    )
    .expect("surface domain");
    let dispositions = members
        .into_iter()
        .map(|member| {
            SurfaceDisposition::new(
                member,
                SurfaceDispositionKind::Normative,
                Some(payload("surface-value/v1", vec![1])),
                Some(authority_ref.clone()),
            )
            .expect("surface disposition")
        })
        .collect();
    DecisionSurface::new(domain, dispositions, Vec::new()).expect("decision surface")
}

fn build_graph(config: &GraphConfig) -> SpecGraph {
    let atom_a = atom("POLICY-SUBJECT-A", "Subject A MUST remain governed.");
    let atom_b = atom("POLICY-SUBJECT-B", "Subject B MUST remain governed.");
    let authority = atom(
        "SURFACE-AUTHORITY",
        "The decision surface MUST have normative authority.",
    );
    let mut graph = SpecGraph {
        atoms: vec![atom_a.clone(), atom_b.clone(), authority.clone()],
        ..SpecGraph::default()
    };
    let subjects = if let Some(member_count) = config.surface_members {
        let surface = surface(&authority, member_count);
        let subject = object_ref(surface.meta());
        graph.decision_surfaces.push(surface);
        vec![subject]
    } else {
        [object_ref(atom_a.meta()), object_ref(atom_b.meta())]
            .into_iter()
            .take(config.subject_count)
            .collect()
    };
    if config.omit_policy {
        return graph;
    }

    let replacement = config.replacement_roles.as_ref().map(|roles| {
        CoveragePolicy::new(
            sid("POLICY-REPLACEMENT"),
            policy_revision(config, roles.clone()),
            approval(),
            PolicyLifecycle::Active,
            Vec::new(),
            None,
            subjects.clone(),
        )
        .expect("replacement policy")
    });
    let replacements = replacement
        .as_ref()
        .map(|policy| vec![object_ref(policy.meta())])
        .unwrap_or_default();
    let terminal_approval = (config.lifecycle != PolicyLifecycle::Active).then(approval);
    let policy = CoveragePolicy::new(
        sid("POLICY-RATCHET"),
        policy_revision(config, config.roles.clone()),
        approval(),
        config.lifecycle,
        replacements,
        terminal_approval,
        subjects.clone(),
    )
    .expect("fixture policy");

    for (index, class) in config.waiver_classes.iter().enumerate() {
        let obligation = ObligationRef::new(
            sid("POLICY-RATCHET"),
            ObligationKind::RequiredRole,
            subjects[0].clone(),
            Some(EvidenceRole::Positive),
        )
        .expect("waiver obligation");
        graph.waivers.push(
            Waiver::new(
                sid(&format!("WAIVER-CLASS-{index}")),
                vec![obligation],
                vec![subjects[0].clone()],
                "Reviewed fixture waiver.",
                ScopeId::new("maintainers").expect("valid scope"),
                payload("waiver-class/v1", class.as_bytes().to_vec()),
                approval(),
                Vec::new(),
                WaiverLifecycle::Active,
                Vec::new(),
                Vec::new(),
                None,
            )
            .expect("fixture waiver"),
        );
    }
    graph.coverage_policies.push(policy);
    if let Some(replacement) = replacement {
        graph.coverage_policies.push(replacement);
    }
    graph
}

fn fixture_graphs(dimension: &str, polarity: &str) -> (SpecGraph, SpecGraph) {
    let negative = polarity == "negative";
    let mut base = GraphConfig::default();
    let mut head = base.clone();
    match dimension {
        "authority" => {
            base.authority_count = 1;
            head.authority_count = if negative { 0 } else { 2 };
        }
        "derivation" => {
            base.derivation = negative;
            head.derivation = !negative;
        }
        "enforcement-mode" => {
            base.mode = if negative {
                EnforcementMode::Blocking
            } else {
                EnforcementMode::Advisory
            };
            head.mode = if negative {
                EnforcementMode::Advisory
            } else {
                EnforcementMode::Blocking
            };
        }
        "evidence-role" => {
            if negative {
                base.roles.push(EvidenceRole::Negative);
            } else {
                head.roles.push(EvidenceRole::Negative);
            }
        }
        "lineage" => {
            if negative {
                head.ratchet = "ratchet-line-b";
            } else {
                head.scope_byte = 2;
            }
        }
        "retirement" => {
            if negative {
                head.omit_policy = true;
            } else {
                head.lifecycle = PolicyLifecycle::Retired;
            }
        }
        "scope" => {
            base.subject_count = if negative { 2 } else { 1 };
            head.subject_count = if negative { 1 } else { 2 };
        }
        "supersession" => {
            head.lifecycle = PolicyLifecycle::Superseded;
            head.replacement_roles = Some(if negative {
                Vec::new()
            } else {
                vec![EvidenceRole::Positive]
            });
        }
        "surface-evolution" => {
            base.surface_members = Some(if negative { 2 } else { 1 });
            head.surface_members = Some(if negative { 1 } else { 2 });
        }
        "waiver-class" => {
            base.waiver_classes = if negative {
                vec!["exact-a"]
            } else {
                vec!["exact-a", "exact-b"]
            };
            head.waiver_classes = if negative {
                vec!["wildcard"]
            } else {
                vec!["exact-a"]
            };
        }
        other => panic!("unknown fixture dimension {other}"),
    }
    (build_graph(&base), build_graph(&head))
}

#[test]
fn all_frozen_policy_ratchet_fixtures_execute_against_graph_derivation() {
    let source = fs::read_to_string("../../fixtures/hardening/v3/policy-ratchet-fixtures.tsv")
        .expect("policy-ratchet fixtures");
    let mut executed = 0;
    for line in source.lines().skip(1) {
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 11, "fixture row shape");
        let case_id = columns[1];
        let polarity = columns[2];
        let dimension = columns[3];
        let expected_classification = columns[7];
        let expected_diagnostic = columns[9];
        let (base, head) = fixture_graphs(dimension, polarity);
        let ratchets = derive_policy_ratchets(&base, &head).expect("graph-derived ratchets");
        let ratchet = ratchets
            .iter()
            .find(|ratchet| ratchet.policy_id().as_str() == "POLICY-RATCHET")
            .unwrap_or_else(|| panic!("missing primary ratchet for {case_id}"));
        let expected = match expected_classification {
            "weakening" => PolicyChangeClassification::Weakening,
            "strengthening" => PolicyChangeClassification::Strengthening,
            "equivalent-reviewed" => PolicyChangeClassification::EquivalentReviewed,
            other => panic!("unknown fixture classification {other}"),
        };
        assert_eq!(ratchet.classification(), expected, "{case_id}");
        let expected_diagnostics = if expected_diagnostic == "-" {
            Vec::new()
        } else {
            vec![expected_diagnostic]
        };
        assert_eq!(
            ratchet
                .diagnostics()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected_diagnostics,
            "{case_id}"
        );
        if expected == PolicyChangeClassification::Weakening {
            assert!(
                !ratchet.removed_obligations().is_empty(),
                "{case_id} must expose exact weakened obligations"
            );
            if dimension != "retirement" {
                assert!(
                    ratchet.impact().is_some(),
                    "{case_id} must expose a canonical ratchet impact"
                );
            }
        }
        if polarity == "positive"
            && matches!(
                dimension,
                "authority" | "derivation" | "evidence-role" | "scope" | "surface-evolution"
            )
        {
            assert!(
                !ratchet.added_obligations().is_empty(),
                "{case_id} must expose exact added obligations"
            );
        }
        executed += 1;
    }
    assert_eq!(executed, 20);
}
