use crate::BadgeResponse;
use sqlx::SqlitePool;

/// Initialize the SQLite database and run migrations.
pub async fn init_db(url: &str) -> anyhow::Result<SqlitePool> {
    let filename = url.strip_prefix("sqlite:").unwrap_or(url);
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(filename)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await?;

    // Run migration (multiple statements)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS verified_badges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            platform TEXT NOT NULL,
            username TEXT NOT NULL,
            badge_tier INTEGER NOT NULL,
            badge_name TEXT NOT NULL,
            badge_image TEXT NOT NULL,
            proof_type TEXT NOT NULL,
            network TEXT NOT NULL,
            address TEXT NOT NULL,
            block_height INTEGER NOT NULL,
            verified_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            proof_json TEXT NOT NULL,
            UNIQUE(platform, username)
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_platform_username ON verified_badges(platform, username)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_expires ON verified_badges(expires_at)")
        .execute(&pool)
        .await?;

    tracing::info!("Database initialized");
    Ok(pool)
}

/// Insert or update a verified badge.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_badge(
    pool: &SqlitePool,
    platform: &str,
    username: &str,
    badge_tier: u8,
    badge_name: &str,
    badge_image: &str,
    proof_type: &str,
    network: &str,
    address: &str,
    block_height: i64,
    expires_at: &str,
    proof_json: &str,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO verified_badges
            (platform, username, badge_tier, badge_name, badge_image, proof_type, network, address, block_height, verified_at, expires_at, proof_json)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(platform, username) DO UPDATE SET
            badge_tier = excluded.badge_tier,
            badge_name = excluded.badge_name,
            badge_image = excluded.badge_image,
            proof_type = excluded.proof_type,
            network = excluded.network,
            address = excluded.address,
            block_height = excluded.block_height,
            verified_at = excluded.verified_at,
            expires_at = excluded.expires_at,
            proof_json = excluded.proof_json",
    )
    .bind(platform)
    .bind(username)
    .bind(badge_tier as i32)
    .bind(badge_name)
    .bind(badge_image)
    .bind(proof_type)
    .bind(network)
    .bind(address)
    .bind(block_height)
    .bind(&now)
    .bind(expires_at)
    .bind(proof_json)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get a single badge if it exists and hasn't expired.
pub async fn get_badge(
    pool: &SqlitePool,
    platform: &str,
    username: &str,
    now: &str,
) -> anyhow::Result<Option<BadgeResponse>> {
    let row = sqlx::query_as::<_, BadgeRow>(
        "SELECT platform, username, badge_tier, badge_name, badge_image, expires_at
         FROM verified_badges
         WHERE platform = ? AND username = ? AND expires_at > ?",
    )
    .bind(platform)
    .bind(username)
    .bind(now)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| BadgeResponse {
        platform: r.platform,
        username: r.username,
        badge_tier: r.badge_tier as u8,
        badge_name: r.badge_name,
        badge_image: r.badge_image,
        verified: true,
        expires_at: r.expires_at,
    }))
}

#[derive(sqlx::FromRow)]
struct BadgeRow {
    platform: String,
    username: String,
    badge_tier: i32,
    badge_name: String,
    badge_image: String,
    expires_at: String,
}
