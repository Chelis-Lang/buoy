#[spec_provenance::implementation(
    syntax = "v1",
    body = r#"
id = "IMPL-ADAPTER-001"
atoms = [{ id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" }]
navigation = "implements"
"#
)]
fn implementation_item() {}

#[spec_provenance::governed_surface(
    syntax = "v1",
    body = r#"
id = "SURFACE-ADAPTER-001"
member_key_schema = { schema = "adapter.member-key/v1", bytes_hex = "736368656d61" }
disposition_vocabulary = { schema = "adapter.disposition/v1", bytes_hex = "766f636162756c617279" }
member_keys = [{ schema = "adapter.member/v1", bytes_hex = "61" }]
generation = 1
policies = []
dispositions = [{ key = { schema = "adapter.member/v1", bytes_hex = "61" }, disposition = "normative", payload = { schema = "adapter.disposition/v1", bytes_hex = "6e6f726d6174697665" }, authority = { id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" } }]
"#
)]
const SURFACE: &[&str] = &["a"];

#[spec_provenance::derivation(
    syntax = "v1",
    body = r#"
id = "DERIV-ADAPTER-001"
sources = [{ kind = "repository-artifact", path = "src/input.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "xxh3-128:18181818181818181818181818181818" }]
targets = [{ kind = "repository-artifact", path = "generated/output.rs", raw_domain = "raw-artifact", media_type = "text/x-rust", representation = "saved-bytes/v1", digest = "xxh3-128:19191919191919191919191919191919" }]
tool = { id = "adapter-generator", version = "1.0.0", executable = "xxh3-128:20202020202020202020202020202020", configuration = "xxh3-128:21212121212121212121212121212121" }
configuration = "xxh3-128:22222222222222222222222222222222"
oracle = { kind = "oracle", id = "ORACLE-ADAPTER-001", digest = "xxh3-128:23232323232323232323232323232323" }
relation = "generated-from"
relation_scope = { schema = "adapter.relation-scope/v1", bytes_hex = "6578616374" }
exclusions = ["Compiler backend behavior is excluded."]
"#
)]
const DERIVED: &str = "generated/output.rs";

#[spec_provenance::structural(
    syntax = "v1",
    body = r#"
id = "CAR-STRUCTURAL-001"
role = "structural"
atoms = [{ id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" }]
oracle = { kind = "oracle", id = "ORACLE-STRUCTURAL-001", digest = "xxh3-128:34343434343434343434343434343434" }
configuration = "xxh3-128:35353535353535353535353535353535"
corpora = []
scope = { schema = "adapter.structural-scope/v1", bytes_hex = "73796e746178" }
assumptions = []
exclusions = ["Runtime behavior is excluded."]
"#
)]
#[test]
fn structural_item() {}

#[spec_provenance::verifier(
    syntax = "v1",
    body = r#"
id = "CAR-VERIFIER-001"
role = "formal-property"
atoms = [{ id = "ATOM-ADAPTER-001", revision = "xxh3-128:11111111111111111111111111111111" }]
oracle = { kind = "oracle", id = "ORACLE-VERIFIER-001", digest = "xxh3-128:36363636363636363636363636363636" }
configuration = "xxh3-128:37373737373737373737373737373737"
corpora = ["xxh3-128:38383838383838383838383838383838"]
scope = { schema = "spec-provenance.scope.bounded-domain/v1", bytes_hex = "626f756e646564" }
assumptions = [{ kind = "assumption", id = "ASSUMPTION-ADAPTER-001", digest = "xxh3-128:55555555555555555555555555555555" }]
exclusions = ["Compiler backend behavior is excluded."]
"#
)]
fn verifier_item() {}
