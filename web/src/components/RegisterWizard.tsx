"use client";

import { useMemo, useState } from "react";
import {
  Badge,
  formatZec,
  registerBadges,
  scanBalance,
  ScanResponse,
} from "@/lib/api";

const DEFAULT_START_HEIGHT = "3385575";

type Step = "wallet" | "identity" | "review" | "result";

const STEPS: { id: Step; label: string }[] = [
  { id: "wallet", label: "Wallet" },
  { id: "identity", label: "Identity" },
  { id: "review", label: "Review" },
  { id: "result", label: "Done" },
];

const PLATFORMS = [
  {
    key: "x" as const,
    label: "X (Twitter)",
    placeholder: "your_handle",
    hint: "Without the @ symbol",
  },
  {
    key: "bluesky" as const,
    label: "Bluesky",
    placeholder: "handle.bsky.social",
    hint: "Your full Bluesky handle",
  },
  {
    key: "zcashforum" as const,
    label: "Zcash Forum",
    placeholder: "forum_username",
    hint: "Your forum.zcashcommunity.com username",
  },
];

export default function RegisterWizard() {
  const [step, setStep] = useState<Step>("wallet");
  const [seed, setSeed] = useState("");
  const [startHeight, setStartHeight] = useState(DEFAULT_START_HEIGHT);
  const [network, setNetwork] = useState("main");
  const [handles, setHandles] = useState({ x: "", bluesky: "", zcashforum: "" });
  const [scanResult, setScanResult] = useState<ScanResponse | null>(null);
  const [badges, setBadges] = useState<Badge[]>([]);
  const [resultMessage, setResultMessage] = useState("");
  const [loading, setLoading] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState("");

  const wordCount = useMemo(
    () => seed.trim().split(/\s+/).filter(Boolean).length,
    [seed]
  );

  const hasIdentity = Object.values(handles).some((h) => h.trim());
  const seedReady = wordCount === 12 || wordCount === 24;

  async function runScan(): Promise<ScanResponse | null> {
    setError("");
    setScanning(true);
    try {
      const result = await scanBalance({
        seed: seed.trim(),
        start_height: startHeight ? Number(startHeight) : undefined,
        network,
      });
      setScanResult(result);
      if (!result.success) {
        setError(result.message || "Scan failed");
        return null;
      }
      return result;
    } catch (err) {
      setScanResult(null);
      setError(err instanceof Error ? err.message : "Scan failed");
      return null;
    } finally {
      setScanning(false);
    }
  }

  async function handleScan() {
    await runScan();
  }

  async function handleContinueFromWallet() {
    if (scanResult?.success) {
      setStep("identity");
      return;
    }
    const result = await runScan();
    if (result?.success) {
      setStep("identity");
    }
  }

  async function handleRegister() {
    setError("");
    setLoading(true);
    try {
      const result = await registerBadges({
        seed: seed.trim(),
        start_height: startHeight ? Number(startHeight) : undefined,
        network,
        x: handles.x.trim() || undefined,
        bluesky: handles.bluesky.trim() || undefined,
        zcashforum: handles.zcashforum.trim() || undefined,
      });
      setBadges(result.badges);
      setResultMessage(result.message);

      const identities: Record<string, string> = {};
      if (handles.x.trim()) identities.x = handles.x.trim().replace(/^@/, "").toLowerCase();
      if (handles.zcashforum.trim()) {
        identities.zcashforum = handles.zcashforum.trim().replace(/^@/, "").toLowerCase();
      }
      if (handles.bluesky.trim()) {
        let bsky = handles.bluesky.trim().replace(/^@/, "").toLowerCase();
        if (!bsky.includes(".") && !bsky.startsWith("did:")) {
          bsky = `${bsky}.bsky.social`;
        }
        identities.bluesky = bsky;
      }
      if (typeof window !== "undefined" && Object.keys(identities).length > 0) {
        localStorage.setItem("zcashbadge_identities", JSON.stringify(identities));
      }

      setStep("result");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Registration failed");
    } finally {
      setLoading(false);
    }
  }

  function goNext() {
    setError("");
    if (step === "wallet") setStep("identity");
    else if (step === "identity") setStep("review");
  }

  function goBack() {
    setError("");
    if (step === "identity") setStep("wallet");
    else if (step === "review") setStep("identity");
  }

  return (
    <div className="mx-auto w-full max-w-2xl">
      <div className="mb-8 flex items-center justify-between gap-2">
        {STEPS.map((s, i) => {
          const active = s.id === step;
          const done = STEPS.findIndex((x) => x.id === step) > i;
          return (
            <div key={s.id} className="flex flex-1 items-center gap-2">
              <div
                className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-sm font-semibold ${
                  active
                    ? "bg-emerald-500 text-black"
                    : done
                      ? "bg-emerald-500/20 text-emerald-300"
                      : "bg-zinc-800 text-zinc-500"
                }`}
              >
                {done ? "✓" : i + 1}
              </div>
              <span
                className={`hidden text-sm sm:inline ${
                  active ? "text-white" : "text-zinc-500"
                }`}
              >
                {s.label}
              </span>
              {i < STEPS.length - 1 && (
                <div className="mx-1 hidden h-px flex-1 bg-zinc-800 sm:block" />
              )}
            </div>
          );
        })}
      </div>

      {error && (
        <div className="mb-6 rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-200">
          {error}
        </div>
      )}

      {step === "wallet" && (
        <section className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6 shadow-xl">
          <h2 className="text-xl font-semibold text-white">Connect your wallet</h2>
          <p className="mt-2 text-sm text-zinc-400">
            Enter your BIP39 seed phrase. The Rust verifier scans your Orchard
            shielded balance via lightwalletd (same logic as{" "}
            <code className="text-zinc-300">verifier scan orchard</code>).
          </p>

          <div className="mt-4 rounded-xl border border-amber-500/20 bg-amber-500/10 px-4 py-3 text-sm text-amber-100">
            Your seed is sent only to your local badge server on{" "}
            <code className="rounded bg-black/30 px-1">localhost:3000</code>. Never
            use this on a public server you do not control.
          </div>

          <label className="mt-6 block text-sm font-medium text-zinc-300">
            Seed phrase
            <textarea
              value={seed}
              onChange={(e) => setSeed(e.target.value)}
              rows={3}
              placeholder="word1 word2 word3 ..."
              className="mt-2 w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 text-sm text-white outline-none ring-emerald-500/0 transition focus:border-emerald-500/50 focus:ring-2 focus:ring-emerald-500/20"
            />
          </label>
          <p className="mt-2 text-xs text-zinc-500">
            {wordCount > 0 ? `${wordCount} words entered` : "Typically 12 or 24 words"}
          </p>

          <div className="mt-6 grid gap-4 sm:grid-cols-2">
            <label className="block text-sm font-medium text-zinc-300">
              Scan start height
              <input
                value={startHeight}
                onChange={(e) => {
                  setStartHeight(e.target.value);
                  setScanResult(null);
                }}
                type="number"
                placeholder={DEFAULT_START_HEIGHT}
                className="mt-2 w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 text-sm text-white outline-none focus:border-emerald-500/50"
              />
              <span className="mt-1 block text-xs font-normal text-zinc-500">
                Defaults to current mainnet height — lower this to the block before
                your first shielded receive for balance scanning
              </span>
            </label>
            <label className="block text-sm font-medium text-zinc-300">
              Network
              <select
                value={network}
                onChange={(e) => setNetwork(e.target.value)}
                className="mt-2 w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 text-sm text-white outline-none focus:border-emerald-500/50"
              >
                <option value="main">Mainnet</option>
                <option value="test">Testnet</option>
              </select>
            </label>
          </div>

          <div className="mt-6 flex flex-wrap gap-3">
            <button
              type="button"
              onClick={handleScan}
              disabled={scanning || loading || !seedReady || !startHeight}
              className="rounded-xl border border-zinc-700 px-4 py-2.5 text-sm font-medium text-zinc-200 transition hover:bg-zinc-800 disabled:cursor-not-allowed disabled:opacity-40"
            >
              {scanning ? "Scanning ZEC balance..." : "Scan balance"}
            </button>
            <button
              type="button"
              onClick={handleContinueFromWallet}
              disabled={scanning || loading || !seedReady || !startHeight}
              className="rounded-xl bg-emerald-500 px-5 py-2.5 text-sm font-semibold text-black transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-40"
            >
              {scanning ? "Scanning..." : "Continue"}
            </button>
          </div>

          {scanning && (
            <div className="mt-6 rounded-xl border border-zinc-700 bg-zinc-950/80 p-4">
              <p className="text-sm font-medium text-white">
                Scanning Orchard balance on the Zcash blockchain...
              </p>
              <p className="mt-2 text-sm text-zinc-400">
                This uses the verifier&apos;s shielded scanner and can take several
                minutes depending on start height. Keep this tab open.
              </p>
            </div>
          )}

          {scanResult?.success && scanResult.balance_zat !== undefined && (
            <div className="mt-6 rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-4">
              <p className="text-sm text-emerald-100">
                Balance:{" "}
                <span className="font-semibold">
                  {scanResult.balance_zec || formatZec(scanResult.balance_zat)}
                </span>
              </p>
              <p className="mt-1 text-sm text-emerald-200/80">
                Badge tier: {scanResult.badge_name || scanResult.badge_tier}
              </p>
              {scanResult.block_height && (
                <p className="mt-1 text-sm text-emerald-200/80">
                  Chain height: {scanResult.block_height.toLocaleString()}
                </p>
              )}
              {scanResult.address && (
                <p className="mt-2 break-all font-mono text-xs text-emerald-200/70">
                  {scanResult.address}
                </p>
              )}
            </div>
          )}
        </section>
      )}

      {step === "identity" && (
        <section className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6 shadow-xl">
          <h2 className="text-xl font-semibold text-white">Link social profiles</h2>
          <p className="mt-2 text-sm text-zinc-400">
            Add at least one handle. Your proof will be bound to each identity and
            stored in the badge database for the Chrome extension to display.
          </p>

          <div className="mt-6 space-y-5">
            {PLATFORMS.map((platform) => (
              <label
                key={platform.key}
                className="block text-sm font-medium text-zinc-300"
              >
                {platform.label}
                <input
                  value={handles[platform.key]}
                  onChange={(e) =>
                    setHandles((prev) => ({
                      ...prev,
                      [platform.key]: e.target.value,
                    }))
                  }
                  placeholder={platform.placeholder}
                  className="mt-2 w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 text-sm text-white outline-none focus:border-emerald-500/50"
                />
                <span className="mt-1 block text-xs font-normal text-zinc-500">
                  {platform.hint}
                </span>
              </label>
            ))}
          </div>

          <div className="mt-8 flex gap-3">
            <button
              type="button"
              onClick={goBack}
              className="rounded-xl border border-zinc-700 px-4 py-2.5 text-sm font-medium text-zinc-200 hover:bg-zinc-800"
            >
              Back
            </button>
            <button
              type="button"
              onClick={goNext}
              disabled={!hasIdentity}
              className="rounded-xl bg-emerald-500 px-5 py-2.5 text-sm font-semibold text-black hover:bg-emerald-400 disabled:opacity-40"
            >
              Continue
            </button>
          </div>
        </section>
      )}

      {step === "review" && (
        <section className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6 shadow-xl">
          <h2 className="text-xl font-semibold text-white">Review & register</h2>
          <p className="mt-2 text-sm text-zinc-400">
            We will scan your Orchard wallet, generate proofs, verify them, and save
            badges to the database. The extension reads from there automatically.
          </p>

          <div className="mt-6 space-y-3 rounded-xl border border-zinc-800 bg-zinc-950/80 p-4 text-sm">
            <div className="flex justify-between gap-4">
              <span className="text-zinc-500">Network</span>
              <span className="text-zinc-200">{network}</span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-zinc-500">Start height</span>
              <span className="text-zinc-200">{startHeight || "auto"}</span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-zinc-500">Seed words</span>
              <span className="text-zinc-200">{wordCount}</span>
            </div>
            {scanResult?.success && scanResult.balance_zat !== undefined && (
              <>
                <div className="flex justify-between gap-4">
                  <span className="text-zinc-500">Balance</span>
                  <span className="text-zinc-200">
                    {scanResult.balance_zec || formatZec(scanResult.balance_zat)}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-zinc-500">Badge tier</span>
                  <span className="text-zinc-200">
                    {scanResult.badge_name || scanResult.badge_tier}
                  </span>
                </div>
              </>
            )}
            {Object.entries(handles)
              .filter(([, v]) => v.trim())
              .map(([platform, username]) => (
                <div key={platform} className="flex justify-between gap-4">
                  <span className="text-zinc-500">{platform}</span>
                  <span className="font-mono text-emerald-300">{username}</span>
                </div>
              ))}
          </div>

          <div className="mt-8 flex gap-3">
            <button
              type="button"
              onClick={goBack}
              className="rounded-xl border border-zinc-700 px-4 py-2.5 text-sm font-medium text-zinc-200 hover:bg-zinc-800"
            >
              Back
            </button>
            <button
              type="button"
              onClick={handleRegister}
              disabled={loading || !startHeight}
              className="rounded-xl bg-emerald-500 px-5 py-2.5 text-sm font-semibold text-black hover:bg-emerald-400 disabled:opacity-60"
            >
              {loading ? "Generating proof..." : "Generate & register badges"}
            </button>
          </div>

          {loading && (
            <p className="mt-4 text-sm text-zinc-400">
              Scanning the blockchain can take a minute. Keep this tab open.
            </p>
          )}
        </section>
      )}

      {step === "result" && (
        <section className="rounded-2xl border border-emerald-500/20 bg-emerald-500/5 p-6 shadow-xl">
          <div className="flex items-start gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-full bg-emerald-500 text-2xl text-black">
              ✓
            </div>
            <div>
              <h2 className="text-xl font-semibold text-white">Badges registered</h2>
              <p className="mt-2 text-sm text-zinc-300">{resultMessage}</p>
            </div>
          </div>

          <div className="mt-6 space-y-3">
            {badges.map((badge) => (
              <div
                key={`${badge.platform}:${badge.username}`}
                className="flex items-center justify-between rounded-xl border border-zinc-800 bg-zinc-950/80 px-4 py-3"
              >
                <div>
                  <p className="font-medium text-white">{badge.badge_name}</p>
                  <p className="text-sm text-zinc-400">
                    {badge.platform}:{badge.username}
                  </p>
                </div>
                <span className="rounded-full bg-emerald-500/15 px-3 py-1 text-xs font-medium text-emerald-300">
                  Verified
                </span>
              </div>
            ))}
          </div>

          <div className="mt-8 rounded-xl border border-zinc-800 bg-zinc-950/80 p-4">
            <h3 className="font-medium text-white">Next steps</h3>
            <ol className="mt-3 list-decimal space-y-2 pl-5 text-sm text-zinc-400">
              <li>
                Open the ZcashBadge extension — your handles sync from the web app
              </li>
              <li>
                Visit X, Bluesky, or Zcash Forum — badges appear next to your username
              </li>
            </ol>
          </div>

          <button
            type="button"
            onClick={() => {
              setStep("wallet");
              setSeed("");
              setHandles({ x: "", bluesky: "", zcashforum: "" });
              setScanResult(null);
              setBadges([]);
            }}
            className="mt-6 rounded-xl border border-zinc-700 px-4 py-2.5 text-sm font-medium text-zinc-200 hover:bg-zinc-800"
          >
            Register another identity
          </button>
        </section>
      )}
    </div>
  );
}
