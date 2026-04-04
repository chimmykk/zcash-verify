use secp256k1::{Message, Secp256k1, SecretKey, ecdsa::RecoverableSignature, ecdsa::RecoveryId};
use sha2::{Digest, Sha256};

use crate::badge::BadgeTier;
use crate::challenge::{self, DEFAULT_EXPIRY_DAYS};
use crate::error::{Error, Result};
use crate::scanner;
use crate::types::{OwnershipProof, ProofData, VerificationResult};

/// Zcash mainnet t-address version bytes (P2PKH)
const MAINNET_PUBKEY_PREFIX: [u8; 2] = [0x1C, 0xB8];
/// Zcash testnet t-address version bytes (P2PKH)
const TESTNET_PUBKEY_PREFIX: [u8; 2] = [0x1D, 0x25];

fn address_prefix(network: &str) -> [u8; 2] {
    if network == "test" {
        TESTNET_PUBKEY_PREFIX
    } else {
        MAINNET_PUBKEY_PREFIX
    }
}

/// Double-SHA256 hash.
fn sha256d(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(&first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

/// Derive a Zcash t-address from a compressed public key (mainnet).
fn pubkey_to_taddr(pubkey: &[u8; 33], network: &str) -> String {
    let sha = Sha256::digest(pubkey);
    let hash160 = ripemd::Ripemd160::digest(&sha);

    let mut payload = Vec::with_capacity(22);
    payload.extend_from_slice(&address_prefix(network));
    payload.extend_from_slice(&hash160);

    bs58::encode(&payload).with_check().into_string()
}

/// Generate a transparent ownership + balance proof.
///
/// 1. Parses the secret key
/// 2. Queries lightwalletd for UTXO balance
/// 3. Signs a challenge message binding address + tier + height
pub async fn prove_transparent(
    secret_key_hex: &str,
    lwd_url: &str,
    challenge_str: &str,
    network: &str,
) -> Result<OwnershipProof> {
    let secp = Secp256k1::new();

    // Parse secret key from hex
    let sk_bytes = hex::decode(secret_key_hex)
        .map_err(|e| Error::Key(format!("Invalid hex secret key: {}", e)))?;
    let sk = SecretKey::from_slice(&sk_bytes)
        .map_err(|e| Error::Key(format!("Invalid secret key: {}", e)))?;
    let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
    let pk_bytes = pk.serialize(); // 33 bytes compressed

    // Derive t-address
    let address = pubkey_to_taddr(&pk_bytes, network);
    tracing::info!("Transparent address: {}", address);

    // Connect to lightwalletd and get balance
    let mut client = scanner::connect(lwd_url).await?;
    let balance = scanner::scan_transparent_balance(&mut client, &address).await?;
    let height = scanner::get_chain_height(&mut client).await?;

    let tier = BadgeTier::from_balance(balance);
    tracing::info!(
        "Balance: {} zats ({:.8} ZEC) — Badge: {}",
        balance,
        balance as f64 / 100_000_000.0,
        tier
    );

    // Build message and sign
    let msg_bytes = challenge::build_sign_message(
        &address,
        tier.threshold_zec(),
        height,
        challenge_str,
    );
    let msg = Message::from_digest(sha256d(&msg_bytes));
    let sig = secp.sign_ecdsa_recoverable(&msg, &sk);
    let (rec_id, sig_bytes) = sig.serialize_compact();

    // Encode signature as rec_id byte + 64 sig bytes = 65 bytes total
    let mut full_sig = Vec::with_capacity(65);
    full_sig.push(rec_id.to_i32() as u8);
    full_sig.extend_from_slice(&sig_bytes);

    Ok(OwnershipProof {
        version: 1,
        proof_type: "transparent".to_string(),
        network: network.to_string(),
        badge_tier: tier.threshold_zec(),
        address,
        balance_zat: Some(balance),
        block_height: height,
        timestamp: challenge::now_iso8601(),
        expires: challenge::expiry_iso8601(DEFAULT_EXPIRY_DAYS),
        challenge: challenge_str.to_string(),
        platform: None,
        username: None,
        proof_data: ProofData {
            signature: hex::encode(&full_sig),
            pub_key: Some(hex::encode(pk_bytes)),
            fvk: None,
        },
    })
}

/// Verify a transparent ownership proof.
///
/// 1. Recovers the public key from the signature
/// 2. Derives the t-address and checks it matches
/// 3. Checks expiry
pub fn verify_transparent(proof: &OwnershipProof) -> Result<VerificationResult> {
    let secp = Secp256k1::new();

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

    // Extract signature
    let sig_bytes = hex::decode(&proof.proof_data.signature)
        .map_err(|e| Error::Verify(format!("Invalid signature hex: {}", e)))?;
    if sig_bytes.len() != 65 {
        return Err(Error::Verify("Signature must be 65 bytes".to_string()));
    }
    let rec_id = RecoveryId::from_i32(sig_bytes[0] as i32)
        .map_err(|e| Error::Verify(format!("Invalid recovery ID: {}", e)))?;
    let sig = RecoverableSignature::from_compact(&sig_bytes[1..], rec_id)
        .map_err(|e| Error::Verify(format!("Invalid signature: {}", e)))?;

    // Reconstruct the signed message
    let msg_bytes = challenge::build_sign_message(
        &proof.address,
        proof.badge_tier,
        proof.block_height,
        &proof.challenge,
    );
    let msg = Message::from_digest(sha256d(&msg_bytes));

    // Recover public key
    let recovered_pk = secp.recover_ecdsa(&msg, &sig)
        .map_err(|e| Error::Verify(format!("Signature recovery failed: {}", e)))?;
    let recovered_pk_bytes = recovered_pk.serialize();

    // Derive t-address from recovered pubkey
    let recovered_addr = pubkey_to_taddr(&recovered_pk_bytes, &proof.network);

    // Verify address matches
    if recovered_addr != proof.address {
        return Ok(VerificationResult {
            is_valid: false,
            badge_tier: BadgeTier::Holder,
            address: proof.address.clone(),
            block_height: proof.block_height,
            message: format!(
                "Address mismatch: recovered {} but claimed {}",
                recovered_addr, proof.address
            ),
        });
    }

    // Also check the provided pubkey matches (if present)
    if let Some(pub_key_hex) = &proof.proof_data.pub_key {
        let provided_pk = hex::decode(pub_key_hex)
            .map_err(|e| Error::Verify(format!("Invalid pubkey hex: {}", e)))?;
        if provided_pk != recovered_pk_bytes {
            return Ok(VerificationResult {
                is_valid: false,
                badge_tier: BadgeTier::Holder,
                address: proof.address.clone(),
                block_height: proof.block_height,
                message: "Provided public key doesn't match signature".to_string(),
            });
        }
    }

    let tier = BadgeTier::from_balance(proof.badge_tier * crate::badge::ZAT_PER_ZEC);

    Ok(VerificationResult {
        is_valid: true,
        badge_tier: tier,
        address: proof.address.clone(),
        block_height: proof.block_height,
        message: format!("Valid transparent proof — {}", tier),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubkey_to_taddr() {
        // Known test vector: the private key 0x01 on secp256k1
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[0u8; 31].iter().chain(&[1u8]).cloned().collect::<Vec<u8>>()).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
        let addr_main = pubkey_to_taddr(&pk.serialize(), "main");
        assert!(addr_main.starts_with("t1"), "Expected t1 prefix, got: {}", addr_main);
        let addr_test = pubkey_to_taddr(&pk.serialize(), "test");
        assert!(addr_test.starts_with("tm"), "Expected tm prefix, got: {}", addr_test);
    }
}
