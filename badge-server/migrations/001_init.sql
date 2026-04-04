-- ZcashVerify Badge Server — Initial Schema
-- Run: sqlite3 badges.db < 001_init.sql

CREATE TABLE IF NOT EXISTS verified_badges (
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
);

CREATE INDEX IF NOT EXISTS idx_platform_username ON verified_badges(platform, username);
CREATE INDEX IF NOT EXISTS idx_expires ON verified_badges(expires_at);
