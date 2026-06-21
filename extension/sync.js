// Syncs identities saved by the web app into extension storage.
(function () {
  "use strict";

  const STORAGE_KEY = "zcashbadge_identities";
  const LEGACY_KEY = "zcashverify_identities";

  function syncIdentities() {
    try {
      const raw =
        localStorage.getItem(STORAGE_KEY) || localStorage.getItem(LEGACY_KEY);
      if (!raw) return;

      const identities = JSON.parse(raw);
      if (!identities || typeof identities !== "object") return;

      chrome.storage.local.get(["identities"], (result) => {
        const current = result.identities || {};
        const merged = { ...current, ...identities };
        chrome.storage.local.set({ identities: merged }, () => {
          chrome.runtime.sendMessage({ type: "CLEAR_CACHE" });
        });
      });
    } catch (err) {
      console.warn("[ZcashBadge] Could not sync identities:", err);
    }
  }

  syncIdentities();

  window.addEventListener("storage", (event) => {
    if (event.key === STORAGE_KEY || event.key === LEGACY_KEY) syncIdentities();
  });
})();
