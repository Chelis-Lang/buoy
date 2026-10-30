use buoy_core::{ParsedItemIdentity, RepositoryPath, SchemaId, SourceItemKind};

fn construct_identity(kind: SourceItemKind) {
    let _ = ParsedItemIdentity::new(
        SchemaId::new("rust/v1").unwrap(),
        RepositoryPath::new("src/lib.rs").unwrap(),
        kind,
        Vec::new(),
        None,
    );
}

fn main() {
    construct_identity("rust/item/v1");
}
