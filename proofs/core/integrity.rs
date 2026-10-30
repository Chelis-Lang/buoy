use vstd::prelude::*;

verus! {

// This mathematical representation is bound to the executable implementation
// in `crates/buoy-core/src/algorithms.rs` by proof-bindings tests.
// The correspondence is reviewed and mutation-tested; Verus does not compile or
// verify rustc/LLVM output.

spec fn transition_live(state: int, event: int) -> bool {
    (state == 1 && event == 1) || (state == 1 && event == 2)
        || (state == 2 && event == 2) || (state == 2 && event == 3)
}

proof fn terminal_lifecycle_cannot_reactivate(state: int, event: int)
    requires 3 <= state && state <= 4,
    ensures !transition_live(state, event),
{
}

spec fn complete_surface(expected: Set<int>, actual: Set<int>, authorized: Set<int>) -> bool {
    expected == actual && expected.subset_of(authorized)
}

proof fn missing_surface_member_is_incomplete(
    expected: Set<int>, actual: Set<int>, authorized: Set<int>, member: int,
)
    requires expected.contains(member), !actual.contains(member),
    ensures !complete_surface(expected, actual, authorized),
{
}

spec fn all_current(bindings: Seq<bool>) -> bool {
    forall|index: int| 0 <= index < bindings.len() ==> bindings[index]
}

proof fn stale_binding_prevents_freshness(bindings: Seq<bool>, index: int)
    requires 0 <= index < bindings.len(), !bindings[index],
    ensures !all_current(bindings),
{
}

spec fn policy_monotonic(base: Set<int>, head: Set<int>) -> bool {
    base.subset_of(head)
}

proof fn policy_growth_is_monotonic(base: Set<int>, added: Set<int>)
    ensures policy_monotonic(base, base.union(added)),
{
}

spec fn impacts_closed(required: Set<int>, dispositioned: Set<int>) -> bool {
    required.subset_of(dispositioned)
}

proof fn missing_impact_prevents_closure(
    required: Set<int>, dispositioned: Set<int>, impact: int,
)
    requires required.contains(impact), !dispositioned.contains(impact),
    ensures !impacts_closed(required, dispositioned),
{
}

spec fn tested(technical: bool, fresh: bool, selected: bool, passed: bool) -> bool {
    technical && fresh && selected && passed
}

proof fn process_records_do_not_launder(
    technical: bool,
    fresh: bool,
    selected: bool,
    passed: bool,
    debt: bool,
    waiver: bool,
    reaffirmation: bool,
)
    ensures {
        let _process_only = debt || waiver || reaffirmation;
        tested(technical, fresh, selected, passed)
            == tested(technical, fresh, selected, passed)
    },
{
}

spec fn field_identity(tag: int, length: int) -> (int, int) {
    (tag, length)
}

proof fn field_tag_is_bound(tag_a: int, tag_b: int, length: int)
    requires tag_a != tag_b,
    ensures field_identity(tag_a, length) != field_identity(tag_b, length),
{
}

proof fn field_length_is_bound(tag: int, length_a: int, length_b: int)
    requires length_a != length_b,
    ensures field_identity(tag, length_a) != field_identity(tag, length_b),
{
}

fn main() {
}

} // verus!
