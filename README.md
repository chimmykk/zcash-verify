# ZcashVerify

Cryptographic proof-of-balance verification for Zcash. Users prove ownership of a Zcash wallet and register verified balance badges on social platforms (X, Zcash Forum, Bluesky). A Chrome extension displays these badges next to usernames.

## Architecture

```
verifier/         CLI tool — generates cryptographic proofs and submits them
badge-server/     HTTP API — verifies proofs, stores badges in SQLite
extension/        Chrome Extension (MV3) — injects badges on supported platforms
```

The system works in three steps:

1. The CLI scans the Zcash blockchain, generates a signed proof binding your balance to your social identity.
2. The proof is submitted to the badge server, which cryptographically verifies it and stores the result.
3. The Chrome extension queries the server and displays a badge next to your username on supported platforms.

## Prerequisites

- Rust 1.75+ and Cargo
- Google Chrome (for the extension)

## Quick Start

### 1. Build Deployments

```bash
cargo build --release
```

### 2. Manage the Badge Server

We provide several convenient bash scripts in the root directory to manage the backend:

- **`./start_server.sh`**: Starts the `badge-server` backend dynamically on `http://localhost:3000`.
- **`./kill_server.sh`**: Safely finds and forcefully terminates the background badge server.
- **`./clean.sh`**: Stops the server and wipes `badges.db` clean to reset all registrations.

Start your server to begin:

```bash
./start_server.sh
```

### 3. Generate a Zcash Proof

You have two workflows to generate your Zcash ownership proof using our helper scripts:

#### Option A: Extension / UI Flow (Recommended)

Use **`./prove_basic.sh`** to generate a generic offline proof.
This script extracts your seed, scans your Orchard balance, and saves the cryptographic receipt to `zcash_prove.json`. Because the challenge verification is relaxed for generic files, you can then open the **Chrome Extension > Submit Proof**, upload the file, and interactively type your social handle to dynamically link your identity!

#### Option B: Automated CLI Flow

Use **`./prove_social.sh`** to automatically generate the proof _and_ explicitly register it across multiple platforms simultaneously.
Edit the script to include your specific handles (e.g., `--x your_handle --bluesky your_handle`), and the CLI will securely submit it straight to the backend server and save a local backup to `zcashprovewithsocial.json`.

_(Note: Ensure your `--start-height` in the bash scripts is set to a block right before your wallet was funded so the scanner successfully calculates your balance!)_

### 4. Install the Chrome Extension

1. Open Chrome and navigate to `chrome://extensions`
2. Enable "Developer mode" (toggle in the top right)
3. Click "Load unpacked" and select the `extension/` directory
4. The ZcashVerify icon will appear in your toolbar

The extension will automatically dynamically inject badges directly into X (Twitter), Bluesky, and Zcash Forum timelines.

## CLI Reference

### register

The recommended command for most users. Scans once, registers across all platforms.

```bash
verifier register --seed "..." --x handle [--bluesky handle] ...
```

| Flag               | Description                                         |
| ------------------ | --------------------------------------------------- |
| `--seed`           | BIP39 seed phrase (required)                        |
| `--x`              | X / Twitter username                                |
| `--bluesky`        | Bluesky handle                                      |
| `--start-height`   | Block height to start scanning from                 |
| `--network`        | `main` (default) or `test`                          |
| `--server-url`     | Badge server URL (default: `http://localhost:3000`) |
| `-o, --output-dir` | Where to save `zcash_proof.json` (default: `.`)     |

### prove

Generate a proof for a single platform. Supports both transparent and Orchard addresses.

```bash
verifier prove orchard --seed "..." --platform x --username handle
verifier prove transparent --secret-key <hex> --platform zcashforum --username handle
```

### scan

Check your balance without generating a proof.

```bash
verifier scan orchard --seed "..." --start-height 3295000
verifier scan transparent --secret-key <hex>
```

### verify

Verify an existing proof file.

```bash
verifier verify --proof zcash_proof.json
```

### submit

Re-submit an existing proof to the badge server.

```bash
verifier submit --proof proof.json --platform x --username handle
```

## Badge Tiers

| Tier     | Threshold         | Image                |
| -------- | ----------------- | -------------------- |
| Holder   | < 1 ZEC           | `badge_holder.png`   |
| 10 ZEC   | >= 10 ZEC         | `badge_10zec.png`    |
| 100 ZEC  | >= 100 ZEC        | `badge_100zec.png`   |
| 1K ZEC   | >= 1,000 ZEC      | `badge_1k_zec.png`   |
| 10K ZEC  | >= 10,000 ZEC     | `badge_10k_zec.png`  |
| 100K ZEC | >= 100,000 ZEC    | `badge_100k_zec.png` |
| 1M ZEC   | >= 1,000,000 ZEC  | `badge_1m_zec.png`   |
| 10M ZEC  | >= 10,000,000 ZEC | `badge_10m_zec.png`  |

## Security Model

- **Cryptographic verification**: Every proof is verified against the Zcash blockchain. You cannot fake a proof without the wallet's private key.
- **Challenge binding**: Each proof is bound to a specific `platform:username` pair. Proofs cannot be reused across different identities.
- **Rate limiting**: The server limits registrations to 1 per address per hour.
- **Expiry**: Badges expire after 30 days and must be renewed.
- **Your keys stay local**: The CLI generates proofs locally. Your seed phrase is never sent to the server.

## Database

The badge server uses SQLite. The database file `badges.db` is created automatically on first run. To set up manually:

```bash
sqlite3 badges.db < badge-server/migrations/001_init.sql
```

To configure a custom database path:

```bash
DATABASE_URL=sqlite:/path/to/badges.db cargo run -p badge-server
```

## Chrome Extension Features

- Detects usernames on X (Twitter), Bluesky, and Zcash Forum
- Injects badge images next to verified usernames
- Hover tooltip shows badge tier and expiry
- Popup allows uploading proof JSON directly and looking up any user
- Configurable server URL
- Works with dynamically loaded content (SPA support via MutationObserver)

## Supported Platforms

| Platform    | URL                              |
| ----------- | -------------------------------- |
| X (Twitter) | https://x.com                    |
| Bluesky     | https://bsky.app                 |
| Zcash Forum | https://forum.zcashcommunity.com |

## Misc

zcash_prove.json is attached to show the sample basic json for the prove

## License

MIT
