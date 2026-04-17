use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha256};

/// Cryptographic RNG with commit-reveal scheme.
///
/// Before a round starts the server generates a random seed, computes its
/// SHA-256 hash (the *commitment*), and stores both. The commitment can be
/// shared with the client. During play the seed drives a deterministic
/// ChaCha20 PRNG. After the round the seed is revealed so the client can
/// verify no manipulation occurred.
#[derive(Debug, Clone)]
pub struct CommitRevealRng {
    seed: [u8; 32],
    commitment: String,
}

impl CommitRevealRng {
    /// Create a new RNG from OS entropy.
    pub fn new() -> Self {
        let mut seed = [0u8; 32];
        rand::rngs::OsRng.try_fill_bytes(&mut seed)
            .expect("OS RNG should be available");
        let commitment = Self::compute_commitment(&seed);
        Self { seed, commitment }
    }

    /// Create from an explicit seed (useful for determinism tests).
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let commitment = Self::compute_commitment(&seed);
        Self { seed, commitment }
    }

    /// The SHA-256 commitment that can be shared before the game starts.
    pub fn commitment(&self) -> &str {
        &self.commitment
    }

    /// Build the deterministic PRNG for use during the round.
    pub fn rng(&self) -> ChaCha20Rng {
        ChaCha20Rng::from_seed(self.seed)
    }

    /// Reveal the seed after the round so the client can verify.
    pub fn reveal_seed(&self) -> String {
        hex::encode(self.seed)
    }

    /// Verify that a revealed seed matches a commitment.
    pub fn verify(seed_hex: &str, commitment: &str) -> bool {
        if let Ok(seed_bytes) = hex::decode(seed_hex) {
            if seed_bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&seed_bytes);
                let computed = Self::compute_commitment(&arr);
                return computed == commitment;
            }
        }
        false
    }

    fn compute_commitment(seed: &[u8; 32]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hex::encode(hasher.finalize())
    }
}

impl Default for CommitRevealRng {
    fn default() -> Self {
        Self::new()
    }
}

use rand::RngCore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_verification() {
        let rng = CommitRevealRng::new();
        let commitment = rng.commitment().to_string();
        let seed_hex = rng.reveal_seed();
        assert!(CommitRevealRng::verify(&seed_hex, &commitment));
    }

    #[test]
    fn test_deterministic_from_seed() {
        let seed = [42u8; 32];
        let rng1 = CommitRevealRng::from_seed(seed);
        let rng2 = CommitRevealRng::from_seed(seed);

        let mut prng1 = rng1.rng();
        let mut prng2 = rng2.rng();

        let mut buf1 = [0u8; 16];
        let mut buf2 = [0u8; 16];
        prng1.fill_bytes(&mut buf1);
        prng2.fill_bytes(&mut buf2);
        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_wrong_seed_fails_verification() {
        let rng = CommitRevealRng::new();
        let commitment = rng.commitment().to_string();
        let wrong_seed = hex::encode([0u8; 32]);
        assert!(!CommitRevealRng::verify(&wrong_seed, &commitment));
    }
}
