use sha2::{Digest as _, Sha256};

pub(super) fn hash(preimage: &[u8]) -> [u8; 32] {
    Sha256::digest(preimage).into()
}
