#!/bin/bash
# Deploy ZcashBadge on Ubuntu using GNU screen + Cloudflare quick tunnels.
#
# Usage:
#   ./deployscript.sh          Build, start screens, expose tunnels, print URLs
#   ./deployscript.sh --stop   Stop all screens, services, and tunnels
#   ./deployscript.sh --status Show running screen sessions and saved URLs
#
# Screen sessions created:
#   zcashbadge-server       badge-server on :3000
#   zcashbadge-web          Next.js app on :3001
#   zcashbadge-tunnel-api   Cloudflare tunnel → badge-server (extension)
#   zcashbadge-tunnel-web   Cloudflare tunnel → web app (registration page)
#
# Attach to a session:  screen -r zcashbadge-server
# Detach from screen:   Ctrl+A then D

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
DEPLOY_DIR="$ROOT/.deploy"
PID_FILE="$DEPLOY_DIR/pids"
URL_FILE="$DEPLOY_DIR/urls.env"
LOG_DIR="$DEPLOY_DIR/logs"

BADGE_PORT=3000
WEB_PORT=3001

SCREEN_SERVER="zcashbadge-server"
SCREEN_WEB="zcashbadge-web"
SCREEN_TUNNEL_API="zcashbadge-tunnel-api"
SCREEN_TUNNEL_WEB="zcashbadge-tunnel-web"

mkdir -p "$DEPLOY_DIR" "$LOG_DIR"

log() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*"; }
err() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; }

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "Missing required command: $1"
    exit 1
  fi
}

screen_exists() {
  screen -list 2>/dev/null | grep -q "\.${1}[[:space:]]"
}

stop_screen() {
  local name=$1
  if screen_exists "$name"; then
    echo "  stopping screen: $name"
    screen -S "$name" -X quit 2>/dev/null || true
  fi
}

kill_port() {
  local port=$1
  local pids
  pids=$(lsof -t -i:"$port" 2>/dev/null || true)
  if [ -n "$pids" ]; then
    kill -9 $pids 2>/dev/null || true
  fi
}

stop_all() {
  log "Stopping ZcashBadge deploy stack..."

  stop_screen "$SCREEN_TUNNEL_WEB"
  stop_screen "$SCREEN_TUNNEL_API"
  stop_screen "$SCREEN_WEB"
  stop_screen "$SCREEN_SERVER"

  if [ -f "$PID_FILE" ]; then
    while read -r pid name; do
      [ -z "${pid:-}" ] && continue
      if kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
      fi
    done < "$PID_FILE"
    rm -f "$PID_FILE"
  fi

  kill_port "$BADGE_PORT"
  kill_port "$WEB_PORT"
  pkill -f "cloudflared tunnel --url http://127.0.0.1:$BADGE_PORT" 2>/dev/null || true
  pkill -f "cloudflared tunnel --url http://127.0.0.1:$WEB_PORT" 2>/dev/null || true

  log "Stopped."
}

start_screen() {
  local name=$1
  local cmd=$2
  stop_screen "$name"
  screen -dmS "$name" bash -lc "$cmd"
  if ! screen_exists "$name"; then
    err "Failed to start screen session: $name"
    exit 1
  fi
  log "Screen started: $name"
}

wait_for_local() {
  local url=$1
  local label=$2
  local logfile=$3
  local i
  for i in $(seq 1 90); do
    if curl -sf "$url" >/dev/null 2>&1; then
      log "$label is up ($url)"
      return 0
    fi
    sleep 1
  done
  err "$label did not become ready at $url"
  tail -30 "$logfile" 2>/dev/null || true
  exit 1
}

wait_for_tunnel_url() {
  local logfile=$1
  local i
  for i in $(seq 1 120); do
    local url
    url=$(grep -oE 'https://[a-zA-Z0-9-]+\.trycloudflare\.com' "$logfile" 2>/dev/null | head -1 || true)
    if [ -n "$url" ]; then
      echo "$url"
      return 0
    fi
    sleep 1
  done
  err "Timed out waiting for Cloudflare tunnel URL. Check $logfile"
  exit 1
}

install_cloudflared_hint() {
  cat <<'EOF'
Install cloudflared on Ubuntu:

  curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
    -o /tmp/cloudflared
  sudo install -m 755 /tmp/cloudflared /usr/local/bin/cloudflared
  cloudflared --version
EOF
}

show_status() {
  echo ""
  echo "Screen sessions:"
  screen -list 2>/dev/null || echo "  (none)"
  echo ""
  if [ -f "$URL_FILE" ]; then
    echo "Saved URLs ($URL_FILE):"
    cat "$URL_FILE"
  else
    echo "No saved URLs yet. Run ./deployscript.sh first."
  fi
}

start_stack() {
  need_cmd curl
  need_cmd lsof
  need_cmd screen
  need_cmd cargo
  need_cmd npm
  need_cmd cloudflared || {
    install_cloudflared_hint
    exit 1
  }

  stop_all
  : > "$PID_FILE"
  : > "$LOG_DIR/badge-server.log"
  : > "$LOG_DIR/web.log"
  : > "$LOG_DIR/tunnel-api.log"
  : > "$LOG_DIR/tunnel-web.log"

  log "Building badge-server (release)..."
  (cd "$ROOT" && cargo build --release -p badge-server)

  log "Installing web dependencies..."
  (cd "$ROOT/web" && npm install)

  log "Building web app (production)..."
  (
    cd "$ROOT/web"
    export BADGE_SERVER_URL="http://127.0.0.1:$BADGE_PORT"
    export NEXT_PUBLIC_API_URL="http://127.0.0.1:$BADGE_PORT"
    npm run build
  )

  log "Starting badge-server in screen ($SCREEN_SERVER)..."
  start_screen "$SCREEN_SERVER" "
    cd '$ROOT' && \
    export RUST_LOG='${RUST_LOG:-badge_server=info}' && \
    export DATABASE_URL='${DATABASE_URL:-sqlite:badges.db}' && \
    exec ./target/release/badge-server 2>&1 | tee -a '$LOG_DIR/badge-server.log'
  "
  wait_for_local "http://127.0.0.1:$BADGE_PORT/api/health" "Badge server" "$LOG_DIR/badge-server.log"

  log "Starting web app in screen ($SCREEN_WEB)..."
  start_screen "$SCREEN_WEB" "
    cd '$ROOT/web' && \
    export PORT='$WEB_PORT' && \
    export BADGE_SERVER_URL='http://127.0.0.1:$BADGE_PORT' && \
    export NEXT_PUBLIC_API_URL='http://127.0.0.1:$BADGE_PORT' && \
    exec npm run start 2>&1 | tee -a '$LOG_DIR/web.log'
  "
  wait_for_local "http://127.0.0.1:$WEB_PORT/api/health" "Web app" "$LOG_DIR/web.log"

  log "Starting Cloudflare tunnel for badge-server in screen ($SCREEN_TUNNEL_API)..."
  start_screen "$SCREEN_TUNNEL_API" "
    exec cloudflared tunnel --no-autoupdate --url 'http://127.0.0.1:$BADGE_PORT' 2>&1 | tee -a '$LOG_DIR/tunnel-api.log'
  "

  log "Starting Cloudflare tunnel for web app in screen ($SCREEN_TUNNEL_WEB)..."
  start_screen "$SCREEN_TUNNEL_WEB" "
    exec cloudflared tunnel --no-autoupdate --url 'http://127.0.0.1:$WEB_PORT' 2>&1 | tee -a '$LOG_DIR/tunnel-web.log'
  "

  API_PUBLIC_URL=$(wait_for_tunnel_url "$LOG_DIR/tunnel-api.log")
  WEB_PUBLIC_URL=$(wait_for_tunnel_url "$LOG_DIR/tunnel-web.log")

  cat > "$URL_FILE" <<EOF
# Generated by deployscript.sh — $(date -u +"%Y-%m-%dT%H:%M:%SZ")
serverurlforextension=$API_PUBLIC_URL
webpage access=$WEB_PUBLIC_URL
BADGE_SERVER_URL=$API_PUBLIC_URL
WEB_APP_URL=$WEB_PUBLIC_URL
EXTENSION_SERVER_URL=$API_PUBLIC_URL
EOF

  cat <<EOF

╔══════════════════════════════════════════════════════════════════╗
║                 ZcashBadge deployment complete                   ║
╠══════════════════════════════════════════════════════════════════╣
║  serverurlforextension:                                          ║
║    $API_PUBLIC_URL
║                                                                  ║
║  webpage access:                                                 ║
║    $WEB_PUBLIC_URL
╠══════════════════════════════════════════════════════════════════╣
║  Extension: Settings → Badge Server URL → paste server URL above   ║
║  Register badges at the webpage access URL                       ║
╠══════════════════════════════════════════════════════════════════╣
║  Running in screen (attach with: screen -r <name>)               ║
║    $SCREEN_SERVER
║    $SCREEN_WEB
║    $SCREEN_TUNNEL_API
║    $SCREEN_TUNNEL_WEB
╠══════════════════════════════════════════════════════════════════╣
║  URLs saved: $URL_FILE
║  Logs:       $LOG_DIR
║  Stop all:   ./deployscript.sh --stop
╚══════════════════════════════════════════════════════════════════╝

EOF

  warn "Quick tunnel URLs change each time you redeploy."
  warn "Detach from screen anytime with Ctrl+A then D."
}

case "${1:-}" in
  --stop|stop)
    stop_all
    ;;
  --status|status)
    show_status
    ;;
  --help|-h)
    sed -n '1,18p' "$0"
    ;;
  *)
    start_stack
    ;;
esac
