use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod db;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

/// Badge info returned by the API
#[derive(Debug, Serialize, Deserialize)]
struct BadgeResponse {
    platform: String,
    username: String,
    badge_tier: u8,
    badge_name: String,
    badge_image: String,
    verified: bool,
    expires_at: String,
}

/// Request to verify and register a proof
#[derive(Debug, Deserialize)]
struct VerifyRequest {
    proof: zcash_verifier::OwnershipProof,
    platform: String,
    username: String,
}

/// Response after verification
#[derive(Debug, Serialize)]
struct VerifyResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    badge: Option<BadgeResponse>,
}

/// Query params for batch badge lookup
#[derive(Debug, Deserialize)]
struct BadgesQuery {
    platform: String,
    usernames: String,
}

/// Request to generate proofs and register badges (web app flow)
#[derive(Debug, Deserialize)]
struct RegisterRequest {
    seed: String,
    #[serde(default)]
    account: u32,
    start_height: Option<u64>,
    #[serde(default = "default_network")]
    network: String,
    x: Option<String>,
    zcashforum: Option<String>,
    bluesky: Option<String>,
}

fn default_network() -> String {
    "main".to_string()
}

/// Response after generating and registering badges
#[derive(Debug, Serialize)]
struct RegisterResponse {
    success: bool,
    message: String,
    badges: Vec<BadgeResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    balance_zat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    badge_tier: Option<String>,
}

/// Request to scan balance without registering
#[derive(Debug, Deserialize)]
struct ScanRequest {
    seed: String,
    #[serde(default)]
    account: u32,
    start_height: Option<u64>,
    #[serde(default = "default_network")]
    network: String,
}

/// Response from balance scan
#[derive(Debug, Serialize)]
struct ScanResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    balance_zat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    balance_zec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    badge_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    badge_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
}

fn default_lwd_url(network: &str) -> String {
    match network {
        "main" => "https://zec.rocks:443".to_string(),
        _ => "https://testnet.zec.rocks:443".to_string(),
    }
}

fn normalize_username(username: &str) -> String {
    username.replace('@', "").to_lowercase()
}

fn valid_platforms() -> [&'static str; 3] {
    ["x", "bluesky", "zcashforum"]
}

fn validate_platform(platform: &str) -> Result<(), String> {
    if valid_platforms().contains(&platform) {
        Ok(())
    } else {
        Err(format!(
            "Invalid platform '{}'. Valid: {:?}",
            platform,
            valid_platforms()
        ))
    }
}

async fn verify_and_store(
    db: &SqlitePool,
    proof: &zcash_verifier::OwnershipProof,
    platform: &str,
    username: &str,
) -> Result<BadgeResponse, String> {
    validate_platform(platform)?;

    let expected_challenge = format!("{}:{}", platform, username);
    if !proof.challenge.is_empty() && proof.challenge != expected_challenge {
        return Err(format!(
            "Challenge mismatch: proof has '{}' but expected '{}'",
            proof.challenge, expected_challenge
        ));
    }

    let result = match proof.proof_type.as_str() {
        "transparent" => zcash_verifier::transparent::verify_transparent(proof),
        "orchard" => zcash_verifier::orchard_proof::verify_orchard(proof),
        other => return Err(format!("Unknown proof type: {}", other)),
    };

    match result {
        Ok(vr) if vr.is_valid => {
            let tier = zcash_verifier::BadgeTier::from_balance(
                proof.badge_tier * zcash_verifier::badge::ZAT_PER_ZEC,
            );

            let badge = BadgeResponse {
                platform: platform.to_string(),
                username: username.to_string(),
                badge_tier: tier.level(),
                badge_name: tier.to_string(),
                badge_image: tier.image_filename().to_string(),
                verified: true,
                expires_at: proof.expires.clone(),
            };

            let proof_json = serde_json::to_string(proof).unwrap_or_default();
            db::upsert_badge(
                db,
                platform,
                username,
                tier.level(),
                &tier.to_string(),
                tier.image_filename(),
                &proof.proof_type,
                &proof.network,
                &proof.address,
                proof.block_height as i64,
                &proof.expires,
                &proof_json,
            )
            .await
            .map_err(|e| format!("Database error: {}", e))?;

            tracing::info!(
                "✅ Badge verified: {}:{} → {} ({})",
                platform,
                username,
                tier,
                tier.emoji()
            );

            Ok(badge)
        }
        Ok(vr) => Err(format!("Proof invalid: {}", vr.message)),
        Err(e) => Err(format!("Verification error: {}", e)),
    }
}

// ── Handlers ──────────────────────────────────────────

/// POST /api/verify — Submit and verify a proof
async fn verify_proof(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> (StatusCode, Json<VerifyResponse>) {
    let proof = &req.proof;

    match verify_and_store(&state.db, proof, &req.platform, &req.username).await {
        Ok(badge) => {
            let tier = badge.badge_name.clone();
            (
                StatusCode::OK,
                Json(VerifyResponse {
                    success: true,
                    message: format!("Proof verified! Badge: {}", tier),
                    badge: Some(badge),
                }),
            )
        }
        Err(message) => (
            StatusCode::BAD_REQUEST,
            Json(VerifyResponse {
                success: false,
                message,
                badge: None,
            }),
        ),
    }
}

/// POST /api/register — Generate proofs and register badges for social identities
async fn register_badges(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> (StatusCode, Json<RegisterResponse>) {
    let lwd_url = default_lwd_url(&req.network);
    let mut platforms: Vec<(&str, String)> = Vec::new();

    if let Some(u) = req.x.as_deref().filter(|s| !s.trim().is_empty()) {
        platforms.push(("x", normalize_username(u)));
    }
    if let Some(u) = req.zcashforum.as_deref().filter(|s| !s.trim().is_empty()) {
        platforms.push(("zcashforum", normalize_username(u)));
    }
    if let Some(u) = req.bluesky.as_deref().filter(|s| !s.trim().is_empty()) {
        platforms.push(("bluesky", normalize_username(u)));
    }

    if platforms.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: "Provide at least one social username (x, zcashforum, or bluesky).".into(),
                badges: vec![],
                balance_zat: None,
                badge_tier: None,
            }),
        );
    }

    tracing::info!(
        "Generating proofs for platforms: {}",
        platforms
            .iter()
            .map(|(p, u)| format!("{}:{}", p, u))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let base_challenge = platforms
        .iter()
        .map(|(p, u)| format!("{}:{}", p, u))
        .collect::<Vec<_>>()
        .join("|");

    let base_proof = match zcash_verifier::orchard_proof::prove_orchard(
        &req.seed,
        req.account,
        &lwd_url,
        &base_challenge,
        req.start_height,
        &req.network,
    )
    .await
    {
        Ok(proof) => proof,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(RegisterResponse {
                    success: false,
                    message: format!("Proof generation failed: {}", e),
                    badges: vec![],
                    balance_zat: None,
                    badge_tier: None,
                }),
            );
        }
    };

    let balance_zat = base_proof.balance_zat;
    let tier = zcash_verifier::BadgeTier::from_balance(
        base_proof.badge_tier * zcash_verifier::badge::ZAT_PER_ZEC,
    );
    let tier_name = tier.to_string();

    let mut badges = Vec::new();
    let mut errors = Vec::new();

    for (platform, username) in &platforms {
        let challenge = format!("{}:{}", platform, username);
        let mut proof = match zcash_verifier::orchard_proof::prove_orchard(
            &req.seed,
            req.account,
            &lwd_url,
            &challenge,
            req.start_height,
            &req.network,
        )
        .await
        {
            Ok(proof) => proof,
            Err(e) => {
                errors.push(format!("{}:{} — {}", platform, username, e));
                continue;
            }
        };

        proof.platform = Some(platform.to_string());
        proof.username = Some(username.clone());

        match verify_and_store(&state.db, &proof, platform, username).await {
            Ok(badge) => badges.push(badge),
            Err(e) => errors.push(format!("{}:{} — {}", platform, username, e)),
        }
    }

    if badges.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: errors.join("; "),
                badges,
                balance_zat,
                badge_tier: Some(tier_name),
            }),
        );
    }

    let message = if errors.is_empty() {
        format!(
            "Registered {} badge(s). Install the Chrome extension to see them on social platforms.",
            badges.len()
        )
    } else {
        format!(
            "Registered {} badge(s). Some platforms failed: {}",
            badges.len(),
            errors.join("; ")
        )
    };

    (
        StatusCode::OK,
        Json(RegisterResponse {
            success: true,
            message,
            badges,
            balance_zat,
            badge_tier: Some(tier_name),
        }),
    )
}

/// POST /api/scan — Scan Orchard balance without registering
async fn scan_balance(Json(req): Json<ScanRequest>) -> (StatusCode, Json<ScanResponse>) {
    let lwd_url = default_lwd_url(&req.network);

    match zcash_verifier::orchard_proof::scan_orchard_balance_from_seed(
        &req.seed,
        req.account,
        &lwd_url,
        req.start_height,
        &req.network,
    )
    .await
    {
        Ok((balance, address_hex, height)) => {
            let tier = zcash_verifier::BadgeTier::from_balance(balance);
            let balance_zec = format!("{:.8} ZEC", balance as f64 / 100_000_000.0);
            let address = zcash_verifier::orchard_proof::encode_unified_address(
                &address_hex,
                &req.network,
            )
            .or_else(|| Some(address_hex));
            (
                StatusCode::OK,
                Json(ScanResponse {
                    success: true,
                    message: "Balance scan complete".into(),
                    balance_zat: Some(balance),
                    balance_zec: Some(balance_zec),
                    badge_tier: Some(tier.to_string()),
                    badge_name: Some(tier.to_string()),
                    block_height: Some(height),
                    address,
                }),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ScanResponse {
                success: false,
                message: format!("Scan failed: {}", e),
                balance_zat: None,
                balance_zec: None,
                badge_tier: None,
                badge_name: None,
                block_height: None,
                address: None,
            }),
        ),
    }
}

/// GET /api/badges?platform=x&usernames=user1,user2 (public, read-only)
async fn get_badges(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BadgesQuery>,
) -> Json<Vec<BadgeResponse>> {
    let usernames: Vec<&str> = params.usernames.split(',').map(|s| s.trim()).collect();
    let now = chrono::Utc::now().to_rfc3339();

    let mut results = Vec::new();
    for username in &usernames {
        if let Ok(Some(badge)) = db::get_badge(&state.db, &params.platform, username, &now).await {
            results.push(badge);
        }
    }

    Json(results)
}

/// GET /api/badge/:platform/:username (public, read-only)
async fn get_badge(
    State(state): State<Arc<AppState>>,
    Path((platform, username)): Path<(String, String)>,
) -> Result<Json<BadgeResponse>, StatusCode> {
    let now = chrono::Utc::now().to_rfc3339();
    match db::get_badge(&state.db, &platform, &username, &now).await {
        Ok(Some(badge)) => Ok(Json(badge)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/health
async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .compact()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("badge_server=info".parse().unwrap()),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:badges.db".to_string());
    let db = db::init_db(&db_url).await?;

    let state = Arc::new(AppState { db });

    let app = Router::new()
        .route("/api/verify", post(verify_proof))
        .route("/api/register", post(register_badges))
        .route("/api/scan", post(scan_balance))
        .route("/api/badges", get(get_badges))
        .route("/api/badge/{platform}/{username}", get(get_badge))
        .route("/api/health", get(health))
        .nest_service("/badges", ServeDir::new("static/badges"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("Badge server starting on http://{}", addr);
    tracing::info!(" Rate limiting is disabled");
    tracing::info!(" Endpoints:");
    tracing::info!("   POST /api/verify      — Submit proof");
    tracing::info!("   POST /api/register    — Generate + register badges");
    tracing::info!("   POST /api/scan        — Scan balance only");
    tracing::info!("   GET  /api/badges      — Batch lookup");
    tracing::info!("   GET  /api/badge/:p/:u  — Single lookup");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
