use blake2b_simd::Params;
use chrono::{Duration, Utc};

/// Default proof expiry: 30 days.
pub const DEFAULT_EXPIRY_DAYS: i64 = 30;

/// Personalization tag for challenge hashing.
const CHALLENGE_PERSONAL: &[u8; 16] = b"ZcBadgeChallnge_";

/// Construct the message bytes to be signed.
///
/// The message binds the proof to:
/// - The claimed address
/// - The badge tier
/// - The block height
/// - The user-provided challenge (identity binding)
pub fn build_sign_message(
    address: &str,
    badge_tier_zec: u64,
    block_height: u64,
    challenge: &str,
) -> Vec<u8> {
    let input = format!(
        "ZcashBadgeProof:{}:{}:{}:{}",
        address, badge_tier_zec, block_height, challenge
    );
    let hash = Params::new()
        .personal(CHALLENGE_PERSONAL)
        .hash_length(32)
        .hash(input.as_bytes());
    hash.as_bytes().to_vec()
}

/// Generate the current ISO 8601 timestamp.
pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339()
}

/// Generate the expiry timestamp (now + days).
pub fn expiry_iso8601(days: i64) -> String {
    let expiry = Utc::now() + Duration::days(days);
    expiry.to_rfc3339()
}

/// Check if a proof has expired.
pub fn is_expired(expires: &str) -> bool {
    match chrono::DateTime::parse_from_rfc3339(expires) {
        Ok(expiry) => Utc::now() > expiry,
        Err(_) => true, // If we can't parse the expiry, treat as expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_message_deterministic() {
        let msg1 = build_sign_message("t1addr", 100, 3240000, "test");
        let msg2 = build_sign_message("t1addr", 100, 3240000, "test");
        assert_eq!(msg1, msg2);
        assert_eq!(msg1.len(), 32);
    }

    #[test]
    fn test_sign_message_different_inputs() {
        let msg1 = build_sign_message("t1addr", 100, 3240000, "test");
        let msg2 = build_sign_message("t1addr", 10, 3240000, "test");
        assert_ne!(msg1, msg2);
    }

    #[test]
    fn test_expiry() {
        let past = "2020-01-01T00:00:00+00:00";
        assert!(is_expired(past));

        let future = expiry_iso8601(7);
        assert!(!is_expired(&future));
    }
}
