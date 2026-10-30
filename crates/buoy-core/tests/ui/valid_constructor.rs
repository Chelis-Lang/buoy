use buoy_core::{ObjectKind, ObjectRef, SemanticDigest, StableId};

fn main() {
    let id = StableId::new("COMPILE-PASS").expect("valid stable ID");
    let reference = ObjectRef::new(ObjectKind::Atom, id, SemanticDigest::xxh3_128([1; 16]));
    assert_eq!(reference.id().as_str(), "COMPILE-PASS");
}
