#![forbid(unsafe_code)]

#[path = "../logical_mounts.rs"]
pub mod logical_mounts;

pub enum Lifecycle {
    Proposed,
    Ratified,
    Superseded,
    Withdrawn,
}

pub enum Event {
    Ratify,
    Withdraw,
    Supersede,
}

pub fn transition(state: Lifecycle, event: Event) -> Option<Lifecycle> {
    match (state, event) {
        (Lifecycle::Proposed, Event::Ratify) => Some(Lifecycle::Ratified),
        (Lifecycle::Proposed | Lifecycle::Ratified, Event::Withdraw) => Some(Lifecycle::Withdrawn),
        (Lifecycle::Ratified, Event::Supersede) => Some(Lifecycle::Superseded),
        _ => None,
    }
}

pub fn surface_complete(
    domain_closed: bool,
    member_a_exact: bool,
    member_b_exact: bool,
    authority_a: bool,
    authority_b: bool,
) -> bool {
    domain_closed && member_a_exact && member_b_exact && authority_a && authority_b
}

pub fn carrier_fresh(
    atom: bool,
    oracle: bool,
    configuration: bool,
    corpus: bool,
    item: bool,
) -> bool {
    atom && oracle && configuration && corpus && item
}

pub fn derivation_fresh(
    source: bool,
    target: bool,
    tool: bool,
    configuration: bool,
    oracle: bool,
    consistency_passed: bool,
) -> bool {
    source && target && tool && configuration && oracle && consistency_passed
}

pub fn policy_monotonic(base_a: bool, base_b: bool, head_a: bool, head_b: bool) -> bool {
    (!base_a || head_a) && (!base_b || head_b)
}

pub fn impact_closed(carrier: bool, surface: bool, derivation: bool, proof: bool) -> bool {
    carrier && surface && derivation && proof
}

pub fn aggregate_tested(
    technical: bool,
    carrier_current: bool,
    verdict_passed: bool,
    debt: bool,
    waiver: bool,
    reaffirmation: bool,
) -> bool {
    let _process_only = debt || waiver || reaffirmation;
    technical && carrier_current && verdict_passed
}

pub struct FieldHeader {
    pub tag: u8,
    pub length: u64,
}

pub fn field_header(tag: u8, length: u64) -> FieldHeader {
    FieldHeader { tag, length }
}

pub fn versioned_envelope_valid(schema: u8, encoding: u8, algorithm: u8) -> bool {
    schema == 3 && encoding == 3 && (algorithm == 2 || algorithm == 3 || algorithm == 4)
}

pub struct Octets {
    pub b0: u8,
    pub b1: u8,
    pub b2: u8,
    pub b3: u8,
    pub b4: u8,
    pub b5: u8,
    pub b6: u8,
    pub b7: u8,
}

pub struct Digest128Bytes {
    pub b0: u8,
    pub b1: u8,
    pub b2: u8,
    pub b3: u8,
    pub b4: u8,
    pub b5: u8,
    pub b6: u8,
    pub b7: u8,
    pub b8: u8,
    pub b9: u8,
    pub b10: u8,
    pub b11: u8,
    pub b12: u8,
    pub b13: u8,
    pub b14: u8,
    pub b15: u8,
}

pub fn canonical_xxh128_bytes(upstream_low_be: Octets, upstream_high_be: Octets) -> Digest128Bytes {
    Digest128Bytes {
        b0: upstream_high_be.b0,
        b1: upstream_high_be.b1,
        b2: upstream_high_be.b2,
        b3: upstream_high_be.b3,
        b4: upstream_high_be.b4,
        b5: upstream_high_be.b5,
        b6: upstream_high_be.b6,
        b7: upstream_high_be.b7,
        b8: upstream_low_be.b0,
        b9: upstream_low_be.b1,
        b10: upstream_low_be.b2,
        b11: upstream_low_be.b3,
        b12: upstream_low_be.b4,
        b13: upstream_low_be.b5,
        b14: upstream_low_be.b6,
        b15: upstream_low_be.b7,
    }
}

pub fn digest_shape_valid(type_tag: u8, algorithm_tag: u8, length: u8) -> bool {
    type_tag == 12
        && ((algorithm_tag == 2 && length == 16)
            || ((algorithm_tag == 3 || algorithm_tag == 4) && length == 32))
}

pub fn migrated_lifecycle(state: Lifecycle) -> Lifecycle {
    match state {
        Lifecycle::Proposed => Lifecycle::Proposed,
        Lifecycle::Ratified => Lifecycle::Ratified,
        Lifecycle::Superseded => Lifecycle::Superseded,
        Lifecycle::Withdrawn => Lifecycle::Withdrawn,
    }
}
