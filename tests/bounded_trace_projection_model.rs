#![doc = "Independent bounded models for trace classification and digest abbreviation."]
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Model 1: current, historical, and stale record classification.
// ---------------------------------------------------------------------------

/// Closed classification of one retained implementation record.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RecordState {
    /// One current attachment selects the record and both sides are current.
    Current,
    /// One current attachment selects the record and one side is not current.
    Stale,
    /// No current attachment selects the record.
    Historical,
}

/// The exact observable facts that classify one retained record.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RecordFacts {
    /// A current source item declares this record locator.
    attached: bool,
    /// The exact implementation subject node is current.
    subject_current: bool,
    /// The exact requirement node is current.
    requirement_current: bool,
}

impl RecordFacts {
    fn all() -> Vec<Self> {
        let mut cases = Vec::new();
        for attached in [false, true] {
            for subject_current in [false, true] {
                for requirement_current in [false, true] {
                    cases.push(Self {
                        attached,
                        subject_current,
                        requirement_current,
                    });
                }
            }
        }
        cases
    }
}

/// Candidate kernel: an attachment decides currency, then each side decides staleness.
fn kernel_state(facts: RecordFacts) -> RecordState {
    if !facts.attached {
        return RecordState::Historical;
    }
    if facts.subject_current && facts.requirement_current {
        return RecordState::Current;
    }
    RecordState::Stale
}

/// Candidate kernel: the primary blocker for one stale side, in fixed order.
fn kernel_diagnostic(facts: RecordFacts) -> Option<&'static str> {
    match kernel_state(facts) {
        RecordState::Current | RecordState::Historical => None,
        RecordState::Stale if !facts.requirement_current => Some("CAB-REQUIREMENT-STALE"),
        RecordState::Stale => Some("CAB-SUBJECT-STALE"),
    }
}

/// Independent reference: one explicit truth table over the three facts.
///
/// The table is written from the delta specification, not from the kernel.
fn reference_state(facts: RecordFacts) -> RecordState {
    match (
        facts.attached,
        facts.subject_current,
        facts.requirement_current,
    ) {
        (false, _, _) => RecordState::Historical,
        (true, true, true) => RecordState::Current,
        (true, _, _) => RecordState::Stale,
    }
}

/// Independent reference: a requirement fault outranks a subject fault.
fn reference_diagnostic(facts: RecordFacts) -> Option<&'static str> {
    if reference_state(facts) != RecordState::Stale {
        return None;
    }
    if facts.requirement_current {
        Some("CAB-SUBJECT-STALE")
    } else {
        Some("CAB-REQUIREMENT-STALE")
    }
}

/// Candidate kernel: the exit class one selected record produces.
fn kernel_exit(facts: RecordFacts) -> u8 {
    match kernel_state(facts) {
        RecordState::Current | RecordState::Historical => 0,
        RecordState::Stale => 2,
    }
}

/// Candidate kernel: whether the projection emits this record.
fn kernel_emitted(facts: RecordFacts, include_history: bool) -> bool {
    kernel_state(facts) != RecordState::Historical || include_history
}

#[test]
fn exhaustive_record_classification_agrees_with_the_independent_table() {
    let mut checked = 0_u32;
    let mut states = BTreeSet::new();
    for facts in RecordFacts::all() {
        checked += 1;
        let state = kernel_state(facts);
        states.insert(state);
        assert_eq!(
            state,
            reference_state(facts),
            "classification disagrees for {facts:?}"
        );
        assert_eq!(
            kernel_diagnostic(facts),
            reference_diagnostic(facts),
            "primary diagnostic disagrees for {facts:?}"
        );

        // A historical record never carries a primary blocker, so it never
        // changes a successful exit class.
        if state == RecordState::Historical {
            assert_eq!(kernel_diagnostic(facts), None, "{facts:?}");
            assert_eq!(kernel_exit(facts), 0, "{facts:?}");
        }
        // A current record has both sides current and blocks nothing.
        if state == RecordState::Current {
            assert!(facts.subject_current && facts.requirement_current);
            assert_eq!(kernel_exit(facts), 0, "{facts:?}");
        }
        // A stale record always names exactly one primary blocker.
        if state == RecordState::Stale {
            assert!(kernel_diagnostic(facts).is_some(), "{facts:?}");
            assert_eq!(kernel_exit(facts), 2, "{facts:?}");
        }
    }
    assert_eq!(checked, 8, "the model must cover every fact combination");
    assert_eq!(states.len(), 3, "every classification must be reachable");
}

#[test]
fn history_selection_never_changes_a_successful_exit_class() {
    for facts in RecordFacts::all() {
        for include_history in [false, true] {
            let emitted = kernel_emitted(facts, include_history);
            if kernel_state(facts) == RecordState::Historical {
                assert_eq!(
                    emitted, include_history,
                    "a historical record appears only under explicit history selection"
                );
                assert_eq!(
                    kernel_exit(facts),
                    0,
                    "a historical record keeps a successful exit class"
                );
            } else {
                assert!(emitted, "a current attachment always appears");
            }
        }
    }
}

#[test]
fn an_old_record_never_becomes_current_without_a_new_attachment() {
    // A retained record with both sides current is still historical while no
    // current source item declares its locator.
    let retained = RecordFacts {
        attached: false,
        subject_current: true,
        requirement_current: true,
    };
    assert_eq!(kernel_state(retained), RecordState::Historical);
    assert_eq!(reference_state(retained), RecordState::Historical);

    let attached = RecordFacts {
        attached: true,
        ..retained
    };
    assert_eq!(kernel_state(attached), RecordState::Current);
}

// ---------------------------------------------------------------------------
// Model 2: human digest abbreviation and prefix collisions.
// ---------------------------------------------------------------------------

/// Bit width of one modeled digest.
const DIGEST_BITS: u32 = 4;
/// Number of leading bits the human view displays by default.
const DISPLAY_BITS: u32 = 2;

fn digest_text(value: u8) -> String {
    (0..DIGEST_BITS)
        .rev()
        .map(|bit| if value & (1 << bit) == 0 { '0' } else { '1' })
        .collect()
}

fn abbreviated(value: u8) -> String {
    digest_text(value)
        .chars()
        .take(DISPLAY_BITS as usize)
        .collect()
}

/// Candidate kernel: abbreviate unless two displayed records share the prefix.
fn kernel_display(displayed: &[u8]) -> Vec<String> {
    let distinct = displayed.iter().copied().collect::<BTreeSet<_>>();
    let prefixes = distinct
        .iter()
        .map(|value| abbreviated(*value))
        .collect::<BTreeSet<_>>();
    let collision = prefixes.len() != distinct.len();
    displayed
        .iter()
        .map(|value| {
            if collision {
                digest_text(*value)
            } else {
                abbreviated(*value)
            }
        })
        .collect()
}

/// Independent reference: search every distinct pair for a shared prefix.
fn reference_display(displayed: &[u8]) -> Vec<String> {
    let mut collision = false;
    for (left_index, left) in displayed.iter().enumerate() {
        for (right_index, right) in displayed.iter().enumerate() {
            if left_index == right_index || left == right {
                continue;
            }
            if abbreviated(*left) == abbreviated(*right) {
                collision = true;
            }
        }
    }
    displayed
        .iter()
        .map(|value| {
            if collision {
                digest_text(*value)
            } else {
                abbreviated(*value)
            }
        })
        .collect()
}

#[test]
fn exhaustive_digest_abbreviation_agrees_with_the_independent_search() {
    let limit = 1_u8 << DIGEST_BITS;
    let mut checked = 0_u32;
    let mut collisions = 0_u32;
    for first in 0..limit {
        for second in 0..limit {
            for third in 0..limit {
                for size in 1..=3_usize {
                    let displayed = [first, second, third][..size].to_vec();
                    checked += 1;
                    let kernel = kernel_display(&displayed);
                    assert_eq!(
                        kernel,
                        reference_display(&displayed),
                        "display disagrees for {displayed:?}"
                    );
                    if kernel.iter().any(|text| text.len() == DIGEST_BITS as usize) {
                        collisions += 1;
                        // A complete display must resolve every displayed record.
                        let distinct = displayed.iter().copied().collect::<BTreeSet<_>>();
                        let shown = kernel.iter().collect::<BTreeSet<_>>();
                        assert_eq!(shown.len(), distinct.len(), "{displayed:?}");
                    }
                }
            }
        }
    }
    assert_eq!(checked, 12_288);
    assert!(collisions > 0, "the model must reach a collision");
    assert!(
        collisions < checked,
        "the model must also reach the abbreviated case"
    );
}

#[test]
fn one_record_displayed_twice_is_not_a_collision() {
    let repeated = vec![0b1010_u8, 0b1010];
    assert_eq!(
        kernel_display(&repeated),
        vec![abbreviated(0b1010), abbreviated(0b1010)],
        "equal digests stay abbreviated"
    );
    let shared_prefix = vec![0b1010_u8, 0b1011];
    assert_eq!(
        kernel_display(&shared_prefix),
        vec![digest_text(0b1010), digest_text(0b1011)],
        "a shared prefix requires complete digests"
    );
}
