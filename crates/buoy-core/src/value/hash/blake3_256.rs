pub(super) fn hash(preimage: &[u8]) -> [u8; 32] {
    *blake3::hash(preimage).as_bytes()
}
