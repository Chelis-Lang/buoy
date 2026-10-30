#![doc = "Deprecated no-std compatibility facade for `buoy_core`."]
#![no_std]
#![forbid(unsafe_code)]

pub use buoy_core::*;

#[cfg(test)]
mod tests {
    #[test]
    fn facade_reexports_the_primary_no_std_values() {
        let digest = crate::SemanticDigest::xxh3_128([8; 16]);
        assert_eq!(
            digest.canonical_text(),
            "xxh3-128:08080808080808080808080808080808"
        );
    }

    #[test]
    fn facade_reexports_the_typed_stable_identifier() {
        let identifier = crate::StableId::new("ATOM-1");
        assert_eq!(
            identifier.as_ref().map(crate::StableId::as_str),
            Ok("ATOM-1")
        );
        assert_eq!(
            crate::StableId::new("atom-1")
                .err()
                .map(buoy_core::IdentifierError::code),
            Some("SCF-CONSTRUCT-STABLE-ID")
        );
    }

    #[test]
    fn facade_reexports_the_remaining_identifier_newtypes() {
        assert_eq!(
            crate::ScopeId::new("spec.graph")
                .as_ref()
                .map(crate::ScopeId::as_str),
            Ok("spec.graph")
        );
        assert_eq!(
            crate::SchemaId::new("spec.graph/v1")
                .as_ref()
                .map(crate::SchemaId::as_str),
            Ok("spec.graph/v1")
        );
        assert_eq!(
            crate::NamespacePrefix::new("SPG")
                .as_ref()
                .map(crate::NamespacePrefix::as_str),
            Ok("SPG")
        );
    }
}
