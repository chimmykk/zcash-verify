// ── ZcashVerify Extension — Popup Logic ──

const API_DEFAULT = "http://localhost:3000";
let serverUrl = API_DEFAULT;
let loadedProof = null;

// ── Init ──
document.addEventListener("DOMContentLoaded", () => {
  // Load server URL
  if (chrome?.storage) {
    chrome.storage.local.get(["serverUrl", "myBadges"], (r) => {
      if (r.serverUrl) {
        serverUrl = r.serverUrl;
        document.getElementById("server-url").value = serverUrl;
      } else {
        document.getElementById("server-url").value = API_DEFAULT;
      }
      if (r.myBadges && Object.keys(r.myBadges).length > 0) showBadges(r.myBadges);
    });
  } else {
    document.getElementById("server-url").value = API_DEFAULT;
  }

  // Tab switching
  document.querySelectorAll(".tab").forEach((btn) => {
    btn.addEventListener("click", () => switchTab(btn.dataset.tab));
  });

  // Empty state submit button
  document.getElementById("empty-submit-btn").addEventListener("click", () => switchTab("submit"));

  // File upload
  document.getElementById("file-input").addEventListener("change", handleFile);

  // Submit proof
  document.getElementById("submit-btn").addEventListener("click", submitProof);

  // Lookup
  document.getElementById("lookup-btn").addEventListener("click", lookupBadge);
  document.getElementById("lookup-username").addEventListener("keydown", (e) => {
    if (e.key === "Enter") lookupBadge();
  });

  // Save URL
  document.getElementById("save-url").addEventListener("click", () => {
    serverUrl = document.getElementById("server-url").value.trim();
    if (chrome?.storage) {
      chrome.storage.local.set({ serverUrl });
    }
    showStatus("Server URL saved", "success");
  });

  // Enable submit button when form is ready
  document.getElementById("submit-username").addEventListener("input", checkSubmitReady);
});

// ── Tab Switching ──
function switchTab(id) {
  document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
  document.querySelectorAll(".panel").forEach((p) => p.classList.add("hidden"));
  document.querySelector(`[data-tab="${id}"]`).classList.add("active");
  document.getElementById(`tab-${id}`).classList.remove("hidden");
}

// ── Status ──
function showStatus(msg, type) {
  const el = document.getElementById("status");
  el.textContent = (type === "success" ? "✓ " : "✕ ") + msg;
  el.className = `status ${type}`;
  el.classList.remove("hidden");
  setTimeout(() => el.classList.add("hidden"), 3000);
}

// ── File Upload ──
function handleFile(e) {
  const file = e.target.files[0];
  if (!file) return;

  const reader = new FileReader();
  reader.onload = (ev) => {
    try {
      loadedProof = JSON.parse(ev.target.result);
      document.getElementById("proof-loaded").classList.remove("hidden");
      document.getElementById("file-drop").querySelector(".file-drop-text").textContent = file.name;

      // Auto-fill platform and username from proof
      if (loadedProof.platform) {
        document.getElementById("submit-platform").value = loadedProof.platform;
      }
      if (loadedProof.username) {
        document.getElementById("submit-username").value = loadedProof.username;
      }

      checkSubmitReady();
      showStatus("Proof loaded", "success");
    } catch {
      showStatus("Invalid JSON file", "error");
    }
  };
  reader.readAsText(file);
}

function checkSubmitReady() {
  const ready = loadedProof && document.getElementById("submit-username").value.trim();
  document.getElementById("submit-btn").disabled = !ready;
}

// ── Submit Proof ──
async function submitProof() {
  if (!loadedProof) return;
  const platform = document.getElementById("submit-platform").value;
  const username = document.getElementById("submit-username").value.trim();
  if (!username) return;

  const btn = document.getElementById("submit-btn");
  btn.disabled = true;
  btn.textContent = "Submitting...";

  try {
    const resp = await fetch(`${serverUrl}/api/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ proof: loadedProof, platform, username }),
    });

    const data = await resp.json();
    if (data.success && data.badge) {
      if (chrome?.storage) {
        chrome.storage.local.get(["myBadges"], (res) => {
          const myBadges = res.myBadges || {};
          myBadges[platform] = data.badge;
          chrome.storage.local.set({ myBadges });
          showBadges(myBadges);
        });
      } else {
        showBadges({ [platform]: data.badge });
      }
      showStatus(data.message, "success");
      switchTab("home");
    } else {
      showStatus(data.message || "Verification failed", "error");
    }
  } catch (err) {
    showStatus("Cannot reach server: " + err.message, "error");
  }

  btn.disabled = false;
  btn.textContent = "Submit & Verify";
  checkSubmitReady();
}

// ── Show Badge on Home ──
function showBadges(myBadges) {
  document.getElementById("empty-state").classList.add("hidden");
  const display = document.getElementById("badge-display");
  display.classList.remove("hidden");

  // Get generic tier/image info from the first available badge
  const firstBadge = Object.values(myBadges)[0];

  document.getElementById("badge-img").src = `icons/badges/${firstBadge.badge_image}`;
  document.getElementById("badge-name").textContent = firstBadge.badge_name;
  
  const usersContainer = document.getElementById("badge-users");
  usersContainer.innerHTML = "";
  for (const [plat, badge] of Object.entries(myBadges)) {
    const el = document.createElement("div");
    el.textContent = `${plat}: ${badge.username}`;
    el.style.color = "#a1a1aa"; // subtle gray
    usersContainer.appendChild(el);
  }

  document.getElementById("badge-expiry").textContent =
    "Expires " + new Date(firstBadge.expires_at).toLocaleDateString();
}

// ── Lookup ──
async function lookupBadge() {
  const platform = document.getElementById("lookup-platform").value;
  const username = document.getElementById("lookup-username").value.trim();
  if (!username) return;

  const resultEl = document.getElementById("lookup-result");
  resultEl.classList.remove("hidden");
  resultEl.innerHTML = '<div class="lookup-empty">Searching...</div>';

  try {
    const resp = await fetch(`${serverUrl}/api/badge/${platform}/${username}`);
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
