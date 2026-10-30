use vstd::prelude::*;

verus! {

spec fn logical_projection(prefix: Seq<int>, relative: Seq<int>) -> Seq<int> {
    prefix + relative
}

proof fn invalid_equal_input_injectivity_claim(prefix: Seq<int>, relative: Seq<int>)
    ensures logical_projection(prefix, relative) != logical_projection(prefix, relative),
{
}

fn main() {
}

} // verus!
