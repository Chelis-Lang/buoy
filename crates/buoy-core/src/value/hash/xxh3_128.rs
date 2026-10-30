pub(super) fn hash(preimage: &[u8]) -> [u8; 16] {
    xxhash_rust::xxh3::xxh3_128(preimage).to_be_bytes()
}
