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

// ── Handlers ──────────────────────────────────────────

/// POST /api/verify — Submit and verify a proof
async fn verify_proof(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> (StatusCode, Json<VerifyResponse>) {
    let proof = &req.proof;

    // No rate limiting per user request

    // ── Validate platform ──
    let valid_platforms = ["x", "bluesky", "zcashforum"];
    if !valid_platforms.contains(&req.platform.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(VerifyResponse {
                success: false,
                message: format!("Invalid platform '{}'. Valid: {:?}", req.platform, valid_platforms),
                badge: None,
            }),
        );
    }

    // ── Challenge binding ──
    let expected_challenge = format!("{}:{}", req.platform, req.username);
    if !proof.challenge.is_empty() && proof.challenge != expected_challenge {
        return (
            StatusCode::BAD_REQUEST,
            Json(VerifyResponse {
                success: false,
                message: format!(
                    "Challenge mismatch: proof has '{}' but expected '{}'",
                    proof.challenge, expected_challenge
                ),
                badge: None,
            }),
        );
    }

    // ── Cryptographic verification ──
    let result = match proof.proof_type.as_str() {
        "transparent" => zcash_verifier::transparent::verify_transparent(proof),
        "orchard" => zcash_verifier::orchard_proof::verify_orchard(proof),
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(VerifyResponse {
                    success: false,
                    message: format!("Unknown proof type: {}", other),
                    badge: None,
                }),
            );
        }
    };

    match result {
        Ok(vr) if vr.is_valid => {
            let tier = zcash_verifier::BadgeTier::from_balance(
                proof.badge_tier * zcash_verifier::badge::ZAT_PER_ZEC,
            );

            let badge = BadgeResponse {
                platform: req.platform.clone(),
                username: req.username.clone(),
                badge_tier: tier.level(),
                badge_name: tier.to_string(),
                badge_image: tier.image_filename().to_string(),
                verified: true,
                expires_at: proof.expires.clone(),
            };

            let proof_json = serde_json::to_string(proof).unwrap_or_default();
            if let Err(e) = db::upsert_badge(
                &state.db,
                &req.platform,
                &req.username,
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
            {
                tracing::error!("DB error: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(VerifyResponse {
                        success: false,
                        message: format!("Database error: {}", e),
                        badge: None,
                    }),
                );
            }

            tracing::info!(
                "✅ Badge verified: {}:{} → {} ({})",
                req.platform, req.username, tier, tier.emoji()
            );

            (
                StatusCode::OK,
                Json(VerifyResponse {
                    success: true,
                    message: format!("Proof verified! Badge: {}", tier),
                    badge: Some(badge),
                }),
            )
        }
        Ok(vr) => (
            StatusCode::BAD_REQUEST,
            Json(VerifyResponse {
                success: false,
                message: format!("Proof invalid: {}", vr.message),
                badge: None,
            }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(VerifyResponse {
                success: false,
                message: format!("Verification error: {}", e),
                badge: None,
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
    tracing::info!("   GET  /api/badges      — Batch lookup");
    tracing::info!("   GET  /api/badge/:p/:u  — Single lookup");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
