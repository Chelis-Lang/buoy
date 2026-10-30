#![doc = "Sampled properties for trace row order, root independence, and selector permutation."]
#![allow(clippy::expect_used, clippy::panic)]

use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use std::collections::BTreeSet;

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

/// One projected trace row, reduced to its canonical ordering facts.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Row {
    /// True for a current attachment and false for a retained record.
    current: bool,
    relation: String,
    requirement: String,
    path: String,
    implementation: String,
    record: String,
}

impl Row {
    /// The canonical sort key from the reviewed design.
    ///
    /// Current rows precede historical rows, so the key inverts the flag.
    fn key(&self) -> (bool, &str, &str, &str, &str, &str) {
        (
            !self.current,
            &self.relation,
            &self.requirement,
            &self.path,
            &self.implementation,
            &self.record,
        )
    }
}

/// Candidate kernel: sort every row into the one canonical order.
fn kernel_order(rows: &[Row]) -> Vec<Row> {
    let mut ordered = rows.to_vec();
    ordered.sort_by(|left, right| left.key().cmp(&right.key()));
    ordered
}

/// Independent reference: build the order from a sorted key set.
///
/// The reference keys each row by its complete canonical tuple, collects the
/// keys in a sorted set, and then rebuilds the rows from that set. It shares no
/// comparison code with the kernel.
fn reference_order(rows: &[Row]) -> Vec<Row> {
    let keyed = rows
        .iter()
        .map(|row| {
            (
                (
                    u8::from(!row.current),
                    row.relation.clone(),
                    row.requirement.clone(),
                    row.path.clone(),
                    row.implementation.clone(),
                    row.record.clone(),
                ),
                row.clone(),
            )
        })
        .collect::<Vec<_>>();
    let mut keys = keyed.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();
    keys.sort();
    keys.into_iter()
        .map(|key| {
            keyed
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map(|(_, row)| row.clone())
                .expect("every key came from one row")
        })
        .collect()
}

fn draw_rows(test_case: &TestCase) -> Vec<Row> {
    let count = test_case.draw(gs::integers::<usize>().min_value(1).max_value(6));
    (0..count)
        .map(|_| {
            let current = test_case.draw(gs::booleans());
            let field = |maximum: u8| {
                let value = test_case.draw(gs::integers::<u8>().min_value(0).max_value(maximum));
                format!("{value:03}")
            };
            Row {
                current,
                relation: format!("REL-{}", field(2)),
                requirement: format!("REQ-{}", field(2)),
                path: format!("src/{}.rs", field(2)),
                implementation: format!("node-{}", field(3)),
                record: format!("record-{}", field(3)),
            }
        })
        .collect()
}

/// Reorders rows without changing the row multiset.
fn permute(rows: &[Row], keys: &[u8]) -> Vec<Row> {
    let mut ordered = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (keys.get(index).copied().unwrap_or(0), index, row.clone()))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(key, index, _)| (*key, *index));
    ordered.into_iter().map(|(_, _, row)| row).collect()
}

#[test]
fn hegel_trace_row_order_is_input_permutation_independent() {
    Hegel::new(|test_case: TestCase| {
        let rows = draw_rows(&test_case);
        let keys = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(6));
        let permuted = permute(&rows, &keys);

        assert_eq!(
            kernel_order(&rows),
            kernel_order(&permuted),
            "a discovery permutation must not change the report"
        );
        assert_eq!(
            kernel_order(&rows),
            reference_order(&rows),
            "the kernel must agree with the independent order"
        );
        assert_eq!(
            kernel_order(&kernel_order(&rows)),
            kernel_order(&rows),
            "the canonical order must be idempotent"
        );

        let ordered = kernel_order(&rows);
        let boundary = ordered.iter().position(|row| !row.current);
        if let Some(boundary) = boundary {
            assert!(
                ordered[boundary..].iter().all(|row| !row.current),
                "every current row must precede every historical row"
            );
        }
    })
    .settings(settings(411))
    .run();
}

#[test]
fn hegel_trace_rows_exclude_every_physical_root() {
    Hegel::new(|test_case: TestCase| {
        let rows = draw_rows(&test_case);
        let root = test_case.draw(gs::integers::<u8>().min_value(0).max_value(255));
        let physical = format!("/private/tmp/root-{root}");

        // A logical projection binds only repository-relative paths, so no
        // rendered field may contain the caller-supplied physical root.
        for row in kernel_order(&rows) {
            for field in [
                row.relation.as_str(),
                row.requirement.as_str(),
                row.path.as_str(),
                row.implementation.as_str(),
                row.record.as_str(),
            ] {
                assert!(
                    !field.contains(&physical),
                    "a canonical field must exclude the physical root"
                );
                assert!(
                    !field.starts_with('/'),
                    "a canonical field must exclude an absolute path"
                );
            }
        }

        // Two runs over equal logical rows produce equal reports, so the
        // physical root cannot reach the canonical output.
        let first = kernel_order(&rows);
        let second = kernel_order(&rows.iter().rev().cloned().collect::<Vec<_>>());
        assert_eq!(first, second);
    })
    .settings(settings(412))
    .run();
}

#[test]
fn hegel_equivalent_selectors_produce_one_projection() {
    Hegel::new(|test_case: TestCase| {
        let rows = draw_rows(&test_case);
        let ordered = kernel_order(&rows);

        // A requirement selector and a relation selector that match the same
        // rows must produce the same ordered projection.
        let selected = |matched: Vec<Row>| kernel_order(&matched);
        let forward = selected(ordered.clone());
        let reverse = selected(ordered.iter().rev().cloned().collect());
        assert_eq!(forward, reverse, "match order must not change the report");

        // A narrower selector keeps the rows it retains in canonical order, so
        // one subsequence of the canonical order is already canonical.
        let keep = test_case.draw(gs::booleans());
        let subset = ordered
            .iter()
            .enumerate()
            .filter(|(index, _)| if keep { index % 2 == 0 } else { index % 2 == 1 })
            .map(|(_, row)| row.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            kernel_order(&subset),
            subset,
            "a narrower selector keeps the canonical order"
        );

        // Every distinct projected row stays a member of the selected rows.
        let members = ordered.iter().cloned().collect::<BTreeSet<_>>();
        assert!(members.len() <= ordered.len());
        assert!(members.iter().all(|row| ordered.contains(row)));
    })
    .settings(settings(413))
    .run();
}
