//! Allowlisted digest implementation adapters.

#[cfg(feature = "blake3-256")]
mod blake3_256;
#[cfg(feature = "sha-256")]
mod sha_256;
#[cfg(feature = "xxh3-128")]
mod xxh3_128;

use super::{ActiveAlgorithmError, DigestAlgorithm};

#[allow(clippy::unnecessary_wraps)]
pub(super) fn hash(
    algorithm: DigestAlgorithm,
    preimage: &[u8],
) -> Result<[u8; 32], ActiveAlgorithmError> {
    let mut output = [0_u8; 32];
    match algorithm {
        DigestAlgorithm::Xxh3_128 => {
            #[cfg(feature = "xxh3-128")]
            {
                output[..16].copy_from_slice(&xxh3_128::hash(preimage));
                Ok(output)
            }
            #[cfg(not(feature = "xxh3-128"))]
            {
                Err(ActiveAlgorithmError::Unavailable(algorithm))
            }
        }
        DigestAlgorithm::Blake3_256 => {
            #[cfg(feature = "blake3-256")]
            {
                output.copy_from_slice(&blake3_256::hash(preimage));
                Ok(output)
            }
            #[cfg(not(feature = "blake3-256"))]
            {
                Err(ActiveAlgorithmError::Unavailable(algorithm))
            }
        }
        DigestAlgorithm::Sha256 => {
            #[cfg(feature = "sha-256")]
            {
                output.copy_from_slice(&sha_256::hash(preimage));
                Ok(output)
            }
            #[cfg(not(feature = "sha-256"))]
            {
                Err(ActiveAlgorithmError::Unavailable(algorithm))
            }
        }
    }
}
