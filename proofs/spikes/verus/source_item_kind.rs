use vstd::prelude::*;

verus! {

spec fn source_item_kind_valid(path_components: nat, version: nat) -> bool {
    path_components >= 2 && version >= 1
}

spec fn parsed_item_fields(
    schema: int,
    path: int,
    source_item_kind: int,
    semantic_bytes: int,
) -> (int, int, int, int) {
    (schema, path, source_item_kind, semantic_bytes)
}

proof fn version_zero_is_not_a_source_item_kind(path_components: nat)
    ensures !source_item_kind_valid(path_components, 0),
{
}

proof fn parsed_item_identity_binds_source_item_kind(
    schema: int,
    path: int,
    left_kind: int,
    right_kind: int,
    semantic_bytes: int,
)
    requires left_kind != right_kind,
    ensures
        parsed_item_fields(schema, path, left_kind, semantic_bytes)
            != parsed_item_fields(schema, path, right_kind, semantic_bytes),
{
}

}

fn main() {}
