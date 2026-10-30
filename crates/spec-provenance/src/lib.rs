#![doc = "Deprecated compatibility facade for the Buoy library."]
#![forbid(unsafe_code)]

pub use buoy::*;

#[cfg(test)]
mod tests {
    #[test]
    fn facade_reexports_the_primary_core_without_a_second_implementation() {
        let digest = crate::core::SemanticDigest::xxh3_128([7; 16]);
        assert_eq!(
            digest.canonical_text(),
            "xxh3-128:07070707070707070707070707070707"
        );
    }
}
