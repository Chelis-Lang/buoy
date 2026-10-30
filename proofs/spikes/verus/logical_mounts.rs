use vstd::prelude::*;

verus! {

spec fn logical_projection(prefix: Seq<int>, relative: Seq<int>) -> Seq<int> {
    prefix + relative
}

spec fn prefixes_are_disjoint(left: Seq<int>, right: Seq<int>) -> bool {
    exists |index: int|
        0 <= index
            && index < left.len()
            && index < right.len()
            && left[index] != right[index]
}

proof fn disjoint_prefixes_make_projection_injective(
    prefix_a: Seq<int>,
    prefix_b: Seq<int>,
    relative_a: Seq<int>,
    relative_b: Seq<int>,
)
    requires prefixes_are_disjoint(prefix_a, prefix_b),
    ensures logical_projection(prefix_a, relative_a)
        != logical_projection(prefix_b, relative_b),
{
    let index = choose |index: int|
        0 <= index
            && index < prefix_a.len()
            && index < prefix_b.len()
            && prefix_a[index] != prefix_b[index];
    assert(logical_projection(prefix_a, relative_a)[index] == prefix_a[index]);
    assert(logical_projection(prefix_b, relative_b)[index] == prefix_b[index]);
}

proof fn one_prefix_preserves_relative_identity(
    prefix: Seq<int>,
    relative_a: Seq<int>,
    relative_b: Seq<int>,
)
    requires relative_a != relative_b,
    ensures logical_projection(prefix, relative_a)
        != logical_projection(prefix, relative_b),
{
    if logical_projection(prefix, relative_a) == logical_projection(prefix, relative_b) {
        assert(
            logical_projection(prefix, relative_a).len()
                == prefix.len() + relative_a.len()
        );
        assert(
            logical_projection(prefix, relative_b).len()
                == prefix.len() + relative_b.len()
        );
        assert(relative_a.len() == relative_b.len());
        assert(
            logical_projection(prefix, relative_a).subrange(
                prefix.len() as int,
                (prefix.len() + relative_a.len()) as int,
            ) == relative_a
        );
        assert(
            logical_projection(prefix, relative_b).subrange(
                prefix.len() as int,
                (prefix.len() + relative_b.len()) as int,
            ) == relative_b
        );
    }
}

spec fn attachment_key_preimage(relation: int) -> (int, int) {
    (1, relation)
}

proof fn attachment_key_preimage_binds_the_relation(
    relation_a: int,
    relation_b: int,
)
    requires relation_a != relation_b,
    ensures attachment_key_preimage(relation_a)
        != attachment_key_preimage(relation_b),
{
}

fn main() {
}

} // verus!
