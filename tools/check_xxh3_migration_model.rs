//! Exhaustive digest acceptance, active-member, transition, and index model.

#![allow(
    clippy::expect_used,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

mod digest_binding;

use std::env;
use std::fs;

const MODEL_ID: &str = "buoy-digest-transition/v1";
const MODEL_REVISION: &str = "xxh3-128:538c71225931a9e9b6c37e71cab91d8d";
const MODEL_CONTRACT: &str = "fixtures/digest/v3/model-contract.txt";
const DIGEST_STATES: usize = 64;
const ACTIVE_STATES: usize = 12;
const TRANSITION_STATES: usize = 4096;
const TOTAL_STATES: usize = DIGEST_STATES + ACTIVE_STATES + TRANSITION_STATES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Diagnostic {
    ObsoleteTag,
    UnknownTag,
    WrongLength,
    NoncanonicalCase,
    InvalidHex,
    InactiveAlgorithm,
    ActiveMissing,
    ActiveMultiple,
    ActiveUnregistered,
    ActiveUnavailable,
    MissingAuthoritativeBytes,
    MissingApproval,
    DuplicateStableId,
    ObservedCollision,
    IllegalLifecycle,
    StaleReference,
    MixedMember,
    IncompleteImpactClosure,
}

#[derive(Clone, Copy, Debug)]
enum TagClass {
    ActiveRegistered,
    InactiveRegistered,
    Obsolete,
    Unknown,
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    Parse,
    Compute,
}

#[derive(Clone, Copy, Debug)]
struct DigestState {
    tag: TagClass,
    length_matches: bool,
    lowercase: bool,
    valid_hex: bool,
    operation: Operation,
}

#[derive(Clone, Copy, Debug)]
struct ActiveState {
    declared_count: u8,
    registered: bool,
    feature_available: bool,
}

#[derive(Clone, Copy, Debug)]
struct TransitionState {
    authoritative_bytes: bool,
    approval: bool,
    duplicate_stable_id: bool,
    observed_collision: bool,
    illegal_lifecycle: bool,
    stale_reference: bool,
    mixed_member: bool,
    incomplete_impact_closure: bool,
    index_present: bool,
    index_contradicts: bool,
    index_digest_keyed: bool,
    index_absent: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IndexVerdict {
    Valid,
    Absent,
    Contradiction,
    DigestKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TargetState {
    schema: &'static str,
    stable_ids: [u8; 3],
    lifecycle: [u8; 3],
    active_tag: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    None,
    DigestDivergence,
    ActiveDivergence,
    ModelIndexAuthority,
    IndexInfluencesIdentity,
    IndexContradictionAccepted,
    MissingInputAccepted,
    MissingApprovalAccepted,
    MutatedStableId,
    NonIdempotent,
    TransitionDivergence,
}

#[derive(Clone, Copy, Debug, Default)]
struct Counts {
    accepted: usize,
    rejected: usize,
    fixed_points: usize,
    max_fixed_point_iterations: usize,
}

fn digest_state(index: usize) -> DigestState {
    let tag = match index & 0b11 {
        0 => TagClass::ActiveRegistered,
        1 => TagClass::InactiveRegistered,
        2 => TagClass::Obsolete,
        _ => TagClass::Unknown,
    };
    DigestState {
        tag,
        length_matches: index & (1 << 2) != 0,
        lowercase: index & (1 << 3) != 0,
        valid_hex: index & (1 << 4) != 0,
        operation: if index & (1 << 5) == 0 {
            Operation::Parse
        } else {
            Operation::Compute
        },
    }
}

fn model_digest_acceptance(state: DigestState) -> Result<(), Diagnostic> {
    match state.tag {
        TagClass::Obsolete => return Err(Diagnostic::ObsoleteTag),
        TagClass::Unknown => return Err(Diagnostic::UnknownTag),
        TagClass::ActiveRegistered | TagClass::InactiveRegistered => {}
    }
    if !state.length_matches {
        return Err(Diagnostic::WrongLength);
    }
    if !state.lowercase {
        return Err(Diagnostic::NoncanonicalCase);
    }
    if !state.valid_hex {
        return Err(Diagnostic::InvalidHex);
    }
    if matches!(state.operation, Operation::Compute)
        && matches!(state.tag, TagClass::InactiveRegistered)
    {
        return Err(Diagnostic::InactiveAlgorithm);
    }
    Ok(())
}

fn implementation_digest_acceptance(state: DigestState, fault: Fault) -> Result<(), Diagnostic> {
    if matches!(fault, Fault::DigestDivergence)
        && matches!(state.tag, TagClass::ActiveRegistered)
        && state.length_matches
        && state.lowercase
        && state.valid_hex
    {
        return Err(Diagnostic::UnknownTag);
    }
    if matches!(state.tag, TagClass::Obsolete) {
        return Err(Diagnostic::ObsoleteTag);
    }
    if matches!(state.tag, TagClass::Unknown) {
        return Err(Diagnostic::UnknownTag);
    }
    if !state.length_matches {
        return Err(Diagnostic::WrongLength);
    }
    if !state.lowercase {
        return Err(Diagnostic::NoncanonicalCase);
    }
    if !state.valid_hex {
        return Err(Diagnostic::InvalidHex);
    }
    match (state.operation, state.tag) {
        (Operation::Compute, TagClass::InactiveRegistered) => Err(Diagnostic::InactiveAlgorithm),
        _ => Ok(()),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn active_state(index: usize) -> ActiveState {
    ActiveState {
        declared_count: (index % 3) as u8,
        registered: (index / 3) % 2 != 0,
        feature_available: (index / 6) % 2 != 0,
    }
}

fn model_active_resolution(state: ActiveState) -> Result<(), Diagnostic> {
    if state.declared_count == 0 {
        return Err(Diagnostic::ActiveMissing);
    }
    if state.declared_count > 1 {
        return Err(Diagnostic::ActiveMultiple);
    }
    if !state.registered {
        return Err(Diagnostic::ActiveUnregistered);
    }
    if !state.feature_available {
        return Err(Diagnostic::ActiveUnavailable);
    }
    Ok(())
}

#[allow(clippy::match_same_arms)]
fn implementation_active_resolution(state: ActiveState, fault: Fault) -> Result<(), Diagnostic> {
    if matches!(fault, Fault::ActiveDivergence)
        && state.declared_count == 1
        && state.registered
        && state.feature_available
    {
        return Err(Diagnostic::ActiveUnavailable);
    }
    match state.declared_count {
        0 => Err(Diagnostic::ActiveMissing),
        2 => Err(Diagnostic::ActiveMultiple),
        1 if !state.registered => Err(Diagnostic::ActiveUnregistered),
        1 if !state.feature_available => Err(Diagnostic::ActiveUnavailable),
        1 => Ok(()),
        _ => Err(Diagnostic::ActiveMultiple),
    }
}

fn transition_state(mask: usize) -> TransitionState {
    let bit = |index: usize| mask & (1usize << index) != 0usize;
    TransitionState {
        authoritative_bytes: bit(0),
        approval: bit(1),
        duplicate_stable_id: bit(2),
        observed_collision: bit(3),
        illegal_lifecycle: bit(4),
        stale_reference: bit(5),
        mixed_member: bit(6),
        incomplete_impact_closure: bit(7),
        index_present: bit(8),
        index_contradicts: bit(9),
        index_digest_keyed: bit(10),
        index_absent: bit(11),
    }
}

fn canonical_target() -> TargetState {
    TargetState {
        schema: "spec-provenance/v3",
        stable_ids: [1, 2, 3],
        lifecycle: [2, 4, 3],
        active_tag: 2,
    }
}

fn model_identity_transition(state: TransitionState) -> Result<TargetState, Diagnostic> {
    if !state.authoritative_bytes {
        return Err(Diagnostic::MissingAuthoritativeBytes);
    }
    if !state.approval {
        return Err(Diagnostic::MissingApproval);
    }
    if state.duplicate_stable_id {
        return Err(Diagnostic::DuplicateStableId);
    }
    if state.observed_collision {
        return Err(Diagnostic::ObservedCollision);
    }
    if state.illegal_lifecycle {
        return Err(Diagnostic::IllegalLifecycle);
    }
    if state.stale_reference {
        return Err(Diagnostic::StaleReference);
    }
    if state.mixed_member {
        return Err(Diagnostic::MixedMember);
    }
    if state.incomplete_impact_closure {
        return Err(Diagnostic::IncompleteImpactClosure);
    }
    Ok(canonical_target())
}

fn implementation_identity_transition(
    state: TransitionState,
    fault: Fault,
) -> Result<TargetState, Diagnostic> {
    if matches!(fault, Fault::IndexInfluencesIdentity) && state.index_present {
        return Err(Diagnostic::MixedMember);
    }
    if !state.authoritative_bytes && !matches!(fault, Fault::MissingInputAccepted) {
        return Err(Diagnostic::MissingAuthoritativeBytes);
    }
    if !state.approval
        && !matches!(
            fault,
            Fault::MissingInputAccepted | Fault::MissingApprovalAccepted
        )
    {
        return Err(Diagnostic::MissingApproval);
    }
    if state.duplicate_stable_id {
        return Err(Diagnostic::DuplicateStableId);
    }
    if state.observed_collision {
        return Err(Diagnostic::ObservedCollision);
    }
    if state.illegal_lifecycle {
        return Err(Diagnostic::IllegalLifecycle);
    }
    if state.stale_reference {
        return Err(Diagnostic::StaleReference);
    }
    if state.mixed_member {
        return Err(Diagnostic::MixedMember);
    }
    if state.incomplete_impact_closure {
        return Err(Diagnostic::IncompleteImpactClosure);
    }
    if matches!(fault, Fault::TransitionDivergence) {
        return Err(Diagnostic::StaleReference);
    }
    let mut target = canonical_target();
    if matches!(fault, Fault::MutatedStableId) {
        target.stable_ids[0] = 9;
    }
    Ok(target)
}

fn model_index_verification(state: TransitionState) -> IndexVerdict {
    if state.index_absent || !state.index_present {
        IndexVerdict::Absent
    } else if state.index_digest_keyed {
        IndexVerdict::DigestKey
    } else if state.index_contradicts {
        IndexVerdict::Contradiction
    } else {
        IndexVerdict::Valid
    }
}

fn implementation_index_verification(state: TransitionState, fault: Fault) -> IndexVerdict {
    if state.index_absent || !state.index_present {
        return IndexVerdict::Absent;
    }
    if state.index_digest_keyed {
        return if matches!(fault, Fault::IndexContradictionAccepted) {
            IndexVerdict::Valid
        } else {
            IndexVerdict::DigestKey
        };
    }
    if state.index_contradicts {
        return if matches!(fault, Fault::IndexContradictionAccepted) {
            IndexVerdict::Valid
        } else {
            IndexVerdict::Contradiction
        };
    }
    IndexVerdict::Valid
}

fn without_index(state: TransitionState) -> TransitionState {
    TransitionState {
        index_present: false,
        index_contradicts: false,
        index_digest_keyed: false,
        index_absent: false,
        ..state
    }
}

fn rerun_transition(target: TargetState, fault: Fault) -> TargetState {
    let state = TransitionState {
        authoritative_bytes: true,
        approval: true,
        duplicate_stable_id: false,
        observed_collision: false,
        illegal_lifecycle: false,
        stale_reference: false,
        mixed_member: false,
        incomplete_impact_closure: false,
        index_present: false,
        index_contradicts: false,
        index_digest_keyed: false,
        index_absent: true,
    };
    let mut rerun = implementation_identity_transition(state, Fault::None)
        .expect("the canonical rerun state always regenerates");
    if matches!(fault, Fault::NonIdempotent) {
        rerun.lifecycle[0] = if target.lifecycle[0] == 9 { 8 } else { 9 };
    }
    rerun
}

fn fixed_point_iterations(target: TargetState, fault: Fault) -> Result<usize, String> {
    let mut current = target;
    for iteration in 1..=2 {
        let next = rerun_transition(current, fault);
        if next == current {
            return Ok(iteration);
        }
        current = next;
    }
    Err(String::from("DIG-MODEL-IDEMPOTENCE"))
}

fn verify_digest_space(fault: Fault) -> Result<Counts, String> {
    let mut counts = Counts::default();
    for index in 0..DIGEST_STATES {
        let state = digest_state(index);
        let modeled = model_digest_acceptance(state);
        let implemented = implementation_digest_acceptance(state, fault);
        if modeled != implemented {
            return Err(format!(
                "DIG-MODEL-DIVERGENCE: space=digest state={index:#04x} model={modeled:?} implementation={implemented:?}"
            ));
        }
        if implemented.is_ok() {
            counts.accepted += 1;
        } else {
            counts.rejected += 1;
        }
    }
    Ok(counts)
}

fn verify_active_space(fault: Fault) -> Result<Counts, String> {
    let mut counts = Counts::default();
    for index in 0..ACTIVE_STATES {
        let state = active_state(index);
        let modeled = model_active_resolution(state);
        let implemented = implementation_active_resolution(state, fault);
        if modeled != implemented {
            return Err(format!(
                "DIG-MODEL-DIVERGENCE: space=active state={index:#04x} model={modeled:?} implementation={implemented:?}"
            ));
        }
        if implemented.is_ok() {
            counts.accepted += 1;
        } else {
            counts.rejected += 1;
        }
    }
    Ok(counts)
}

fn verify_transition_space(fault: Fault, cutoff: usize) -> Result<Counts, String> {
    if cutoff != TRANSITION_STATES {
        return Err(format!(
            "DIG-MODEL-CUTOFF: explored={cutoff} declared={TRANSITION_STATES}"
        ));
    }
    let mut counts = Counts::default();
    for mask in 0..cutoff {
        let state = transition_state(mask);
        let modeled = model_identity_transition(state);
        let implemented = implementation_identity_transition(state, fault);
        if let Ok(target) = implemented {
            if !state.authoritative_bytes || !state.approval {
                return Err(format!(
                    "DIG-MODEL-REGENERATION-SOUNDNESS: state={mask:#05x}"
                ));
            }
            if target != canonical_target() {
                return Err(format!(
                    "DIG-MODEL-PRESERVATION: state={mask:#05x} target={target:?}"
                ));
            }
            let iterations = fixed_point_iterations(target, fault)
                .map_err(|_| format!("DIG-MODEL-IDEMPOTENCE: state={mask:#05x}"))?;
            counts.fixed_points += 1;
            counts.max_fixed_point_iterations = counts.max_fixed_point_iterations.max(iterations);
        }
        let state_without_index = without_index(state);
        let model_without_index =
            if matches!(fault, Fault::ModelIndexAuthority) && state.index_present {
                Err(Diagnostic::MixedMember)
            } else {
                model_identity_transition(state_without_index)
            };
        if modeled != model_without_index {
            return Err(format!(
                "DIG-MODEL-INDEX-AUTHORITY: path=model state={mask:#05x} with-index={modeled:?} without-index={model_without_index:?}"
            ));
        }
        let implementation_without_index =
            implementation_identity_transition(state_without_index, fault);
        if implemented != implementation_without_index {
            return Err(format!(
                "DIG-MODEL-INDEX-AUTHORITY: path=implementation state={mask:#05x} with-index={implemented:?} without-index={implementation_without_index:?}"
            ));
        }
        if modeled != implemented {
            return Err(format!(
                "DIG-MODEL-DIVERGENCE: invariant=transition state={mask:#05x} model={modeled:?} implementation={implemented:?}"
            ));
        }
        let model_index = model_index_verification(state);
        let implementation_index = implementation_index_verification(state, fault);
        if model_index != implementation_index {
            return Err(format!(
                "DIG-MODEL-INDEX-VERIFICATION: state={mask:#05x} model={model_index:?} implementation={implementation_index:?}"
            ));
        }
        if implemented.is_ok() {
            counts.accepted += 1;
        } else {
            counts.rejected += 1;
        }
    }
    Ok(counts)
}

fn validate_model_contract() -> Result<(), String> {
    let source = fs::read(MODEL_CONTRACT)
        .map_err(|error| format!("DIG-MODEL-CONTRACT: {MODEL_CONTRACT}: {error}"))?;
    validate_contract_bytes(&source, MODEL_REVISION)
}

fn validate_contract_bytes(source: &[u8], expected_revision: &str) -> Result<(), String> {
    let actual_revision = digest_binding::bytes_binding(source);
    if actual_revision != expected_revision {
        return Err(format!(
            "DIG-MODEL-REVISION: expected={expected_revision} actual={actual_revision}"
        ));
    }
    let text =
        std::str::from_utf8(source).map_err(|error| format!("DIG-MODEL-CONTRACT: {error}"))?;
    for required in [
        "model_id=buoy-digest-transition/v1",
        "state_spaces=digest-acceptance:64,active-member-resolution:12,transition-index-lifecycle:4096",
        "bounds=tag-class:4,payload-length:2,text-case:2,hex-validity:2,operation:2,declared-count:3,registration:2,feature:2,transition-bits:12",
        "fixed_point=accepted-target-rerun-until-equal,max-iterations:2",
        "claim_scope=model-internal-exhaustive-only",
        "production_correspondence=separate-focused-controls",
    ] {
        if !text.lines().any(|line| line == required) {
            return Err(format!("DIG-MODEL-CONTRACT: missing `{required}`"));
        }
    }
    Ok(())
}

fn verify(cutoff: usize) -> Result<(Counts, Counts, Counts), String> {
    validate_model_contract().and_then(|()| {
        verify_digest_space(Fault::None).and_then(|digest| {
            verify_active_space(Fault::None).and_then(|active| {
                verify_transition_space(Fault::None, cutoff)
                    .map(|transition| (digest, active, transition))
            })
        })
    })
}

fn report(cutoff: usize) -> Result<(), String> {
    verify(cutoff).and_then(report_counts)
}

fn report_counts((digest, active, transition): (Counts, Counts, Counts)) -> Result<(), String> {
    if (digest.accepted, active.accepted, transition.accepted) != (3, 1, 16) {
        return Err(format!(
            "DIG-MODEL-COUNT: digest={} active={} transition={}",
            digest.accepted, active.accepted, transition.accepted
        ));
    }
    let accepted = digest.accepted + active.accepted + transition.accepted;
    let rejected = digest.rejected + active.rejected + transition.rejected;
    let fixed_point = transition.fixed_points == transition.accepted;
    println!(
        "digest-transition-model: model-id={MODEL_ID} revision={MODEL_REVISION} contract={MODEL_CONTRACT} event-alphabet=parse,compute,resolve-active,regenerate,verify-index,delete-index transition-relation=independent-model-vs-separate-implementation-relation claim-scope=model-internal-exhaustive-only production-correspondence=separate-focused-controls invariants=regeneration-soundness,index-non-authority,index-verification,preservation,idempotence"
    );
    println!(
        "digest-transition-model: exhaustively-checked states={TOTAL_STATES} accepted={accepted} rejected={rejected} digest-states={DIGEST_STATES} digest-accepted={} digest-rejected={} active-states={ACTIVE_STATES} active-accepted={} active-rejected={} transition-states={TRANSITION_STATES} transition-accepted={} transition-rejected={} fixed-points={} fixed-point={fixed_point} iterations={}",
        digest.accepted,
        digest.rejected,
        active.accepted,
        active.rejected,
        transition.accepted,
        transition.rejected,
        transition.fixed_points,
        transition.max_fixed_point_iterations
    );
    Ok(())
}

fn expect_control(fault: Fault, expected: &str) {
    let error = verify_transition_space(fault, TRANSITION_STATES)
        .expect_err("injected transition fault must fail");
    assert!(error.contains(expected), "fault {fault:?}: {error}");
    assert!(error.contains("state=0x"), "fault {fault:?}: {error}");
}

#[allow(clippy::unnecessary_wraps)]
fn self_test() -> Result<(), String> {
    report(TRANSITION_STATES).expect("the production model passes");
    let invalid_active = ActiveState {
        declared_count: 3,
        registered: true,
        feature_available: true,
    };
    assert_eq!(
        implementation_active_resolution(invalid_active, Fault::None),
        Err(Diagnostic::ActiveMultiple)
    );
    let digest_keyed_index = TransitionState {
        authoritative_bytes: true,
        approval: true,
        index_present: true,
        index_digest_keyed: true,
        ..transition_state(0)
    };
    assert_eq!(
        implementation_index_verification(digest_keyed_index, Fault::IndexContradictionAccepted),
        IndexVerdict::Valid
    );
    let digest_error =
        verify_digest_space(Fault::DigestDivergence).expect_err("injected digest fault must fail");
    assert!(digest_error.contains("DIG-MODEL-DIVERGENCE"));
    let active_error = verify_active_space(Fault::ActiveDivergence)
        .expect_err("injected active-member fault must fail");
    assert!(active_error.contains("DIG-MODEL-DIVERGENCE"));
    expect_control(Fault::ModelIndexAuthority, "DIG-MODEL-INDEX-AUTHORITY");
    expect_control(Fault::IndexInfluencesIdentity, "DIG-MODEL-INDEX-AUTHORITY");
    expect_control(
        Fault::IndexContradictionAccepted,
        "DIG-MODEL-INDEX-VERIFICATION",
    );
    expect_control(
        Fault::MissingInputAccepted,
        "DIG-MODEL-REGENERATION-SOUNDNESS",
    );
    expect_control(
        Fault::MissingApprovalAccepted,
        "DIG-MODEL-REGENERATION-SOUNDNESS",
    );
    expect_control(Fault::MutatedStableId, "DIG-MODEL-PRESERVATION");
    expect_control(Fault::NonIdempotent, "DIG-MODEL-IDEMPOTENCE");
    let cutoff_error = verify_transition_space(Fault::None, TRANSITION_STATES - 1)
        .expect_err("undeclared cutoff must fail");
    assert!(cutoff_error.contains("DIG-MODEL-CUTOFF"));
    expect_control(Fault::TransitionDivergence, "DIG-MODEL-DIVERGENCE");
    let contract = fs::read(MODEL_CONTRACT).expect("the model contract is readable");
    assert!(
        validate_contract_bytes(b"tampered", MODEL_REVISION)
            .expect_err("a tampered contract must fail")
            .starts_with("DIG-MODEL-REVISION")
    );
    let reduced = String::from_utf8(contract)
        .expect("the model contract is UTF-8")
        .lines()
        .filter(|line| *line != "claim_scope=model-internal-exhaustive-only")
        .collect::<Vec<_>>()
        .join("\n");
    let reduced_revision = digest_binding::bytes_binding(reduced.as_bytes());
    assert!(
        validate_contract_bytes(reduced.as_bytes(), &reduced_revision)
            .expect_err("a contract without a required line must fail")
            .starts_with("DIG-MODEL-CONTRACT")
    );
    let non_utf8 = [0xff_u8];
    assert!(
        validate_contract_bytes(&non_utf8, &digest_binding::bytes_binding(&non_utf8))
            .expect_err("a non-UTF-8 contract must fail")
            .starts_with("DIG-MODEL-CONTRACT")
    );
    assert!(
        report_counts((Counts::default(), Counts::default(), Counts::default()))
            .expect_err("wrong space counts must fail")
            .starts_with("DIG-MODEL-COUNT")
    );
    for (value, expected) in [
        ("model-index-authority", "DIG-MODEL-INDEX-AUTHORITY"),
        (
            "implementation-index-authority",
            "DIG-MODEL-INDEX-AUTHORITY",
        ),
        ("index-verification", "DIG-MODEL-INDEX-VERIFICATION"),
        ("missing-input", "DIG-MODEL-REGENERATION-SOUNDNESS"),
        ("missing-approval", "DIG-MODEL-REGENERATION-SOUNDNESS"),
        ("preservation", "DIG-MODEL-PRESERVATION"),
        ("idempotence", "DIG-MODEL-IDEMPOTENCE"),
    ] {
        let error = run_fault_control(value).expect_err("a fault control must fail");
        assert!(error.contains(expected), "fault control {value}: {error}");
    }
    assert!(
        run_fault_control("unknown")
            .expect_err("an unknown fault name must fail")
            .starts_with("DIG-MODEL-CONTROL")
    );
    assert!(
        fault_control_result(String::from("no marker"), "DIG-MODEL-PRESERVATION")
            .expect_err("an unexpected control result must fail")
            .starts_with("DIG-MODEL-CONTROL")
    );
    assert!(
        fault_control_result(
            String::from("DIG-MODEL-PRESERVATION without a state marker"),
            "DIG-MODEL-PRESERVATION",
        )
        .expect_err("a control result without a state marker must fail")
        .starts_with("DIG-MODEL-CONTROL")
    );
    println!("digest-transition-model: self-test ok controls=28");
    Ok(())
}

fn run_fault_control(value: &str) -> Result<(), String> {
    let (fault, expected) = match value {
        "model-index-authority" => (Fault::ModelIndexAuthority, "DIG-MODEL-INDEX-AUTHORITY"),
        "implementation-index-authority" => {
            (Fault::IndexInfluencesIdentity, "DIG-MODEL-INDEX-AUTHORITY")
        }
        "index-verification" => (
            Fault::IndexContradictionAccepted,
            "DIG-MODEL-INDEX-VERIFICATION",
        ),
        "missing-input" => (
            Fault::MissingInputAccepted,
            "DIG-MODEL-REGENERATION-SOUNDNESS",
        ),
        "missing-approval" => (
            Fault::MissingApprovalAccepted,
            "DIG-MODEL-REGENERATION-SOUNDNESS",
        ),
        "preservation" => (Fault::MutatedStableId, "DIG-MODEL-PRESERVATION"),
        "idempotence" => (Fault::NonIdempotent, "DIG-MODEL-IDEMPOTENCE"),
        _ => return Err(format!("DIG-MODEL-CONTROL: unknown fault `{value}`")),
    };
    let error = verify_transition_space(fault, TRANSITION_STATES)
        .expect_err("an injected model fault must fail");
    fault_control_result(error, expected)
}

fn fault_control_result(error: String, expected: &str) -> Result<(), String> {
    if !error.contains(expected) || !error.contains("state=0x") {
        return Err(format!("DIG-MODEL-CONTROL: unexpected result `{error}`"));
    }
    Err(error)
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => report(TRANSITION_STATES),
        [argument] if argument == "--self-test" => self_test(),
        [argument, value] if argument == "--fault" => run_fault_control(value),
        [argument, value] if argument == "--cutoff" => {
            let cutoff = value
                .parse::<usize>()
                .map_err(|_| String::from("DIG-MODEL-CUTOFF: cutoff is not an integer"))?;
            report(cutoff)
        }
        _ => Err(String::from(
            "usage: check-digest-transition-model [--self-test | --fault NAME | --cutoff COUNT]",
        )),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("digest-transition-model: {error}");
        std::process::exit(1);
    }
}
