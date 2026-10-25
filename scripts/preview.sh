#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../mockups"
exec python3 -m http.server "${PORT:-4173}" --bind 127.0.0.1
