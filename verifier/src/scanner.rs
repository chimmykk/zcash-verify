use orchard::keys::{PreparedIncomingViewingKey, Scope, FullViewingKey};
use orchard::note::ExtractedNoteCommitment;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};
use zcash_note_encryption::EphemeralKeyBytes;

use crate::error::Result;
use crate::rpc::{
    BlockId, BlockRange, ChainSpec, CompactOrchardAction, GetAddressUtxosArg,
    compact_tx_streamer_client::CompactTxStreamerClient,
};

pub type LwdClient = CompactTxStreamerClient<Channel>;

/// Connect to a lightwalletd instance.
pub async fn connect(url: &str) -> Result<LwdClient> {
    let ep = Endpoint::from_shared(url.to_string())?;
    let client = CompactTxStreamerClient::connect(ep).await?;
    Ok(client)
}

/// Get the current chain tip height.
pub async fn get_chain_height(client: &mut LwdClient) -> Result<u64> {
    let resp = client
        .get_latest_block(Request::new(ChainSpec {}))
        .await?
        .into_inner();
    Ok(resp.height)
}

/// Query transparent UTXO balance for a given t-address.
/// Returns the total unspent balance in zatoshis.
pub async fn scan_transparent_balance(
    client: &mut LwdClient,
    address: &str,
) -> Result<u64> {
    let resp = client
        .get_address_utxos(Request::new(GetAddressUtxosArg {
            addresses: vec![address.to_string()],
            start_height: 0,
            max_entries: 0, // unlimited
        }))
        .await?
        .into_inner();

    let total: i64 = resp.address_utxos.iter().map(|u| u.value_zat).sum();
    Ok(total as u64)
}

/// Build a `CompactAction` from a protobuf `CompactOrchardAction`.
fn build_compact_action(
    action: &CompactOrchardAction,
) -> orchard::note_encryption::CompactAction {
    let mut nf_buf = [0u8; 32];
    if action.nullifier.len() == 32 {
        nf_buf.copy_from_slice(&action.nullifier);
    }
    let mut cmx_buf = [0u8; 32];
    if action.cmx.len() == 32 {
        cmx_buf.copy_from_slice(&action.cmx);
    }
    let mut epk_buf = [0u8; 32];
    if action.ephemeral_key.len() == 32 {
        epk_buf.copy_from_slice(&action.ephemeral_key);
    }
    let mut enc = [0u8; 52];
    let len = action.ciphertext.len().min(52);
    enc[..len].copy_from_slice(&action.ciphertext[..len]);

    orchard::note_encryption::CompactAction::from_parts(
        orchard::note::Nullifier::from_bytes(&nf_buf).unwrap(),
        ExtractedNoteCommitment::from_bytes(&cmx_buf).unwrap(),
        EphemeralKeyBytes(epk_buf),
        enc,
    )
}

/// Scan Orchard shielded balance by trial-decrypting compact blocks.
///
/// Downloads compact blocks from `start_height` to `end_height` and
/// attempts to decrypt each Orchard action with the provided IVK.
/// Tracks nullifiers to exclude spent notes.
///
/// Returns the total unspent balance in zatoshis.
pub async fn scan_orchard_balance(
    client: &mut LwdClient,
    fvk: &FullViewingKey,
    start_height: u64,
    end_height: u64,
) -> Result<u64> {
    let ivk = fvk.to_ivk(Scope::External);
    let prepared_ivk = PreparedIncomingViewingKey::new(&ivk);

    // Also scan internal (change) scope
    let ivk_internal = fvk.to_ivk(Scope::Internal);
    let prepared_ivk_internal = PreparedIncomingViewingKey::new(&ivk_internal);

    // Collect notes and nullifiers
    let mut notes: Vec<(u64, [u8; 32])> = Vec::new(); // (value, nullifier)
    let mut spent_nullifiers: std::collections::HashSet<[u8; 32]> =
        std::collections::HashSet::new();

    tracing::info!(
        "Scanning Orchard blocks {} to {} ({} blocks)...",
        start_height,
        end_height,
        end_height.saturating_sub(start_height)
    );

    // Stream compact blocks
    let mut stream = client
        .get_block_range(Request::new(BlockRange {
            start: Some(BlockId {
                height: start_height,
                hash: vec![],
            }),
            end: Some(BlockId {
                height: end_height,
                hash: vec![],
            }),
            pool_types: vec![], // all pools
        }))
        .await?
        .into_inner();

    let mut blocks_processed: u64 = 0;

    while let Some(block) = stream.message().await? {
        blocks_processed += 1;
        if blocks_processed % 10000 == 0 {
            tracing::info!(
                "Processed {} blocks (height {})...",
                blocks_processed,
                block.height
            );
        }

        for tx in &block.vtx {
            for action in &tx.actions {
                // Record nullifiers (these are spends)
                if action.nullifier.len() == 32 {
                    let mut nf = [0u8; 32];
                    nf.copy_from_slice(&action.nullifier);
                    spent_nullifiers.insert(nf);
                }

                // Build the compact action once
                let compact_action = build_compact_action(action);
                let domain =
                    orchard::note_encryption::OrchardDomain::for_compact_action(&compact_action);

                // Try to decrypt this action with both IVKs
                for pivk in [&prepared_ivk, &prepared_ivk_internal] {
                    if let Some((note, _)) =
                        zcash_note_encryption::try_compact_note_decryption(
                            &domain,
                            pivk,
                            &compact_action,
                        )
                    {
                        let nf = note.nullifier(fvk);
                        let value = note.value().inner();
                        tracing::info!(
                            "Found Orchard note at height {}: {} zats",
                            block.height,
                            value
                        );
                        notes.push((value, nf.to_bytes()));
                        break; // Don't try the other IVK
                    }
                }
            }
        }
    }

    // Compute unspent balance: notes whose nullifiers are NOT in the spent set
    let unspent_balance: u64 = notes
        .iter()
        .filter(|(_, nf)| !spent_nullifiers.contains(nf))
        .map(|(value, _)| *value)
        .sum();

    tracing::info!(
        "Scan complete: {} blocks, {} notes found, {} spent, balance = {} zats ({:.8} ZEC)",
        blocks_processed,
        notes.len(),
        notes
            .iter()
            .filter(|(_, nf)| spent_nullifiers.contains(nf))
            .count(),
        unspent_balance,
        unspent_balance as f64 / 100_000_000.0,
    );

    Ok(unspent_balance)
}
