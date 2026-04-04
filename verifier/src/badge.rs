use serde::{Deserialize, Serialize};
use std::fmt;

/// 1 ZEC = 100,000,000 zatoshis
pub const ZAT_PER_ZEC: u64 = 100_000_000;

/// Badge tier thresholds in ZEC.
/// Users are assigned the highest tier their balance meets.
///
/// Each tier maps to a badge shield image:
///   badge_holder.png    — < 1 ZEC (basic holder)
///   badge_10zec.png     — ≥ 10 ZEC
///   badge_100zec.png    — ≥ 100 ZEC
///   badge_1k_zec.png    — ≥ 1,000 ZEC
///   badge_10k_zec.png   — ≥ 10,000 ZEC
///   badge_100k_zec.png  — ≥ 100,000 ZEC
///   badge_1m_zec.png    — ≥ 1,000,000 ZEC
///   badge_10m_zec.png   — ≥ 10,000,000 ZEC
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BadgeTier {
    /// Balance < 1 ZEC — basic holder badge
    Holder,
    /// Balance ≥ 10 ZEC
    Zec10,
    /// Balance ≥ 100 ZEC
    Zec100,
    /// Balance ≥ 1,000 ZEC
    Zec1K,
    /// Balance ≥ 10,000 ZEC
    Zec10K,
    /// Balance ≥ 100,000 ZEC
    Zec100K,
    /// Balance ≥ 1,000,000 ZEC
    Zec1M,
    /// Balance ≥ 10,000,000 ZEC
    Zec10M,
}

impl BadgeTier {
    /// Determine the badge tier from a balance in zatoshis.
    pub fn from_balance(balance_zat: u64) -> Self {
        if balance_zat >= 10_000_000 * ZAT_PER_ZEC {
            BadgeTier::Zec10M
        } else if balance_zat >= 1_000_000 * ZAT_PER_ZEC {
            BadgeTier::Zec1M
        } else if balance_zat >= 100_000 * ZAT_PER_ZEC {
            BadgeTier::Zec100K
        } else if balance_zat >= 10_000 * ZAT_PER_ZEC {
            BadgeTier::Zec10K
        } else if balance_zat >= 1_000 * ZAT_PER_ZEC {
            BadgeTier::Zec1K
        } else if balance_zat >= 100 * ZAT_PER_ZEC {
            BadgeTier::Zec100
        } else if balance_zat >= 10 * ZAT_PER_ZEC {
            BadgeTier::Zec10
        } else {
            BadgeTier::Holder
        }
    }

    /// Returns the tier threshold in ZEC, or 0 for Holder.
    pub fn threshold_zec(&self) -> u64 {
        match self {
            BadgeTier::Holder => 0,
            BadgeTier::Zec10 => 10,
            BadgeTier::Zec100 => 100,
            BadgeTier::Zec1K => 1_000,
            BadgeTier::Zec10K => 10_000,
            BadgeTier::Zec100K => 100_000,
            BadgeTier::Zec1M => 1_000_000,
            BadgeTier::Zec10M => 10_000_000,
        }
    }

    /// Returns the tier threshold in zatoshis.
    pub fn threshold_zat(&self) -> u64 {
        self.threshold_zec() * ZAT_PER_ZEC
    }

    /// Emoji badge for display.
    pub fn emoji(&self) -> &'static str {
        match self {
            BadgeTier::Holder => "🛡️",
            BadgeTier::Zec10 => "🗡️",
            BadgeTier::Zec100 => "💎",
            BadgeTier::Zec1K => "⚔️",
            BadgeTier::Zec10K => "👑",
            BadgeTier::Zec100K => "🔥",
            BadgeTier::Zec1M => "🐉",
            BadgeTier::Zec10M => "✨",
        }
    }

    /// Returns the badge image filename.
    pub fn image_filename(&self) -> &'static str {
        match self {
            BadgeTier::Holder => "badge_holder.png",
            BadgeTier::Zec10 => "badge_10zec.png",
            BadgeTier::Zec100 => "badge_100zec.png",
            BadgeTier::Zec1K => "badge_1k_zec.png",
            BadgeTier::Zec10K => "badge_10k_zec.png",
            BadgeTier::Zec100K => "badge_100k_zec.png",
            BadgeTier::Zec1M => "badge_1m_zec.png",
            BadgeTier::Zec10M => "badge_10m_zec.png",
        }
    }

    /// Returns the tier level number (1-8).
    pub fn level(&self) -> u8 {
        match self {
            BadgeTier::Holder => 1,
            BadgeTier::Zec10 => 2,
            BadgeTier::Zec100 => 3,
            BadgeTier::Zec1K => 4,
            BadgeTier::Zec10K => 5,
            BadgeTier::Zec100K => 6,
            BadgeTier::Zec1M => 7,
            BadgeTier::Zec10M => 8,
        }
    }
}

impl fmt::Display for BadgeTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BadgeTier::Holder => write!(f, "Holder (< 1 ZEC)"),
            BadgeTier::Zec10 => write!(f, "≥10 ZEC"),
            BadgeTier::Zec100 => write!(f, "≥100 ZEC"),
            BadgeTier::Zec1K => write!(f, "≥1,000 ZEC"),
            BadgeTier::Zec10K => write!(f, "≥10,000 ZEC"),
            BadgeTier::Zec100K => write!(f, "≥100,000 ZEC"),
            BadgeTier::Zec1M => write!(f, "≥1,000,000 ZEC"),
            BadgeTier::Zec10M => write!(f, "≥10,000,000 ZEC"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_tiers() {
        assert_eq!(BadgeTier::from_balance(0), BadgeTier::Holder);
        assert_eq!(BadgeTier::from_balance(50_000_000), BadgeTier::Holder); // 0.5 ZEC
        assert_eq!(BadgeTier::from_balance(100_000_000), BadgeTier::Holder); // 1 ZEC (still holder)
        assert_eq!(BadgeTier::from_balance(999_999_999), BadgeTier::Holder); // 9.99 ZEC
        assert_eq!(BadgeTier::from_balance(1_000_000_000), BadgeTier::Zec10); // 10 ZEC
        assert_eq!(BadgeTier::from_balance(10_000_000_000), BadgeTier::Zec100); // 100 ZEC
        assert_eq!(BadgeTier::from_balance(100_000_000_000), BadgeTier::Zec1K); // 1,000 ZEC
        assert_eq!(BadgeTier::from_balance(1_000_000_000_000), BadgeTier::Zec10K); // 10,000 ZEC
        assert_eq!(BadgeTier::from_balance(10_000_000_000_000), BadgeTier::Zec100K); // 100,000 ZEC
        assert_eq!(BadgeTier::from_balance(100_000_000_000_000), BadgeTier::Zec1M); // 1,000,000 ZEC
        assert_eq!(BadgeTier::from_balance(1_000_000_000_000_000), BadgeTier::Zec10M); // 10,000,000 ZEC
    }

    #[test]
    fn test_threshold_roundtrip() {
        for tier in [
            BadgeTier::Zec10,
            BadgeTier::Zec100,
            BadgeTier::Zec1K,
            BadgeTier::Zec10K,
            BadgeTier::Zec100K,
            BadgeTier::Zec1M,
            BadgeTier::Zec10M,
        ] {
            assert_eq!(BadgeTier::from_balance(tier.threshold_zat()), tier);
        }
    }

    #[test]
    fn test_image_filenames() {
        assert_eq!(BadgeTier::Holder.image_filename(), "badge_holder.png");
        assert_eq!(BadgeTier::Zec10M.image_filename(), "badge_10m_zec.png");
    }

    #[test]
    fn test_levels() {
        assert_eq!(BadgeTier::Holder.level(), 1);
        assert_eq!(BadgeTier::Zec10M.level(), 8);
    }
}
