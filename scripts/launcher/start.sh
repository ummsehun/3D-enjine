#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
BINARY="${SCRIPT_DIR}/bin/terminal-miku3d"
if [ ! -f "$BINARY" ]; then
  echo "Error: Binary not found at ${BINARY}"
  exit 1
fi
chmod +x "$BINARY"
mkdir -p "${SCRIPT_DIR}/assets"/{glb,stage,pmx,vmd,camera,music,sync}
export LC_ALL="${LC_ALL:-en_US.UTF-8}"
export LANG="${LANG:-en_US.UTF-8}"
exec "$BINARY" start "$@"
