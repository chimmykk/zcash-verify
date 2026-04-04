#!/bin/bash
echo "Cleaning up Zcash Badge Server database..."
./kill_server.sh
rm -f badge-server/badges.db badge-server/badges.db-wal badge-server/badges.db-shm \
      badges.db badges.db-wal badges.db-shm
echo "Database cleaned."
