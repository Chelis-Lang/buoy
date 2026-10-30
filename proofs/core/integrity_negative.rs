use vstd::prelude::*;

verus! {

spec fn tested(technical: bool, fresh: bool, selected: bool, passed: bool) -> bool {
    technical && fresh && selected && passed
}

proof fn invalid_waiver_satisfies_execution()
    ensures tested(false, false, false, true),
{
}

fn main() {
}

} // verus!
