#!/bin/bash
echo "Stopping Zcash Badge Server..."
# Try pkill first
pkill -f "cargo run -p badge-server" || true
pkill -f badge-server || true

# Force kill whatever is running on port 3000
PID=$(lsof -t -i:3000)
if [ ! -z "$PID" ]; then
    echo "Killing process on port 3000 (PID: $PID)..."
    kill -9 $PID
fi

echo "Server stopped."
