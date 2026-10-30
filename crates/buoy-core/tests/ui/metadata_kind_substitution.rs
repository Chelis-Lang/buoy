#![allow(unreachable_code)]

use buoy_core::{EvidenceCarrier, EvidenceRole, ObjectMeta, SemanticDigest, StableId};

fn construct_carrier(id: StableId) {
    let _ = EvidenceCarrier::new(
        id,
        EvidenceRole::Positive,
        Vec::new(),
        SemanticDigest::xxh3_128([1; 16]),
        unreachable!(),
        SemanticDigest::xxh3_128([2; 16]),
        Vec::new(),
        unreachable!(),
        Vec::new(),
        Vec::new(),
    );
}

fn substitute_oracle_metadata_for_carrier(metadata: ObjectMeta) {
    construct_carrier(metadata);
}

fn main() {}
