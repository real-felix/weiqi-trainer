#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ ! -f dist/index.html ]; then
  printf '%s\n' 'Build the frontend first: npm ci && npm run build'
  exit 1
fi
exec cargo run --offline --release
