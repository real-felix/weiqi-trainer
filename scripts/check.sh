#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline
npm run check
npm run lint
npm test
npm run build
npm run test:e2e
