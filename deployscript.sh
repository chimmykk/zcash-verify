#!/bin/bash
# Deploy ZcashBadge using GNU screen + Cloudflare quick tunnels.
# Tested on Ubuntu Linux (primary) and macOS (local dev).
#
# Usage:
#   ./deployscript.sh          Build, start screens, expose tunnels, print URLs
#   ./deployscript.sh --stop   Stop all screens, services, and tunnels
#   ./deployscript.sh --status Show running screen sessions and saved URLs
#
# Screen sessions created (one instance each):
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
LOCK_DIR="$DEPLOY_DIR/deploy.lock"
LOCK_FILE="$DEPLOY_DIR/deploy.lockfile"
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

wait_screen_gone() {
  local name=$1
  local i
  for i in $(seq 1 15); do
    screen_exists "$name" || return 0
    sleep 1
  done
  warn "Screen session $name did not exit cleanly; forcing quit"
  screen -S "$name" -X quit 2>/dev/null || true
  sleep 1
}

stop_screen() {
  local name=$1
  if screen_exists "$name"; then
    echo "  stopping screen: $name"
    screen -S "$name" -X quit 2>/dev/null || true
    wait_screen_gone "$name"
  fi
}

# Port helpers work on Ubuntu (ss/lsof) and macOS (lsof).
port_pids() {
  local port=$1
  local pids

  pids=$(lsof -t -iTCP:"$port" -sTCP:LISTEN 2>/dev/null || true)
  if [ -n "$pids" ]; then
    echo "$pids"
    return 0
  fi

  if command -v ss >/dev/null 2>&1; then
    pids=$(ss -ltnp "sport = :$port" 2>/dev/null \
      | grep -oE 'pid=[0-9]+' | cut -d= -f2 | sort -u || true)
    if [ -n "$pids" ]; then
      echo "$pids"
      return 0
    fi
  fi

  lsof -t -i:"$port" 2>/dev/null || true
}

port_in_use() {
  [ -n "$(port_pids "$1")" ]
}

port_listeners() {
  local port=$1
  if lsof -iTCP:"$port" -sTCP:LISTEN 2>/dev/null | tail -n +2 | grep -q .; then
    lsof -iTCP:"$port" -sTCP:LISTEN 2>/dev/null | tail -n +2
    return 0
  fi
  if command -v ss >/dev/null 2>&1; then
    ss -ltnp "sport = :$port" 2>/dev/null || true
    return 0
  fi
  lsof -i:"$port" 2>/dev/null | tail -n +2 || true
}

wait_port_free() {
  local port=$1
  local label=${2:-"port $port"}
  local i
  for i in $(seq 1 30); do
    port_in_use "$port" || return 0
    sleep 1
  done
  err "$label (port $port) is still in use"
  port_listeners "$port" || true
  return 1
}

kill_port() {
  local port=$1
  local pid
  local pids
  pids=$(port_pids "$port")
  [ -z "$pids" ] && return 0

  echo "  freeing port $port (PIDs: $(echo "$pids" | tr '\n' ' '))"
  while read -r pid; do
    [ -z "$pid" ] && continue
    kill "$pid" 2>/dev/null || true
  done <<< "$pids"

  sleep 2

  pids=$(port_pids "$port")
  [ -z "$pids" ] && return 0

  while read -r pid; do
    [ -z "$pid" ] && continue
    kill -9 "$pid" 2>/dev/null || true
  done <<< "$pids"

  wait_port_free "$port" "port $port"
}

kill_stray_processes() {
  # Stop dev/manual runs that are not managed by our screen sessions.
  pkill -f "target/release/badge-server" 2>/dev/null || true
  pkill -f "target/debug/badge-server" 2>/dev/null || true
  pkill -f "cargo run -p badge-server" 2>/dev/null || true
  pkill -f "next dev -p $WEB_PORT" 2>/dev/null || true
  pkill -f "next start" 2>/dev/null || true
  pkill -f "cloudflared tunnel --url http://127.0.0.1:$BADGE_PORT" 2>/dev/null || true
  pkill -f "cloudflared tunnel --url http://127.0.0.1:$WEB_PORT" 2>/dev/null || true
}

stop_all() {
  log "Stopping ZcashBadge deploy stack..."

  stop_screen "$SCREEN_TUNNEL_WEB"
  stop_screen "$SCREEN_TUNNEL_API"
  stop_screen "$SCREEN_WEB"
  stop_screen "$SCREEN_SERVER"

  kill_stray_processes
  sleep 1

  kill_port "$BADGE_PORT"
  kill_port "$WEB_PORT"

  log "Stopped."
}

start_screen() {
  local name=$1
  local cmd=$2
  stop_screen "$name"
  screen -dmS "$name" bash -lc "$cmd"
  sleep 1
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
    url=$(grep -oE 'https://[a-zA-Z0-9-]+\.trycloudflare\.com' "$logfile" 2>/dev/null | tail -1 || true)
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
Install dependencies on Ubuntu:

  sudo apt update
  sudo apt install -y curl lsof screen build-essential

Install cloudflared:

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
  echo "Port listeners:"
  for port in "$BADGE_PORT" "$WEB_PORT"; do
    if port_in_use "$port"; then
      echo "  :$port in use:"
      port_listeners "$port" | sed 's/^/    /'
    else
      echo "  :$port free"
    fi
  done
  echo ""
  if [ -f "$URL_FILE" ]; then
    echo "Saved URLs ($URL_FILE):"
    cat "$URL_FILE"
  else
    echo "No saved URLs yet. Run ./deployscript.sh first."
  fi
}

USE_FLOCK_LOCK=0

acquire_deploy_lock() {
  # Ubuntu: flock (util-linux). macOS: mkdir + PID file fallback.
  if command -v flock >/dev/null 2>&1; then
    exec 9>"$LOCK_FILE"
    if ! flock -n 9; then
      err "Another deploy is already running"
      err "Wait for it to finish or run: ./deployscript.sh --stop"
      exit 1
    fi
    USE_FLOCK_LOCK=1
    return 0
  fi

  if [ -d "$LOCK_DIR" ]; then
    local old_pid
    old_pid=$(cat "$LOCK_DIR/pid" 2>/dev/null || true)
    if [ -n "$old_pid" ] && kill -0 "$old_pid" 2>/dev/null; then
      err "Another deploy is already running (PID $old_pid)"
      err "Wait for it to finish or run: ./deployscript.sh --stop"
      exit 1
    fi
    warn "Removing stale deploy lock"
    rm -rf "$LOCK_DIR"
  fi

  if ! mkdir "$LOCK_DIR" 2>/dev/null; then
    err "Another deploy is already running (lock: $LOCK_DIR)"
    err "Wait for it to finish or run: ./deployscript.sh --stop"
    exit 1
  fi
  echo "$$" > "$LOCK_DIR/pid"
}

release_deploy_lock() {
  [ "$USE_FLOCK_LOCK" = 1 ] && return 0
  rm -rf "$LOCK_DIR"
}

with_deploy_lock() {
  acquire_deploy_lock
  trap release_deploy_lock EXIT
  "$@"
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

  log "Starting badge-server (1/4) in screen ($SCREEN_SERVER)..."
  wait_port_free "$BADGE_PORT" "badge-server port"
  start_screen "$SCREEN_SERVER" "
    cd '$ROOT' && \
    export RUST_LOG='${RUST_LOG:-badge_server=info}' && \
    export DATABASE_URL='${DATABASE_URL:-sqlite:badges.db}' && \
    exec ./target/release/badge-server 2>&1 | tee -a '$LOG_DIR/badge-server.log'
  "
  wait_for_local "http://127.0.0.1:$BADGE_PORT/api/health" "Badge server" "$LOG_DIR/badge-server.log"

  log "Starting web app (2/4) in screen ($SCREEN_WEB)..."
  wait_port_free "$WEB_PORT" "web app port"
  start_screen "$SCREEN_WEB" "
    cd '$ROOT/web' && \
    export PORT='$WEB_PORT' && \
    export BADGE_SERVER_URL='http://127.0.0.1:$BADGE_PORT' && \
    export NEXT_PUBLIC_API_URL='http://127.0.0.1:$BADGE_PORT' && \
    exec npm run start 2>&1 | tee -a '$LOG_DIR/web.log'
  "
  wait_for_local "http://127.0.0.1:$WEB_PORT/api/health" "Web app" "$LOG_DIR/web.log"

  log "Starting Cloudflare tunnel for badge-server (3/4) in screen ($SCREEN_TUNNEL_API)..."
  start_screen "$SCREEN_TUNNEL_API" "
    exec cloudflared tunnel --no-autoupdate --url 'http://127.0.0.1:$BADGE_PORT' 2>&1 | tee -a '$LOG_DIR/tunnel-api.log'
  "
  API_PUBLIC_URL=$(wait_for_tunnel_url "$LOG_DIR/tunnel-api.log")
  log "API tunnel ready: $API_PUBLIC_URL"

  log "Starting Cloudflare tunnel for web app (4/4) in screen ($SCREEN_TUNNEL_WEB)..."
  start_screen "$SCREEN_TUNNEL_WEB" "
    exec cloudflared tunnel --no-autoupdate --url 'http://127.0.0.1:$WEB_PORT' 2>&1 | tee -a '$LOG_DIR/tunnel-web.log'
  "
  WEB_PUBLIC_URL=$(wait_for_tunnel_url "$LOG_DIR/tunnel-web.log")
  log "Web tunnel ready: $WEB_PUBLIC_URL"

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
    release_deploy_lock 2>/dev/null || true
    ;;
  --status|status)
    show_status
    ;;
  --help|-h)
    sed -n '1,18p' "$0"
    ;;
  *)
    with_deploy_lock start_stack
    ;;
esac
