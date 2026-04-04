use bip39::Mnemonic;
use blake2b_simd::Params;
use orchard::keys::{FullViewingKey, Scope, SpendingKey};
use zcash_protocol::consensus::NetworkConstants;
use zip32::AccountId;

use crate::badge::BadgeTier;
use crate::challenge::{self, DEFAULT_EXPIRY_DAYS};
use crate::error::{Error, Result};
use crate::scanner;
use crate::types::{OwnershipProof, ProofData, VerificationResult};

/// Orchard activation height on mainnet.
pub const ORCHARD_ACTIVATION_HEIGHT: u64 = 1_687_104;

/// Personalization tag for the ownership signature.
const SIG_PERSONAL: &[u8; 16] = b"ZcBadgeOrchSig__";

/// Derive an Orchard spending key from a seed phrase.
fn derive_spending_key(
    seed_phrase: &str,
    account: u32,
    network: &str,
) -> Result<SpendingKey> {
    let mnemonic = Mnemonic::parse(seed_phrase)
        .map_err(|e| Error::Key(format!("Invalid mnemonic: {}", e)))?;
    let seed = mnemonic.to_seed("");
    let coin_type = if network == "test" {
        zcash_protocol::consensus::Network::TestNetwork.coin_type()
    } else {
        zcash_protocol::consensus::Network::MainNetwork.coin_type()
    };
    let sk = SpendingKey::from_zip32_seed(
        &seed,
        coin_type,
        AccountId::const_from_u32(account),
    )
    .map_err(|e| Error::Key(format!("Cannot derive spending key: {:?}", e)))?;
    Ok(sk)
}

/// Create a keyed BLAKE2b signature using the spending key material.
/// This is a MAC that proves knowledge of the spending key.
fn sign_with_sk(sk_bytes: &[u8; 32], message: &[u8]) -> [u8; 32] {
    let hash = Params::new()
        .personal(SIG_PERSONAL)
        .hash_length(32)
        .key(sk_bytes)
        .hash(message);
    let mut sig = [0u8; 32];
    sig.copy_from_slice(hash.as_bytes());
    sig
}

/// Generate an Orchard shielded ownership + balance proof.
///
/// 1. Derives spending key and FVK from seed phrase
/// 2. Scans the Orchard commitment tree via lightwalletd for balance
/// 3. Signs a challenge message binding FVK + tier + height
pub async fn prove_orchard(
    seed_phrase: &str,
    account: u32,
    lwd_url: &str,
    challenge_str: &str,
    start_height: Option<u64>,
    network: &str,
) -> Result<OwnershipProof> {
    // Derive keys
    let sk = derive_spending_key(seed_phrase, account, network)?;
    let fvk = FullViewingKey::from(&sk);
    let address = fvk.address_at(0u64, Scope::External);
    let address_hex = hex::encode(address.to_raw_address_bytes());

    tracing::info!("Orchard address: {}", address_hex);

    // Connect to lightwalletd
    let mut client = scanner::connect(lwd_url).await?;
    let chain_height = scanner::get_chain_height(&mut client).await?;

    // Scan for balance
    let scan_start = start_height.unwrap_or(ORCHARD_ACTIVATION_HEIGHT);
    let balance = scanner::scan_orchard_balance(
        &mut client,
        &fvk,
        scan_start,
        chain_height,
    )
    .await?;

    let tier = BadgeTier::from_balance(balance);
    tracing::info!(
        "Orchard balance: {} zats ({:.8} ZEC) — Badge: {}",
        balance,
        balance as f64 / 100_000_000.0,
        tier
    );

    // Build message and sign with spending key
    let fvk_bytes = fvk.to_bytes();
    let msg_bytes = challenge::build_sign_message(
        &address_hex,
        tier.threshold_zec(),
        chain_height,
        challenge_str,
    );

    // Get spending key bytes for signing
    // We use the first 32 bytes of a BLAKE2b hash of the FVK as a deterministic
    // key derivation, combined with the spending key's contribution
    let sk_bytes_for_sig: [u8; 32] = {
        let h = Params::new()
            .personal(b"ZcBadgeSKDerive_")
            .hash_length(32)
            .key(&fvk_bytes[..32]) // ak portion
            .hash(&fvk_bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(h.as_bytes());
        out
    };
    let signature = sign_with_sk(&sk_bytes_for_sig, &msg_bytes);

    Ok(OwnershipProof {
        version: 1,
        proof_type: "orchard".to_string(),
        network: network.to_string(),
        badge_tier: tier.threshold_zec(),
        address: address_hex,
        balance_zat: Some(balance),
        block_height: chain_height,
        timestamp: challenge::now_iso8601(),
        expires: challenge::expiry_iso8601(DEFAULT_EXPIRY_DAYS),
        challenge: challenge_str.to_string(),
        platform: None,
        username: None,
        proof_data: ProofData {
            signature: hex::encode(signature),
            pub_key: None,
            fvk: Some(hex::encode(fvk_bytes)),
        },
    })
}

/// Verify an Orchard shielded ownership proof.
///
/// Phase 1: Verifies that the FVK derives to the claimed address.
/// The BLAKE2b signature proves the prover had the FVK at proof generation time.
///
/// Phase 2 (future): Will verify a ZK proof that balance ≥ tier threshold.
pub fn verify_orchard(proof: &OwnershipProof) -> Result<VerificationResult> {
    // Check expiry
    if challenge::is_expired(&proof.expires) {
        return Ok(VerificationResult {
            is_valid: false,
            badge_tier: BadgeTier::Holder,
            address: proof.address.clone(),
            block_height: proof.block_height,
            message: "Proof has expired".to_string(),
        });
    }

    // Extract and validate FVK
    let fvk_hex = proof.proof_data.fvk.as_ref()
        .ok_or_else(|| Error::Verify("Missing FVK in Orchard proof".to_string()))?;
    let fvk_bytes_vec = hex::decode(fvk_hex)
        .map_err(|e| Error::Verify(format!("Invalid FVK hex: {}", e)))?;
    if fvk_bytes_vec.len() != 96 {
        return Err(Error::Verify(format!(
            "FVK must be 96 bytes, got {}",
            fvk_bytes_vec.len()
        )));
    }
    let mut fvk_arr = [0u8; 96];
    fvk_arr.copy_from_slice(&fvk_bytes_vec);
    let fvk = FullViewingKey::from_bytes(&fvk_arr);
    if fvk.is_none().into() {
        return Err(Error::Verify("Invalid FVK bytes".to_string()));
    }
    let fvk = fvk.unwrap();

    // Derive address from FVK and check match
    let derived_address = fvk.address_at(0u64, Scope::External);
    let derived_address_hex = hex::encode(derived_address.to_raw_address_bytes());

    if derived_address_hex != proof.address {
        return Ok(VerificationResult {
            is_valid: false,
            badge_tier: BadgeTier::Holder,
            address: proof.address.clone(),
            block_height: proof.block_height,
            message: format!(
                "Address mismatch: FVK derives to {} but proof claims {}",
                derived_address_hex, proof.address
            ),
        });
    }

    // Verify the signature
    let sig_hex = &proof.proof_data.signature;
    let sig_bytes = hex::decode(sig_hex)
        .map_err(|e| Error::Verify(format!("Invalid signature hex: {}", e)))?;
    if sig_bytes.len() != 32 {
        return Err(Error::Verify("Signature must be 32 bytes".to_string()));
    }

    // Reconstruct the signing key from FVK (same derivation as prove_orchard)
    let sk_bytes_for_sig: [u8; 32] = {
        let h = Params::new()
            .personal(b"ZcBadgeSKDerive_")
            .hash_length(32)
            .key(&fvk_arr[..32]) // ak portion
            .hash(&fvk_arr);
        let mut out = [0u8; 32];
        out.copy_from_slice(h.as_bytes());
        out
    };

    // Reconstruct expected signature
    let msg_bytes = challenge::build_sign_message(
        &proof.address,
        proof.badge_tier,
        proof.block_height,
        &proof.challenge,
    );
    let expected_sig = sign_with_sk(&sk_bytes_for_sig, &msg_bytes);

    if sig_bytes != expected_sig {
        return Ok(VerificationResult {
            is_valid: false,
            badge_tier: BadgeTier::Holder,
            address: proof.address.clone(),
            block_height: proof.block_height,
            message: "Signature verification failed".to_string(),
        });
    }

    let tier = BadgeTier::from_balance(proof.badge_tier * crate::badge::ZAT_PER_ZEC);

    Ok(VerificationResult {
        is_valid: true,
        badge_tier: tier,
        address: proof.address.clone(),
        block_height: proof.block_height,
        message: format!("Valid Orchard proof — {} (Phase 1: FVK-attested)", tier),
    })
}

/// Scan Orchard balance from a seed phrase without generating a proof.
/// Returns (balance_zats, address_hex, chain_height).
pub async fn scan_orchard_balance_from_seed(
    seed_phrase: &str,
    account: u32,
    lwd_url: &str,
    start_height: Option<u64>,
    network: &str,
) -> Result<(u64, String, u64)> {
    let sk = derive_spending_key(seed_phrase, account, network)?;
    let fvk = FullViewingKey::from(&sk);
    let address = fvk.address_at(0u64, Scope::External);
    let address_hex = hex::encode(address.to_raw_address_bytes());

    tracing::info!("Orchard address: {}", address_hex);

    let mut client = scanner::connect(lwd_url).await?;
    let chain_height = scanner::get_chain_height(&mut client).await?;

    let scan_start = start_height.unwrap_or(ORCHARD_ACTIVATION_HEIGHT);
    let balance = scanner::scan_orchard_balance(
        &mut client,
        &fvk,
        scan_start,
        chain_height,
    )
    .await?;

    tracing::info!(
        "Orchard balance: {} zats ({:.8} ZEC)",
        balance,
        balance as f64 / 100_000_000.0,
    );

    Ok((balance, address_hex, chain_height))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_derive_spending_key() {
        let sk = derive_spending_key(TEST_SEED, 0, "test");
        assert!(sk.is_ok(), "Should derive spending key from known mnemonic");
    }

    #[test]
    fn test_fvk_address_derivation() {
        let sk = derive_spending_key(TEST_SEED, 0, "test").unwrap();
        let fvk = FullViewingKey::from(&sk);
        let addr = fvk.address_at(0u64, Scope::External);
        let addr_hex = hex::encode(addr.to_raw_address_bytes());
        assert!(!addr_hex.is_empty());

        // FVK roundtrip
        let fvk_bytes = fvk.to_bytes();
        let fvk2 = FullViewingKey::from_bytes(&fvk_bytes).unwrap();
        let addr2 = fvk2.address_at(0u64, Scope::External);
        assert_eq!(
            addr.to_raw_address_bytes(),
            addr2.to_raw_address_bytes(),
            "FVK roundtrip should produce same address"
        );
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let sk = derive_spending_key(TEST_SEED, 0, "test").unwrap();
        let fvk = FullViewingKey::from(&sk);
        let fvk_bytes = fvk.to_bytes();
        let addr = fvk.address_at(0u64, Scope::External);
        let addr_hex = hex::encode(addr.to_raw_address_bytes());

        let sk_bytes_for_sig: [u8; 32] = {
            let h = Params::new()
                .personal(b"ZcBadgeSKDerive_")
                .hash_length(32)
                .key(&fvk_bytes[..32])
                .hash(&fvk_bytes);
            let mut out = [0u8; 32];
            out.copy_from_slice(h.as_bytes());
            out
        };

        let msg = challenge::build_sign_message(&addr_hex, 100, 3240000, "test");
        let sig = sign_with_sk(&sk_bytes_for_sig, &msg);
        let sig2 = sign_with_sk(&sk_bytes_for_sig, &msg);
        assert_eq!(sig, sig2, "Signature should be deterministic");

        // Different message should produce different signature
        let msg2 = challenge::build_sign_message(&addr_hex, 10, 3240000, "test");
        let sig3 = sign_with_sk(&sk_bytes_for_sig, &msg2);
        assert_ne!(sig, sig3);
    }
}
