#[spec_provenance::carrier(
    syntax = "v1",
    body = r#"
id = "CAR-ADAPTER-001"
role = "positive"
atoms = [{ id = "SPG-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" }]
oracle = { kind = "oracle", id = "ORC-ADAPTER-001", digest = "xxh3-128:22222222222222222222222222222222" }
configuration = "xxh3-128:33333333333333333333333333333333"
corpora = ["xxh3-128:44444444444444444444444444444444"]
scope = { schema = "spec-provenance.scope.exact-corpus/v1", bytes_hex = "636173652d61" }
assumptions = [{ kind = "assumption", id = "ASSUMPTION-ADAPTER-001", digest = "xxh3-128:55555555555555555555555555555555" }]
exclusions = ["Network behavior is excluded."]
"#
)]
#[test]
fn adapter_positive() {}
