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
              if (!username) return;

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
        // Bluesky — match @handle links in posts, skip sidebar/nav
        const seenBsky = new Set();
        document
          .querySelectorAll('a[href*="/profile/"]')
          .forEach((el) => {
            // Skip nav/sidebar links (they have role="link" in nav, or are inside nav-like containers)
            if (el.closest('nav, [data-testid="sidebarNav"], [role="navigation"]')) return;
            // Only match links whose text looks like a handle (@something)
            const text = el.textContent.trim();
            const href = el.getAttribute("href") || "";
            const match = href.match(/\/profile\/([^/?#]+)/);
            if (!match) return;
            const username = match[1].replace("@", "").toLowerCase();
            // Skip if it's a generic nav link (text is "Profile", "Home", etc.)
            if (!text.includes("@") && !text.includes(".") && !text.toLowerCase().includes(username.toLowerCase())) return;
            if (
              !seenBsky.has(username) &&
              !el.closest(`[${BADGE_ATTR}]`) &&
              !el.querySelector(".zcash-badge-icon")
            ) {
              seenBsky.add(username);
              results.push({ element: el, username });
            }
          });

        // Add back profile page top handler for Bluesky
        document
          .querySelectorAll('[data-testid="profileHeaderDisplayName"]')
          .forEach((el) => {
            const handleEl = el
              .closest("[data-testid]")
              ?.querySelector('[data-testid="profileHeaderHandle"]');
            if (handleEl && !handleEl.closest(`[${BADGE_ATTR}]`) && !handleEl.querySelector(".zcash-badge-icon")) {
              const username = handleEl.textContent.trim().replace("@", "").toLowerCase();
              if (username && !seenBsky.has(username)) {
                seenBsky.add(username);
                results.push({ element: handleEl, username });
              }
            }
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

            // Build lookup map
            const badgeMap = new Map();
            for (const b of response.badges) {
              badgeMap.set(b.username.toLowerCase(), b);
            }

            // Re-scan and inject
            const elements = extractUserElements();
            for (const { element, username } of elements) {
              const badge = badgeMap.get(username);
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

  console.log("[ZcashBadge] Content script initialized");
})();
