//! Bounded reference models for publication transaction and dependency
//! safety, checked independently of the production pipeline.
//!
//! These models are pure, finite, and effect-free. They exist to prove
//! transition and dependency-acceptance properties by exhaustive
//! enumeration over their bounded state spaces, independent of the
//! effectful shell code in [`crate::publication`], [`crate::publication_fs`],
//! and [`crate::external_link_generation`].

#![allow(missing_docs)]

/// Bounded reference model `publication-transaction/v1`.
///
/// Covers every stage one publication operation can reach: parsing the
/// plan, deriving the retained snapshot, row conformance, candidate
/// construction, the pre-replacement source guard, the write itself,
/// failure, cancellation, and cleanup.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TransactionState {
    Parse,
    Snapshot,
    Conformance,
    Candidate,
    SourceGuard,
    Write,
    Failure,
    Cancellation,
    Cleanup,
    Committed,
}

impl TransactionState {
    /// Every state the bounded model declares, in a fixed order.
    pub const ALL: [Self; 10] = [
        Self::Parse,
        Self::Snapshot,
        Self::Conformance,
        Self::Candidate,
        Self::SourceGuard,
        Self::Write,
        Self::Failure,
        Self::Cancellation,
        Self::Cleanup,
        Self::Committed,
    ];

    /// The exact set of states this state may transition to.
    ///
    /// Every failure path (`Failure`, `Cancellation`) transitions only to
    /// `Cleanup`; only `Write` transitions to `Committed`. No transition
    /// skips `SourceGuard` on the way to `Write`, and no transition reaches
    /// `Conformance` without a captured `Snapshot`.
    #[must_use]
    pub const fn allowed_next(self) -> &'static [Self] {
        match self {
            Self::Parse => &[Self::Snapshot, Self::Failure],
            Self::Snapshot => &[Self::Conformance, Self::Failure],
            Self::Conformance => &[Self::Candidate, Self::Failure],
            Self::Candidate => &[Self::SourceGuard, Self::Failure],
            Self::SourceGuard => &[Self::Write, Self::Failure, Self::Cancellation],
            Self::Write => &[Self::Committed, Self::Failure, Self::Cancellation],
            Self::Failure | Self::Cancellation => &[Self::Cleanup],
            Self::Cleanup | Self::Committed => &[],
        }
    }

    /// Whether one declared transition is a member of this bounded model's
    /// allowed edge set.
    #[must_use]
    pub fn transition_is_allowed(self, next: Self) -> bool {
        self.allowed_next().contains(&next)
    }
}

/// Bounded reference model `publication-dependency/v1`.
///
/// Enumerates every identity kind a rendered path or target can depend on
/// and classifies each as accepted (registered, pre-link) or rejected
/// (would create a cycle back into the artifact this identity helps
/// produce).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DependencyKind {
    PublicationBasis,
    AtomRevision,
    RecordDigest,
    Graph,
    Page,
    Link,
    PageCommit,
    BranchHead,
    Provider,
    PhysicalPath,
}

impl DependencyKind {
    /// Every dependency kind the bounded model declares, in a fixed order.
    pub const ALL: [Self; 10] = [
        Self::PublicationBasis,
        Self::AtomRevision,
        Self::RecordDigest,
        Self::Graph,
        Self::Page,
        Self::Link,
        Self::PageCommit,
        Self::BranchHead,
        Self::Provider,
        Self::PhysicalPath,
    ];

    /// Whether this identity kind exists before external-link construction
    /// and may therefore appear in a rendered path or target.
    ///
    /// Every rejected kind depends on a value that can exist only after (or
    /// as a side effect of) the write the identity would help produce, so
    /// admitting it would create a publication cycle.
    #[must_use]
    pub const fn is_pre_link(self) -> bool {
        matches!(
            self,
            Self::PublicationBasis | Self::AtomRevision | Self::RecordDigest
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{DependencyKind, TransactionState};

    #[test]
    fn every_transaction_state_has_a_reachability_path_to_a_terminal_state() {
        // Terminal states are exactly those with no outgoing transition.
        let terminals = TransactionState::ALL
            .iter()
            .copied()
            .filter(|state| state.allowed_next().is_empty())
            .collect::<Vec<_>>();
        assert_eq!(
            terminals,
            [TransactionState::Cleanup, TransactionState::Committed]
        );

        for state in TransactionState::ALL {
            let mut visited = std::collections::BTreeSet::new();
            let mut frontier = vec![state];
            let mut reached_terminal = terminals.contains(&state);
            while let Some(current) = frontier.pop() {
                if !visited.insert(current) {
                    continue;
                }
                for next in current.allowed_next() {
                    if terminals.contains(next) {
                        reached_terminal = true;
                    }
                    frontier.push(*next);
                }
            }
            assert!(
                reached_terminal,
                "{state:?} must have a bounded path to a terminal state"
            );
        }
    }

    #[test]
    fn write_is_unreachable_without_passing_through_source_guard() {
        // Exhaustively confirm no state other than SourceGuard declares a
        // direct edge into Write.
        for state in TransactionState::ALL {
            if state == TransactionState::SourceGuard {
                continue;
            }
            assert!(
                !state.transition_is_allowed(TransactionState::Write),
                "{state:?} must not transition directly into Write"
            );
        }
    }

    #[test]
    fn failure_and_cancellation_transition_only_to_cleanup() {
        for state in [TransactionState::Failure, TransactionState::Cancellation] {
            assert_eq!(state.allowed_next(), [TransactionState::Cleanup]);
        }
    }

    #[test]
    fn committed_is_reachable_only_from_write() {
        for state in TransactionState::ALL {
            if state == TransactionState::Write {
                continue;
            }
            assert!(!state.transition_is_allowed(TransactionState::Committed));
        }
        assert!(TransactionState::Write.transition_is_allowed(TransactionState::Committed));
    }

    #[test]
    fn transaction_state_transitions_are_exhaustively_bounded() {
        // No state's allowed-next set contains a duplicate or itself, and
        // every listed successor is itself a declared state.
        for state in TransactionState::ALL {
            let next = state.allowed_next();
            let mut seen = std::collections::BTreeSet::new();
            for successor in next {
                assert!(*successor != state, "{state:?} must not self-loop");
                assert!(
                    seen.insert(*successor),
                    "{state:?} must not declare a duplicate transition"
                );
                assert!(TransactionState::ALL.contains(successor));
            }
        }
    }

    #[test]
    fn exactly_the_registered_kinds_are_pre_link() {
        let accepted = DependencyKind::ALL
            .iter()
            .copied()
            .filter(|kind| kind.is_pre_link())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            accepted,
            std::collections::BTreeSet::from([
                DependencyKind::PublicationBasis,
                DependencyKind::AtomRevision,
                DependencyKind::RecordDigest,
            ])
        );
    }

    #[test]
    fn every_cycle_prone_kind_is_rejected() {
        for kind in [
            DependencyKind::Graph,
            DependencyKind::Page,
            DependencyKind::Link,
            DependencyKind::PageCommit,
            DependencyKind::BranchHead,
            DependencyKind::Provider,
            DependencyKind::PhysicalPath,
        ] {
            assert!(
                !kind.is_pre_link(),
                "{kind:?} must be rejected as a publication-cycle dependency"
            );
        }
    }
}
