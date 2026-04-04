use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "verifier")]
#[command(about = "ZcashVerify — Prove your Zcash balance and register badges on social platforms")]
#[command(version)]
struct Cli {
    /// Network: "main" or "test"
    #[arg(long, default_value = "main", global = true)]
    network: String,

    /// Lightwalletd gRPC URL (auto-detected from network)
    #[arg(long, global = true)]
    lwd_url: Option<String>,

    /// Badge server URL
    #[arg(long, default_value = "http://localhost:3000", global = true)]
    server_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register your badge across social platforms (recommended)
    ///
    /// Example:
    ///   verifier register --seed "your words..." --x rilso_y --zcashforum rilso --bluesky handle
    Register {
        /// BIP39 seed phrase
        #[arg(long)]
        seed: String,

        /// Account index (default: 0)
        #[arg(long, default_value = "0")]
        account: u32,

        /// Start height for scanning
        #[arg(long)]
        start_height: Option<u64>,

        /// Your X / Twitter username
        #[arg(long)]
        x: Option<String>,

        /// Your Zcash Forum username
        #[arg(long)]
        zcashforum: Option<String>,

        /// Your Bluesky handle
        #[arg(long)]
        bluesky: Option<String>,

        /// Output directory for proof files
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,
    },

    /// Generate an ownership proof for a single platform
    Prove {
        #[command(subcommand)]
        proof_type: ProveType,
    },

    /// Verify a proof from a JSON file
    Verify {
        /// Path to the proof JSON file
        #[arg(short, long)]
        proof: PathBuf,
    },

    /// Scan and display balance (no proof generated)
    Scan {
        #[command(subcommand)]
        scan_type: ScanType,
    },

    /// Submit an existing proof JSON to the badge server
    Submit {
        /// Path to the proof JSON file
        #[arg(short, long)]
        proof: PathBuf,
        /// Platform: x, zcashforum, bluesky
        #[arg(long)]
        platform: String,
        /// Your username on the platform
        #[arg(long)]
        username: String,
    },
}

#[derive(Subcommand)]
enum ProveType {
    /// Transparent (t-address) proof
    Transparent {
        /// Secret key in hex
        #[arg(long)]
        secret_key: String,
        /// Platform
        #[arg(long)]
        platform: Option<String>,
        /// Username
        #[arg(long)]
        username: Option<String>,
        /// Challenge string (auto-generated from platform:username)
        #[arg(long, default_value = "")]
        challenge: String,
        /// Output file
        #[arg(short, long, default_value = "zcash_prove.json")]
        output: PathBuf,
    },
    /// Orchard (shielded) proof
    Orchard {
        /// BIP39 seed phrase
        #[arg(long)]
        seed: String,
        /// Account index
        #[arg(long, default_value = "0")]
        account: u32,
        /// Platform
        #[arg(long)]
        platform: Option<String>,
        /// Username
        #[arg(long)]
        username: Option<String>,
        /// Challenge string (auto-generated from platform:username)
        #[arg(long, default_value = "")]
        challenge: String,
        /// Start height for scanning
        #[arg(long)]
        start_height: Option<u64>,
        /// Output file
        #[arg(short, long, default_value = "zcash_prove.json")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum ScanType {
    /// Transparent balance
    Transparent {
        #[arg(long)]
        secret_key: String,
    },
    /// Orchard shielded balance
    Orchard {
        #[arg(long)]
        seed: String,
        #[arg(long, default_value = "0")]
        account: u32,
        #[arg(long)]
        start_height: Option<u64>,
    },
}

// ── Helpers ──

fn init_logger() {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .compact()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("verifier=info".parse().unwrap()),
        )
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

fn default_lwd_url(network: &str) -> String {
    match network {
        "main" => "https://zec.rocks:443".to_string(),
        _ => "https://testnet.zec.rocks:443".to_string(),
    }
}

fn build_challenge(challenge: &str, platform: &Option<String>, username: &Option<String>) -> String {
    if let (Some(p), Some(u)) = (platform, username) {
        format!("{}:{}", p, u)
    } else if !challenge.is_empty() {
        challenge.to_string()
    } else {
        String::new()
    }
}

fn format_zec(zats: u64) -> String {
    format!("{:.8} ZEC", zats as f64 / 100_000_000.0)
}

fn encode_unified_address(hex_addr: &str, network: &str) -> Option<String> {
    let addr_bytes = hex::decode(hex_addr).ok()?;
    if addr_bytes.len() != 43 { return None; }
    let mut raw = [0u8; 43];
    raw.copy_from_slice(&addr_bytes);
    use zcash_address::unified::{self, Encoding};
    let items = vec![unified::Receiver::Orchard(raw)];
    let ua = unified::Address::try_from_items(items).ok()?;
    let net = if network == "main" {
        zcash_protocol::consensus::NetworkType::Main
    } else {
        zcash_protocol::consensus::NetworkType::Test
    };
    Some(ua.encode(&net))
}

async fn submit_to_server(
    server_url: &str,
    proof: &zcash_verifier::OwnershipProof,
    platform: &str,
    username: &str,
) -> anyhow::Result<bool> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "proof": proof,
        "platform": platform,
        "username": username,
    });

    let resp = client
        .post(format!("{}/api/verify", server_url))
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(response) => {
            let status = response.status();
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            if status.is_success() {
                let msg = body.get("message").and_then(|m| m.as_str()).unwrap_or("OK");
                println!("     {}: {}", platform, msg);
                Ok(true)
            } else {
                let msg = body.get("message").and_then(|m| m.as_str()).unwrap_or("Failed");
                println!("      {}: {}", platform, msg);
                Ok(false)
            }
        }
        Err(e) => {
            println!("     ⚠️  {}: Could not reach server ({})", platform, e);
            Ok(false)
        }
    }
}

fn print_proof_summary(proof: &zcash_verifier::OwnershipProof, network: &str) {
    let tier = zcash_verifier::BadgeTier::from_balance(
        proof.badge_tier * zcash_verifier::badge::ZAT_PER_ZEC,
    );
    println!();
    println!("  {} Badge: {}", tier.emoji(), tier);
    println!("  Image: {}", tier.image_filename());
    if let Some(ua) = encode_unified_address(&proof.address, network) {
        println!("  Address: {}", ua);
    } else {
        println!("  Address: {}", proof.address);
    }
    if let Some(bal) = proof.balance_zat {
        println!("  Balance: {} ({} zats)", format_zec(bal), bal);
    }
    println!("  Height: {}", proof.block_height);
    println!("  Expires: {}", proof.expires);
}

// ── Main ──

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    let cli = Cli::parse();
    let lwd_url = cli.lwd_url.unwrap_or_else(|| default_lwd_url(&cli.network));

    match cli.command {
        // ── REGISTER (the easy flow) ──
        Commands::Register {
            seed,
            account,
            start_height,
            x,
            zcashforum,
            bluesky,
            output_dir,
        } => {
            let x = x.map(|u| u.replace("@", "").to_lowercase());
            let zcashforum = zcashforum.map(|u| u.replace("@", "").to_lowercase());
            let bluesky = bluesky.map(|u| u.replace("@", "").to_lowercase());

            // Collect all platforms with usernames
            let mut platforms: Vec<(&str, &str)> = Vec::new();
            if let Some(ref u) = x { platforms.push(("x", u)); }
            if let Some(ref u) = zcashforum { platforms.push(("zcashforum", u)); }
            if let Some(ref u) = bluesky { platforms.push(("bluesky", u)); }

            if platforms.is_empty() {
                eprintln!("Error: Provide at least one platform username.");
                eprintln!("  --x <username>  --zcashforum <username>  --bluesky <handle>");
                std::process::exit(1);
            }

            println!();
            println!("  ╔══════════════════════════════════════╗");
            println!("  ║         ZcashVerify — Register       ║");
            println!("  ╚══════════════════════════════════════╝");
            println!();
            println!("  Platforms: {}", platforms.iter()
                .map(|(p, u)| format!("{}:{}", p, u))
                .collect::<Vec<_>>().join(", "));
            println!("  Network:   {}", cli.network);
            println!();

            // Scan once with a combined challenge
            println!("  ⏳ Scanning Orchard balance...");
            let base_challenge = platforms.iter()
                .map(|(p, u)| format!("{}:{}", p, u))
                .collect::<Vec<_>>().join("|");
            let base_proof = zcash_verifier::orchard_proof::prove_orchard(
                &seed, account, &lwd_url, &base_challenge, start_height, &cli.network,
            ).await?;

            print_proof_summary(&base_proof, &cli.network);

            // Save single combined proof JSON
            let registrations: Vec<serde_json::Value> = platforms.iter()
                .map(|(p, u)| serde_json::json!({ "platform": *p, "username": *u }))
                .collect();
            let combined = serde_json::json!({
                "proof": base_proof,
                "registrations": registrations,
            });
            let out_path = output_dir.join("zcashprovewithsocial.json");
            std::fs::write(&out_path, serde_json::to_string_pretty(&combined)?)?;
            println!();
            println!("  📄 Proof saved: {}", out_path.display());

            // Submit per-platform proofs to server
            println!("  📤 Registering badges...");
            let mut all_ok = true;
            for (platform, username) in &platforms {
                let challenge = format!("{}:{}", platform, username);
                let mut plat_proof = zcash_verifier::orchard_proof::prove_orchard(
                    &seed, account, &lwd_url, &challenge, start_height, &cli.network,
                ).await?;
                plat_proof.platform = Some(platform.to_string());
                plat_proof.username = Some(username.to_string());

                let ok = submit_to_server(&cli.server_url, &plat_proof, platform, username).await?;
                if !ok { all_ok = false; }
            }

            println!();
            if all_ok {
                println!("  ✅ All badges registered! Install the Chrome extension to see them.");
            } else {
                println!("  ⚠️  Some badges could not be registered. Check server status.");
                println!("     Proof saved at {} — use 'submit' to retry.", out_path.display());
            }
            println!();
        }

        // ── PROVE (single platform, advanced) ──
        Commands::Prove { proof_type } => match proof_type {
            ProveType::Transparent {
                secret_key, platform, username, challenge, output,
            } => {
                let platform = platform.map(|p| p.to_lowercase());
                let username = username.map(|u| u.replace("@", "").to_lowercase());
                let challenge_str = build_challenge(&challenge, &platform, &username);
                tracing::info!("Generating transparent ownership proof...");
                let mut proof = zcash_verifier::transparent::prove_transparent(
                    &secret_key, &lwd_url, &challenge_str, &cli.network,
                ).await?;
                proof.platform = platform.clone();
                proof.username = username.clone();

                let json = serde_json::to_string_pretty(&proof)?;
                std::fs::write(&output, &json)?;
                print_proof_summary(&proof, &cli.network);
                println!("  Proof: {}", output.display());

                if let (Some(p), Some(u)) = (&platform, &username) {
                    println!();
                    println!("  Submitting...");
                    submit_to_server(&cli.server_url, &proof, p, u).await?;
                }
            }
            ProveType::Orchard {
                seed, account, platform, username, challenge, start_height, output,
            } => {
                let platform = platform.map(|p| p.to_lowercase());
                let username = username.map(|u| u.replace("@", "").to_lowercase());
                let challenge_str = build_challenge(&challenge, &platform, &username);
                tracing::info!("Generating Orchard shielded ownership proof...");
                let mut proof = zcash_verifier::orchard_proof::prove_orchard(
                    &seed, account, &lwd_url, &challenge_str, start_height, &cli.network,
                ).await?;
                proof.platform = platform.clone();
                proof.username = username.clone();

                let json = serde_json::to_string_pretty(&proof)?;
                std::fs::write(&output, &json)?;
                print_proof_summary(&proof, &cli.network);
                println!("  Proof: {}", output.display());

                if let (Some(p), Some(u)) = (&platform, &username) {
                    println!();
                    println!("  Submitting...");
                    submit_to_server(&cli.server_url, &proof, p, u).await?;
                }
            }
        },

        // ── VERIFY ──
        Commands::Verify { proof } => {
            let json = std::fs::read_to_string(&proof)?;
            let p: zcash_verifier::OwnershipProof = serde_json::from_str(&json)?;
            let result = match p.proof_type.as_str() {
                "transparent" => zcash_verifier::transparent::verify_transparent(&p)?,
                "orchard" => zcash_verifier::orchard_proof::verify_orchard(&p)?,
                other => anyhow::bail!("Unknown proof type: {}", other),
            };
            println!("\n{}", result);
        }

        // ── SCAN ──
        Commands::Scan { scan_type } => match scan_type {
            ScanType::Transparent { secret_key } => {
                tracing::info!("Scanning transparent balance...");
                let secp = secp256k1::Secp256k1::new();
                let sk_bytes = hex::decode(&secret_key)?;
                let sk = secp256k1::SecretKey::from_slice(&sk_bytes)?;
                let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
                let pk_bytes = pk.serialize();
                let sha = <sha2::Sha256 as sha2::Digest>::digest(&pk_bytes);
                let hash160 = <ripemd::Ripemd160 as ripemd::Digest>::digest(&sha);
                let prefix: [u8; 2] = if cli.network == "test" {
                    [0x1D, 0x25]
                } else {
                    [0x1C, 0xB8]
                };
                let mut payload = Vec::with_capacity(22);
                payload.extend_from_slice(&prefix);
                payload.extend_from_slice(&hash160);
                let address = bs58::encode(&payload).with_check().into_string();

                let mut client = zcash_verifier::scanner::connect(&lwd_url).await?;
                let height = zcash_verifier::scanner::get_chain_height(&mut client).await?;
                let balance = zcash_verifier::scanner::scan_transparent_balance(&mut client, &address).await?;
                let tier = zcash_verifier::BadgeTier::from_balance(balance);

                println!("\n  Transparent Balance");
                println!("  Address: {}", address);
                println!("  Balance: {} ({} zats)", format_zec(balance), balance);
                println!("  {} Badge: {}", tier.emoji(), tier);
                println!("  Height: {}", height);
            }
            ScanType::Orchard { seed, account, start_height } => {
                tracing::info!("Scanning Orchard shielded balance...");
                let (balance, address_hex, height) =
                    zcash_verifier::orchard_proof::scan_orchard_balance_from_seed(
                        &seed, account, &lwd_url, start_height, &cli.network,
                    ).await?;
                let tier = zcash_verifier::BadgeTier::from_balance(balance);

                println!("\n  Orchard Shielded Balance");
                if let Some(ua) = encode_unified_address(&address_hex, &cli.network) {
                    println!("  Address: {}", ua);
                }
                println!("  Balance: {} ({} zats)", format_zec(balance), balance);
                println!("  {} Badge: {}", tier.emoji(), tier);
                println!("  Height: {}", height);
            }
        },

        // ── SUBMIT ──
        Commands::Submit { proof, platform, username } => {
            let platform = platform.to_lowercase();
            let username = username.replace("@", "").to_lowercase();
            let json = std::fs::read_to_string(&proof)?;
            let p: zcash_verifier::OwnershipProof = serde_json::from_str(&json)?;
            println!("   Submitting...");
            submit_to_server(&cli.server_url, &p, &platform, &username).await?;
        }
    }

    Ok(())
}
