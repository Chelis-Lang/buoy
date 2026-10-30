use vstd::prelude::*;

verus! {

spec fn lifecycle_transition_is_live(state: int, event: int) -> bool {
    (state == 0 && event == 0) || (state == 0 && event == 1)
        || (state == 1 && event == 1) || (state == 1 && event == 2)
}

proof fn lifecycle_terminality(state: int, event: int)
    requires 2 <= state && state <= 3,
    ensures !lifecycle_transition_is_live(state, event),
{
}

spec fn surface_complete(
    domain_closed: bool,
    member_a_exact: bool,
    member_b_exact: bool,
    authority_a: bool,
    authority_b: bool,
) -> bool {
    domain_closed && member_a_exact && member_b_exact && authority_a && authority_b
}

proof fn surface_missing_member_is_incomplete(
    domain_closed: bool,
    member_a_exact: bool,
    authority_a: bool,
    authority_b: bool,
)
    ensures !surface_complete(domain_closed, member_a_exact, false, authority_a, authority_b),
{
}

spec fn carrier_fresh(
    atom: bool,
    oracle: bool,
    configuration: bool,
    corpus: bool,
    item: bool,
) -> bool {
    atom && oracle && configuration && corpus && item
}

spec fn derivation_fresh(
    source: bool,
    target: bool,
    tool: bool,
    configuration: bool,
    oracle: bool,
    consistency_passed: bool,
) -> bool {
    source && target && tool && configuration && oracle && consistency_passed
}

proof fn stale_bindings_prevent_freshness(
    oracle: bool,
    configuration: bool,
    corpus: bool,
    item: bool,
    source: bool,
    tool: bool,
    derivation_configuration: bool,
    derivation_oracle: bool,
    consistency_passed: bool,
)
    ensures
        !carrier_fresh(false, oracle, configuration, corpus, item),
        !derivation_fresh(
            source,
            false,
            tool,
            derivation_configuration,
            derivation_oracle,
            consistency_passed,
        ),
{
}

spec fn policy_monotonic(
    base_a: bool,
    base_b: bool,
    head_a: bool,
    head_b: bool,
) -> bool {
    (!base_a || head_a) && (!base_b || head_b)
}

proof fn policy_growth_is_monotonic(base_a: bool, base_b: bool, added_a: bool, added_b: bool)
    ensures policy_monotonic(base_a, base_b, base_a || added_a, base_b || added_b),
{
}

spec fn impact_closed(
    carrier: bool,
    surface: bool,
    derivation: bool,
    proof: bool,
) -> bool {
    carrier && surface && derivation && proof
}

proof fn missing_impact_prevents_closure(carrier: bool, surface: bool, proof: bool)
    ensures !impact_closed(carrier, surface, false, proof),
{
}

spec fn aggregate_tested(
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

proof fn process_records_do_not_launder(
    technical: bool,
    carrier_current: bool,
    verdict_passed: bool,
    debt: bool,
    waiver: bool,
    reaffirmation: bool,
)
    ensures
        aggregate_tested(
            technical,
            carrier_current,
            verdict_passed,
            debt,
            waiver,
            reaffirmation,
        ) == aggregate_tested(
            technical,
            carrier_current,
            verdict_passed,
            false,
            false,
            false,
        ),
{
}

spec fn field_identity(tag: int, length: int) -> (int, int) {
    (tag, length)
}

proof fn field_tag_is_unambiguous(tag_a: int, tag_b: int, length: int)
    requires tag_a != tag_b,
    ensures field_identity(tag_a, length) != field_identity(tag_b, length),
{
}

proof fn field_length_is_unambiguous(tag: int, length_a: int, length_b: int)
    requires length_a != length_b,
    ensures field_identity(tag, length_a) != field_identity(tag, length_b),
{
}

spec fn versioned_envelope_valid(schema: int, encoding: int, algorithm: int) -> bool {
    schema == 3 && encoding == 3 && (algorithm == 2 || algorithm == 3 || algorithm == 4)
}

proof fn versioned_envelope_binds_v3_registry()
    ensures
        versioned_envelope_valid(3, 3, 2),
        versioned_envelope_valid(3, 3, 3),
        versioned_envelope_valid(3, 3, 4),
        !versioned_envelope_valid(2, 3, 2),
        !versioned_envelope_valid(3, 2, 2),
        !versioned_envelope_valid(3, 3, 1),
        !versioned_envelope_valid(3, 3, 5),
{
}

spec fn canonical_xxh128_byte(high_be: Seq<int>, low_be: Seq<int>, index: int) -> int
    recommends high_be.len() == 8, low_be.len() == 8, 0 <= index < 16,
{
    if index < 8 { high_be[index] } else { low_be[index - 8] }
}

proof fn xxh128_half_and_byte_order(
    high_be: Seq<int>,
    low_be: Seq<int>,
    index: int,
)
    requires high_be.len() == 8, low_be.len() == 8, 0 <= index < 16,
    ensures
        index < 8 ==> canonical_xxh128_byte(high_be, low_be, index) == high_be[index],
        8 <= index ==> canonical_xxh128_byte(high_be, low_be, index) == low_be[index - 8],
{
}

spec fn digest_shape_valid(type_tag: int, algorithm_tag: int, length: int) -> bool {
    type_tag == 12
        && ((algorithm_tag == 2 && length == 16)
            || ((algorithm_tag == 3 || algorithm_tag == 4) && length == 32))
}

proof fn parser_length_and_tag_invariants(type_tag: int, algorithm_tag: int, length: int)
    ensures
        digest_shape_valid(type_tag, algorithm_tag, length)
            <==> (type_tag == 12
                && ((algorithm_tag == 2 && length == 16)
                    || ((algorithm_tag == 3 || algorithm_tag == 4) && length == 32))),
{
}

spec fn migrated_lifecycle(state: int) -> int {
    state
}

proof fn migration_lifecycle_wrapper_preserves_state(state: int)
    requires 0 <= state <= 3,
    ensures migrated_lifecycle(state) == state,
{
}

fn main() {
}

} // verus!
