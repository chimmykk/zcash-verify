#!/bin/bash
set -e

echo "Starting ZcashVerify web app on http://localhost:3001"
echo "Make sure the badge server is running: ./start_server.sh"

cd "$(dirname "$0")/web"
npm run dev
