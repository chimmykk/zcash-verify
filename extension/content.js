// ── Zcash Badge Extension — Content Script ──
// Scans usernames on supported platforms and injects badge shields

(function () {
  "use strict";

  const BADGE_ATTR = "data-zcash-badge";
  const SCAN_INTERVAL = 3000; // Re-scan every 3s for SPA navigation
  const BATCH_DELAY = 500; // Debounce batch lookups

  let pendingUsernames = new Set();
  let batchTimer = null;

  // ── Platform Detection ──

  function detectPlatform() {
    const host = window.location.hostname;
    if (host.includes("x.com") || host.includes("twitter.com")) return "x";
    if (host.includes("forum.zcashcommunity.com")) return "zcashforum";
    if (host.includes("bsky.app")) return "bluesky";
    return null;
  }

  const platform = detectPlatform();
  if (!platform) return;

  function normalizeUsername(username) {
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

  function badgeLookupKeys(username) {
    const keys = new Set();
    const raw = normalizeUsername(username);
    keys.add(raw);
    if (platform === "bluesky" && raw.includes(".")) {
      keys.add(raw.split(".")[0]);
    }
    return keys;
  }

  console.log(`[ZcashBadge] Active on platform: ${platform}`);

  // ── Username Extraction Per Platform ──

  function extractUserElements() {
    const results = [];

    switch (platform) {
      case "x": {
        // X/Twitter — usernames in tweets and profiles
        const seenXContainers = new Set();
        
        document
          .querySelectorAll('a[href^="/"][role="link"], [data-testid="User-Name"] span, [data-testid="UserName"] span')
          .forEach((el) => {
            
            // 1. Handle profile top spans (not links)
            if (el.tagName === "SPAN") {
              const text = el.textContent.trim();
              if (text.startsWith("@")) {
                const username = text.replace("@", "").toLowerCase();
                const container = el.closest('[data-testid="User-Name"], [data-testid="UserName"]');
                if (container && !seenXContainers.has(container) && !el.querySelector(".zcash-badge-icon")) {
                  seenXContainers.add(container);
                  results.push({ element: el, username });
                }
              }
              return;
            }

            // 2. Handle links (tweet headers, mentions)
            const href = el.getAttribute("href");
            if (href && href.startsWith("/") && !href.includes("/", 1)) {
            const username = href.substring(1).replace("@", "").toLowerCase();
            if (!username || username.includes("/")) return;

              // De-duplicate by the parent header if it exists
              const container = el.closest('[data-testid="User-Name"], [data-testid="UserName"]') || el;
              if (!seenXContainers.has(container)) {
                seenXContainers.add(container);

                // Inject into the inner span that contains the text for cleanest flexbox layout
                let targetEl = el;
                const innerSpan = el.querySelector("div[dir] span, span");
                if (innerSpan && innerSpan.textContent.trim().length > 0) {
                  targetEl = innerSpan;
                }

                if (!targetEl.querySelector(".zcash-badge-icon")) {
                  results.push({ element: targetEl, username });
                }
              }
            }
          });
        break;
      }

      case "zcashforum": {
        // Discourse-based forum — only target the <a> inside username containers
        const seen = new Set();
        document
          .querySelectorAll(
            '.username a[href^="/u/"], .poster-name a[href^="/u/"], a.mention[href^="/u/"]'
          )
          .forEach((el) => {
            const href = el.getAttribute("href");
            const username = href.split("/u/")[1]?.split("/")[0]?.toLowerCase() || "";
            if (
              username &&
              !seen.has(username + ":" + el.closest(".topic-post, .latest-topic-list-item, li")?.id) &&
              !el.closest(`[${BADGE_ATTR}]`) &&
              !el.parentElement?.querySelector(".zcash-badge-icon")
            ) {
              seen.add(username + ":" + el.closest(".topic-post, .latest-topic-list-item, li")?.id);
              results.push({ element: el, username });
            }
          });
        break;
      }

      case "bluesky": {
        const seenBsky = new Set();

        function addBlueskyUser(element, username) {
          const normalized = normalizeUsername(username);
          if (!normalized || seenBsky.has(normalized)) return;
          if (element.closest(`[${BADGE_ATTR}]`)) return;
          if (element.querySelector(".zcash-badge-icon")) return;
          seenBsky.add(normalized);
          results.push({ element, username: normalized });
        }

        document.querySelectorAll('a[href*="/profile/"]').forEach((el) => {
          if (el.closest('nav, [data-testid="sidebarNav"], [role="navigation"]')) return;

          const text = el.textContent.trim();
          const href = el.getAttribute("href") || "";

          if (text.startsWith("@")) {
            addBlueskyUser(el, text);
            return;
          }

          const match = href.match(/\/profile\/([^/?#]+)/);
          if (!match) return;
          const slug = match[1].replace("@", "").toLowerCase();
          if (slug.startsWith("did:")) return;
          addBlueskyUser(el, slug);
        });

        document.querySelectorAll('[data-testid="profileHeaderHandle"]').forEach((el) => {
          addBlueskyUser(el, el.textContent.trim());
        });

        document.querySelectorAll('[data-testid="postAuthorHandle"]').forEach((el) => {
          addBlueskyUser(el, el.textContent.trim());
        });
        break;
      }
    }

    return results;
  }

  // Create a single global tooltip element attached to the body
  // This prevents any overflow:hidden or transform CSS on parent containers from breaking the tooltip
  const globalTooltip = document.createElement("div");
  globalTooltip.className = "zcash-badge-tooltip";
  document.body.appendChild(globalTooltip);

  function injectBadge(element, badge) {
    // Don't double-inject
    if (element.parentElement?.querySelector(".zcash-badge-icon")) return;
    if (element.querySelector(".zcash-badge-icon")) return;

    const img = document.createElement("img");
    img.className = "zcash-badge-icon";
    img.src = chrome.runtime.getURL(`icons/badges/${badge.badge_image}`);
    img.alt = badge.badge_name;
    img.setAttribute(BADGE_ATTR, badge.badge_tier);

    const wrapper = document.createElement("span");
    wrapper.className = "zcash-badge-wrapper";
    wrapper.setAttribute(BADGE_ATTR, badge.badge_tier);
    wrapper.appendChild(img);

    // Show tooltip dynamically on body
    wrapper.addEventListener("mouseenter", () => {
      globalTooltip.innerHTML = `
        <strong>Zcash Verified</strong><br/>
        <span class="zcash-badge-tier">${badge.badge_name}</span><br/>
        <span class="zcash-badge-user">${badge.platform}:${badge.username}</span><br/>
        <span class="zcash-badge-expires">Expires: ${new Date(badge.expires_at).toLocaleDateString()}</span>
      `;
      globalTooltip.style.display = "block";
      
      const rect = img.getBoundingClientRect();
      const scrollX = window.scrollX || document.documentElement.scrollLeft;
      const scrollY = window.scrollY || document.documentElement.scrollTop;
      
      // We calculate offsetWidth now that it's display:block
      globalTooltip.style.left = (rect.left + scrollX + rect.width / 2 - globalTooltip.offsetWidth / 2) + "px";
      globalTooltip.style.top = (rect.top + scrollY - globalTooltip.offsetHeight - 8) + "px";
      globalTooltip.classList.add("zcash-tooltip-visible");
    });

    wrapper.addEventListener("mouseleave", () => {
      globalTooltip.style.display = "none";
      globalTooltip.classList.remove("zcash-tooltip-visible");
    });

    // Insert inline
    element.appendChild(wrapper);
  }

  // ── Batch Lookup Logic ──

  function scheduleBatch(userElements) {
    for (const { username } of userElements) {
      pendingUsernames.add(username);
    }

    if (batchTimer) clearTimeout(batchTimer);
    batchTimer = setTimeout(() => {
      const usernames = [...pendingUsernames];
      pendingUsernames.clear();
      if (usernames.length === 0) return;

      // Chrome C++ bindings throw uncatchable errors if the extension was reloaded. 
      // Checking for chrome.runtime?.id safely verifies if the context is still alive!
      if (!chrome.runtime?.id) {
          console.log("[ZcashBadge] Extension reloaded. Auto-refreshing tab...");
          pendingUsernames.clear();
          if (batchTimer) clearTimeout(batchTimer);
          window.location.reload();
          return;
      }

      // Query background for badges
      try {
        chrome.runtime.sendMessage(
          { type: "LOOKUP_BADGES", platform, usernames },
          (response) => {
            // Ignore benign connection errors often caused by extension reload
            if (chrome.runtime.lastError) return;
            
            if (!response || !response.badges) return;

            // Build lookup map with handle aliases (e.g. bluesky short handle)
            const badgeMap = new Map();
            for (const b of response.badges) {
              for (const key of badgeLookupKeys(b.username)) {
                badgeMap.set(key, b);
              }
            }

            // Re-scan and inject
            const elements = extractUserElements();
            for (const { element, username } of elements) {
              const keys = badgeLookupKeys(username);
              let badge = null;
              for (const key of keys) {
                badge = badgeMap.get(key);
                if (badge) break;
              }
              if (badge) {
                injectBadge(element, badge);
              }
            }
          }
        );
      } catch (err) {
        if (err.message.includes("Extension context invalidated")) {
          console.log("[ZcashBadge] Extension context reloaded. Please refresh the page.");
          // Clear pending items to prevent infinite retry loops on dead context
          pendingUsernames.clear();
        }
      }
    }, BATCH_DELAY);
  }

  // ── Scanning Loop ──

  function scanPage() {
    const userElements = extractUserElements();
    if (userElements.length > 0) {
      scheduleBatch(userElements);
    }
  }

  // Initial scan
  scanPage();

  // MutationObserver for SPA navigation
  const observer = new MutationObserver((mutations) => {
    let shouldScan = false;
    for (const m of mutations) {
      if (m.addedNodes.length > 0) {
        shouldScan = true;
        break;
      }
    }
    if (shouldScan) {
      scanPage();
    }
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  // Periodic rescan as fallback
  setInterval(scanPage, SCAN_INTERVAL);

  chrome.runtime.onMessage.addListener((message) => {
    if (message.type === "RESCAN_BADGES") {
      pendingUsernames.clear();
      scanPage();
    }
  });

  console.log("[ZcashBadge] Content script initialized");
})();
