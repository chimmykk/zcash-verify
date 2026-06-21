// ── Zcash Badge Extension — Background Service Worker ──

const API_BASE = "https://voted-lamp-ben-contrary.trycloudflare.com";
const CACHE_TTL = 60 * 60 * 1000; // 1 hour

const badgeCache = new Map();

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === "LOOKUP_BADGES") {
    lookupBadges(message.platform, message.usernames)
      .then((badges) => sendResponse({ badges }))
      .catch((err) => {
        console.error("[ZcashBadge] Lookup error:", err);
        sendResponse({ badges: [] });
      });
    return true;
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

  if (message.type === "CLEAR_CACHE") {
    badgeCache.clear();
    broadcastRescan();
    sendResponse({ success: true });
    return true;
  }

  if (message.type === "SAVE_IDENTITIES") {
    chrome.storage.local.set({ identities: message.identities || {} }, () => {
      badgeCache.clear();
      broadcastRescan();
      sendResponse({ success: true });
    });
    return true;
  }
});

function normalizeUsername(platform, username) {
  const u = String(username || "")
    .trim()
    .replace(/^@/, "")
    .toLowerCase();
  if (!u) return u;
  if (platform === "bluesky" && !u.includes(".") && !u.startsWith("did:")) {
    return `${u}.bsky.social`;
  }
  return u;
}

function lookupKeys(platform, username) {
  const keys = new Set();
  const raw = String(username || "")
    .trim()
    .replace(/^@/, "")
    .toLowerCase();
  if (!raw) return keys;

  keys.add(raw);
  keys.add(normalizeUsername(platform, raw));

  if (platform === "bluesky") {
    if (raw.includes(".")) {
      keys.add(raw.split(".")[0]);
    } else if (!raw.startsWith("did:")) {
      keys.add(`${raw}.bsky.social`);
    }
  }

  return keys;
}

function expandUsernames(platform, usernames) {
  const expanded = new Set();
  for (const username of usernames) {
    for (const key of lookupKeys(platform, username)) {
      expanded.add(key);
    }
  }
  return [...expanded];
}

async function lookupBadges(platform, usernames) {
  if (!usernames || usernames.length === 0) return [];

  const uncached = [];
  const cached = [];
  const now = Date.now();

  for (const u of usernames) {
    const keys = lookupKeys(platform, u);
    let hit = null;
    for (const key of keys) {
      const cacheKey = `${platform}:${key}`;
      const entry = badgeCache.get(cacheKey);
      if (entry && now - entry.timestamp < CACHE_TTL) {
        if (entry.badge) hit = entry.badge;
        break;
      }
    }
    if (hit) {
      cached.push(hit);
    } else {
      uncached.push(u);
    }
  }

  const queryUsernames = expandUsernames(platform, uncached);
  if (queryUsernames.length === 0) return dedupeBadges(cached);

  try {
    const serverUrl = await getServerUrl();
    const url = `${serverUrl}/api/badges?platform=${encodeURIComponent(platform)}&usernames=${encodeURIComponent(queryUsernames.join(","))}`;
    const resp = await fetch(url);
    if (!resp.ok) {
      console.warn("[ZcashBadge] Server returned", resp.status);
      return dedupeBadges(cached);
    }
    const badges = await resp.json();

    for (const badge of badges) {
      const badgeUser = badge.username.toLowerCase();
      for (const key of lookupKeys(platform, badgeUser)) {
        badgeCache.set(`${platform}:${key}`, { badge, timestamp: now });
      }
    }

    for (const u of uncached) {
      const aliases = lookupKeys(platform, u);
      const found = badges.some((b) => aliases.has(b.username.toLowerCase()));
      if (!found) {
        for (const key of aliases) {
          badgeCache.set(`${platform}:${key}`, { badge: null, timestamp: now });
        }
      }
    }

    return dedupeBadges([...cached, ...badges]);
  } catch (e) {
    console.error("[ZcashBadge] Fetch error:", e);
    return dedupeBadges(cached);
  }
}

function dedupeBadges(badges) {
  const seen = new Set();
  const out = [];
  for (const badge of badges) {
    const key = `${badge.platform}:${badge.username.toLowerCase()}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(badge);
  }
  return out;
}

async function getServerUrl() {
  return new Promise((resolve) => {
    chrome.storage.local.get(["serverUrl"], (result) => {
      resolve(result.serverUrl || API_BASE);
    });
  });
}

function broadcastRescan() {
  chrome.tabs.query({}, (tabs) => {
    for (const tab of tabs) {
      if (tab.id) {
        chrome.tabs.sendMessage(tab.id, { type: "RESCAN_BADGES" }).catch(() => {});
      }
    }
  });
}
