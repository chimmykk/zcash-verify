use serde::{Deserialize, Serialize};
use crate::badge::BadgeTier;

/// The complete ownership + balance proof, serializable to JSON.
/// Designed to be consumed by a Chrome extension for badge display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnershipProof {
    /// Schema version
    pub version: u32,
    /// "transparent", "orchard", or "sapling"
    pub proof_type: String,
    /// "main" or "test"
    pub network: String,
    /// The badge tier threshold in ZEC or 0 if below minimum
    pub badge_tier: u64,
    /// The Zcash address being proven
    pub address: String,
    /// Balance in zatoshis at time of proof
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_zat: Option<u64>,
    /// Block height at which balance was checked
    pub block_height: u64,
    /// ISO 8601 timestamp of proof generation
    pub timestamp: String,
    /// ISO 8601 expiry timestamp
    pub expires: String,
    /// Identity-binding challenge string (e.g., "discord:hhanh00")
    pub challenge: String,
    /// Platform identifier: "discord", "x", "telegram", "bluesky", "zcashforum"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// Username on the platform
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Cryptographic proof data
    pub proof_data: ProofData,
}

/// Type-specific cryptographic proof data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofData {
    /// Hex-encoded signature over the proof message
    pub signature: String,
    /// Hex-encoded compressed public key (transparent only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pub_key: Option<String>,
    /// Hex-encoded full viewing key bytes (shielded only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fvk: Option<String>,
}

/// Result of verifying a proof.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub badge_tier: BadgeTier,
    pub address: String,
    pub block_height: u64,
    pub message: String,
}

impl std::fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid {
            write!(
                f,
                "Valid | Badge: {} | Address: {} | Height: {}",
                self.badge_tier, self.address, self.block_height
            )
        } else {
            write!(f, " Invalid | {}", self.message)
        }
    }
}
