#!/bin/bash
echo "Starting Zcash Badge Server..."
export RUST_LOG=badge_server=info
cargo run -p badge-server
