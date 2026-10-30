use vstd::prelude::*;

verus! {

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

proof fn invalid_waiver_laundering_claim()
    ensures aggregate_tested(false, false, false, false, true, false),
{
}

fn main() {
}

} // verus!
