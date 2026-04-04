// ── Zcash Badge Extension — Background Service Worker ──

const API_BASE = "http://localhost:3000";
const CACHE_TTL = 60 * 60 * 1000; // 1 hour

// In-memory cache for current session
const badgeCache = new Map();

// Listen for badge lookup requests from content scripts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === "LOOKUP_BADGES") {
    lookupBadges(message.platform, message.usernames)
      .then((badges) => sendResponse({ badges }))
      .catch((err) => {
        console.error("[ZcashBadge] Lookup error:", err);
        sendResponse({ badges: [] });
      });
    return true; // async response
  }

  if (message.type === "GET_SERVER_URL") {
    chrome.storage.local.get(["serverUrl"], (result) => {
      sendResponse({ serverUrl: result.serverUrl || API_BASE });
    });
    return true;
  }

  if (message.type === "SET_SERVER_URL") {
    chrome.storage.local.set({ serverUrl: message.serverUrl }, () => {
      sendResponse({ success: true });
    });
    return true;
  }
});

async function lookupBadges(platform, usernames) {
  if (!usernames || usernames.length === 0) return [];

  // Check cache first
  const uncached = [];
  const cached = [];
  const now = Date.now();

  for (const u of usernames) {
    const key = `${platform}:${u}`;
    const entry = badgeCache.get(key);
    if (entry && now - entry.timestamp < CACHE_TTL) {
      if (entry.badge) cached.push(entry.badge);
    } else {
      uncached.push(u);
    }
  }

  if (uncached.length === 0) return cached;

  // Fetch uncached from server
  try {
    const serverUrl = await getServerUrl();
    const url = `${serverUrl}/api/badges?platform=${encodeURIComponent(platform)}&usernames=${encodeURIComponent(uncached.join(","))}`;
    const resp = await fetch(url);
    if (!resp.ok) {
      console.warn("[ZcashBadge] Server returned", resp.status);
      return cached;
    }
    const badges = await resp.json();

    // Update cache
    const foundUsernames = new Set(badges.map((b) => b.username));
    for (const badge of badges) {
      const key = `${platform}:${badge.username}`;
      badgeCache.set(key, { badge, timestamp: now });
    }
    // Cache misses (no badge) so we don't re-query
    for (const u of uncached) {
      if (!foundUsernames.has(u)) {
        badgeCache.set(`${platform}:${u}`, { badge: null, timestamp: now });
      }
    }

    return [...cached, ...badges];
  } catch (e) {
    console.error("[ZcashBadge] Fetch error:", e);
    return cached;
  }
}

async function getServerUrl() {
  return new Promise((resolve) => {
    chrome.storage.local.get(["serverUrl"], (result) => {
      resolve(result.serverUrl || API_BASE);
    });
  });
}
