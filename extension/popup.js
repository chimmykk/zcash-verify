// ── ZcashBadge Extension — Popup Logic ──

const API_DEFAULT = "http://localhost:3000";
let serverUrl = API_DEFAULT;

document.addEventListener("DOMContentLoaded", () => {
  if (chrome?.storage) {
    chrome.storage.local.get(["serverUrl", "identities"], (r) => {
      serverUrl = r.serverUrl || API_DEFAULT;
      document.getElementById("server-url").value = serverUrl;
      loadMyBadges(r.identities || {});
    });
  } else {
    document.getElementById("server-url").value = API_DEFAULT;
  }

  document.querySelectorAll(".tab").forEach((btn) => {
    btn.addEventListener("click", () => switchTab(btn.dataset.tab));
  });

  document.getElementById("lookup-btn").addEventListener("click", lookupBadge);
  document.getElementById("lookup-username").addEventListener("keydown", (e) => {
    if (e.key === "Enter") lookupBadge();
  });

  document.getElementById("refresh-badges").addEventListener("click", () => {
    chrome.storage.local.get(["identities"], (r) => {
      clearCacheAndReload(r.identities || {});
    });
  });

  document.getElementById("save-url").addEventListener("click", () => {
    serverUrl = document.getElementById("server-url").value.trim();
    if (chrome?.storage) {
      chrome.storage.local.set({ serverUrl });
    }
    showStatus("Server URL saved", "success");
  });
});

function switchTab(id) {
  document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
  document.querySelectorAll(".panel").forEach((p) => p.classList.add("hidden"));
  document.querySelector(`[data-tab="${id}"]`).classList.add("active");
  document.getElementById(`tab-${id}`).classList.remove("hidden");
}

function showStatus(msg, type) {
  const el = document.getElementById("status");
  el.textContent = (type === "success" ? "✓ " : "✕ ") + msg;
  el.className = `status ${type}`;
  el.classList.remove("hidden");
  setTimeout(() => el.classList.add("hidden"), 3000);
}

function normalizeIdentity(platform, username) {
  const u = username.trim().replace(/^@/, "").toLowerCase();
  if (!u) return u;
  if (platform === "bluesky" && !u.includes(".") && !u.startsWith("did:")) {
    return `${u}.bsky.social`;
  }
  return u;
}

function clearCacheAndReload(identities) {
  chrome.runtime.sendMessage({ type: "CLEAR_CACHE" }, () => {
    loadMyBadges(identities);
  });
}

async function loadMyBadges(identities) {
  const entries = Object.entries(identities || {})
    .filter(([, username]) => username)
    .map(([platform, username]) => [platform, normalizeIdentity(platform, username)]);

  if (entries.length === 0) {
    document.getElementById("badge-list").classList.add("hidden");
    return;
  }

  const badges = [];
  for (const [platform, username] of entries) {
    try {
      const resp = await fetch(
        `${serverUrl}/api/badge/${platform}/${encodeURIComponent(username)}`
      );
      if (resp.ok) {
        badges.push(await resp.json());
      }
    } catch {
      // ignore per-platform failures
    }
  }

  if (badges.length > 0) {
    showBadges(badges);
  } else {
    document.getElementById("badge-list").classList.add("hidden");
    showStatus("No badges found — register at the web app", "error");
  }
}

function showBadges(badges) {
  const list = document.getElementById("badge-list");
  list.classList.remove("hidden");
  list.innerHTML = badges
    .map(
      (badge) => `
      <div class="badge-card">
        <img src="icons/badges/${badge.badge_image}" alt="" class="badge-card-img" />
        <div class="badge-card-info">
          <span class="badge-card-name">${badge.badge_name}</span>
          <span class="badge-card-user">${badge.platform}:${badge.username}</span>
          <span class="badge-card-expiry">Expires ${new Date(badge.expires_at).toLocaleDateString()}</span>
        </div>
        <span class="badge-card-tag">Verified</span>
      </div>
    `
    )
    .join("");
}

async function lookupBadge() {
  const platform = document.getElementById("lookup-platform").value;
  const username = normalizeIdentity(
    platform,
    document.getElementById("lookup-username").value
  );
  if (!username) return;

  const resultEl = document.getElementById("lookup-result");
  resultEl.classList.remove("hidden");
  resultEl.innerHTML = '<div class="lookup-empty">Searching...</div>';

  try {
    const resp = await fetch(
      `${serverUrl}/api/badge/${platform}/${encodeURIComponent(username)}`
    );
    if (resp.ok) {
      const badge = await resp.json();
      resultEl.innerHTML = `
        <div class="badge-card">
          <img src="icons/badges/${badge.badge_image}" alt="" class="badge-card-img" />
          <div class="badge-card-info">
            <span class="badge-card-name">${badge.badge_name}</span>
            <span class="badge-card-user">${badge.platform}:${badge.username}</span>
            <span class="badge-card-expiry">Expires ${new Date(badge.expires_at).toLocaleDateString()}</span>
          </div>
          <span class="badge-card-tag">Verified</span>
        </div>
      `;
    } else {
      resultEl.innerHTML = `<div class="lookup-empty">No badge found for <strong>${username}</strong> on ${platform}</div>`;
    }
  } catch {
    resultEl.innerHTML = '<div class="lookup-err">Could not reach the badge server</div>';
  }
}
