# ZcashBadge

Cryptographic proof-of-balance verification for Zcash. Users prove ownership of an Orchard (shielded) wallet and register verified balance badges on social platforms (X, Zcash Forum, Bluesky). A Chrome extension reads badges from the server and displays them next to usernames.

## Showcase

<p align="center">
  <img src="https://github.com/chimmykk/zcash-verify/raw/main/showcase-example/showcase_1.png" width="45%" style="margin: 5px;" alt="Showcase 1" />
  <img src="https://github.com/chimmykk/zcash-verify/raw/main/showcase-example/showcase_2.png" width="45%" style="margin: 5px;" alt="Showcase 2" />
</p>
<p align="center">
  <img src="https://github.com/chimmykk/zcash-verify/raw/main/showcase-example/showcase_3.png" width="45%" style="margin: 5px;" alt="Showcase 3" />
  <img src="https://github.com/chimmykk/zcash-verify/raw/main/showcase-example/showcase_4.png" width="45%" style="margin: 5px;" alt="Showcase 4" />
</p>

## Architecture

```
verifier/         Rust CLI + library — Orchard scanning, proof generation, verification
badge-server/     HTTP API on :3000 — generates/verifies proofs, stores badges in SQLite
web/              Next.js app on :3001 — registration wizard (recommended UI)
extension/        Chrome extension (MV3) — reads badges from DB, injects shields on social sites
```

### How it works

1. **Register** — The web app (or CLI) scans your Orchard balance via lightwalletd, generates a signed proof bound to your social handles, and sends it to the badge server.
2. **Verify & store** — The badge server cryptographically verifies each proof and upserts badge metadata into `badges.db`.
3. **Display** — The Chrome extension queries the badge server and injects tier badges next to matching usernames on X, Bluesky, and Zcash Forum.

No JSON upload is required in the extension. Badges are read from the database by `platform:username`.

## Prerequisites

- Rust 1.75+ and Cargo
- Node.js 18+ and npm (for the web app)
- Google Chrome, Brave, or another Chromium browser (for the extension)

## Quick Start

### 1. Build

```bash
cargo build
```

For optimized binaries:

```bash
cargo build --release
```

Binaries: `target/debug/verifier`, `target/debug/badge-server` (or under `target/release/`).

Install web dependencies once:

```bash
cd web && npm install
```

### 2. Start services

Helper scripts in the repo root:

| Script | Purpose |
| ------ | ------- |
| `./start_server.sh` | Badge server at `http://localhost:3000` |
| `./start_web.sh` | Registration app at `http://localhost:3001` |
| `./kill_server.sh` | Stop the badge server |
| `./clean.sh` | Stop server and wipe `badges.db` |

```bash
./start_server.sh   # terminal 1
./start_web.sh      # terminal 2
```

### 3. Register your badge (web app — recommended)

Open **http://localhost:3001** and complete the wizard:

1. **Wallet** — Enter your BIP39 seed phrase. Scan start height defaults to the current mainnet height (fetched from [Blockchair](https://api.blockchair.com/zcash/stats)). Lower it to the block **before** your first shielded receive so the scanner finds your balance.
2. **Identity** — Add handles for X, Bluesky, and/or Zcash Forum (at least one).
3. **Review** — Click **Generate & register badges**.

Proofs are verified and saved to SQLite automatically. Your handles are synced to the Chrome extension when you visit the web app.

> **Local dev only:** The web app sends your seed to your local badge server (`localhost:3000`) for proof generation. Do not expose this setup to the public internet without hardening it first.

### 4. Install the Chrome extension

1. Open `chrome://extensions`
2. Enable **Developer mode**
3. Click **Load unpacked** and select the `extension/` directory
4. Open the **ZcashBadge** popup — use **Open Registration App** if you still need to register

After registration, reload social tabs (X, Bluesky, Forum). Badges appear next to verified usernames. Use **Refresh from server** in the popup to reload your own badges.

### 5. Register via CLI (alternative)

```bash
cargo run -p zcash-verifier -- register \
  --seed "your seed phrase" \
  --x your_handle \
  --bluesky your_handle.bsky.social \
  --start-height 3295000
```

Or use the helper script (edit handles and start height first):

```bash
./prove_social.sh
```

Set `--start-height` to a block shortly **before** your wallet received its first shielded ZEC.

## Deploy on Ubuntu (Cloudflare Tunnel)

Expose the badge server and registration app publicly with a single script (uses **GNU screen** + Cloudflare quick tunnels):

```bash
chmod +x deployscript.sh
./deployscript.sh
```

This will:

1. Build and start **badge-server** in screen `zcashbadge-server` (port **3000**)
2. Build and start the **web app** in screen `zcashbadge-web` (port **3001**)
3. Expose the badge server via Cloudflare tunnel (for the Chrome extension)
4. Expose the web app via Cloudflare tunnel (registration page)
5. Print and save:

```
serverurlforextension: https://....trycloudflare.com
webpage access:        https://....trycloudflare.com
```

URLs are saved to `.deploy/urls.env`.

**Extension setup after deploy:**

1. Copy `serverurlforextension` into ZcashBadge extension → **Settings** → **Badge Server URL**
2. Open `webpage access` in a browser to register badges

**Manage deploy:**

```bash
./deployscript.sh --status   # list screens + saved URLs
screen -r zcashbadge-server  # attach to badge-server logs
./deployscript.sh --stop       # stop everything
```

**Requirements on Ubuntu:** `curl`, `lsof`, Rust/Cargo, Node.js/npm, and [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/).

Quick tunnel URLs change on each restart. For a stable domain, configure a named Cloudflare tunnel separately.

## Badge Server API

| Method | Path | Description |
| ------ | ---- | ----------- |
| `POST` | `/api/register` | Scan wallet, generate proofs, verify, and store badges |
| `POST` | `/api/scan` | Scan Orchard balance only (no registration) |
| `POST` | `/api/verify` | Submit an existing proof JSON for verification |
| `GET` | `/api/badges?platform=x&usernames=u1,u2` | Batch badge lookup |
| `GET` | `/api/badge/{platform}/{username}` | Single badge lookup |
| `GET` | `/api/health` | Health check |

Valid platforms: `x`, `bluesky`, `zcashforum`

The web app proxies `/api/register`, `/api/scan`, and `/api/health` through Next.js routes on port **3001**. Chain height for the wizard defaults comes from `/api/chain-height` (Blockchair).

Environment:

```bash
DATABASE_URL=sqlite:badges.db cargo run -p badge-server
```

## CLI Reference

Run via Cargo:

```bash
cargo run -p zcash-verifier -- <command> [flags]
```

Or use the built binary: `./target/debug/verifier`

### register

Scan once and register badges across platforms.

```bash
verifier register --seed "..." --x handle [--bluesky handle] [--zcashforum handle]
```

| Flag | Description |
| ---- | ----------- |
| `--seed` | BIP39 seed phrase (required) |
| `--x` | X / Twitter username |
| `--bluesky` | Bluesky handle |
| `--zcashforum` | Zcash Forum username |
| `--start-height` | Block height to start scanning from |
| `--network` | `main` (default) or `test` |
| `--server-url` | Badge server URL (default: `http://localhost:3000`) |
| `-o, --output-dir` | Output directory for proof JSON (default: `.`) |

### prove

Generate a proof for a single platform.

```bash
verifier prove orchard --seed "..." --platform x --username handle
verifier prove transparent --secret-key <hex> --platform zcashforum --username handle
```

### scan

Check balance without generating a proof.

```bash
verifier scan orchard --seed "..." --start-height 3295000
verifier scan transparent --secret-key <hex>
```

### verify

Verify an existing proof file locally.

```bash
verifier verify --proof zcash_proof.json
```

### submit

Re-submit an existing proof to the badge server.

```bash
verifier submit --proof proof.json --platform x --username handle
```

## Badge Tiers

| Tier | Threshold | Image |
| ---- | --------- | ----- |
| Holder | < 1 ZEC | `badge_holder.png` |
| 10 ZEC | ≥ 10 ZEC | `badge_10zec.png` |
| 100 ZEC | ≥ 100 ZEC | `badge_100zec.png` |
| 1K ZEC | ≥ 1,000 ZEC | `badge_1k_zec.png` |
| 10K ZEC | ≥ 10,000 ZEC | `badge_10k_zec.png` |
| 100K ZEC | ≥ 100,000 ZEC | `badge_100k_zec.png` |
| 1M ZEC | ≥ 1,000,000 ZEC | `badge_1m_zec.png` |
| 10M ZEC | ≥ 10,000,000 ZEC | `badge_10m_zec.png` |

Badge images are bundled in `extension/icons/badges/`.

## Security Model

- **Cryptographic verification** — Proofs are verified against the Zcash blockchain via lightwalletd. You cannot forge a proof without the wallet's private key.
- **Challenge binding** — Each proof is bound to a `platform:username` pair and cannot be reused across identities.
- **Expiry** — Badges expire after 30 days and must be renewed.
- **CLI** — The verifier CLI runs locally; your seed never leaves your machine unless you choose to use the web registration flow.
- **Web app** — Sends your seed to your **local** badge server for scanning and proof generation. Intended for localhost development only.

## Database

SQLite database `badges.db` is created on first server run. Manual setup:

```bash
sqlite3 badges.db < badge-server/migrations/001_init.sql
```

Schema key: `verified_badges(platform, username)` with full proof stored as JSON.

## Chrome Extension

**ZcashBadge** (MV3) features:

- Reads verified badges from the badge server (`/api/badges` batch lookup)
- Injects badge shields next to usernames on X, Bluesky, and Zcash Forum
- Hover tooltip shows tier, handle, and expiry
- Popup: **My Badges**, **Lookup**, **Settings** (server URL only)
- Auto-syncs identities from the web app via `sync.js` on `localhost:3001`
- SPA support via `MutationObserver` + periodic rescan

Configure the badge server URL in **Settings** (default: `http://localhost:3000`).

## Supported Platforms

| Platform | URL |
| -------- | --- |
| X (Twitter) | https://x.com |
| Bluesky | https://bsky.app |
| Zcash Forum | https://forum.zcashcommunity.com |

## Misc

Sample proof JSON: `zcash_prove.json`

## License

MIT
