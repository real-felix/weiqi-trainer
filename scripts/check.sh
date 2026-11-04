#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline
python3 -m unittest discover -s tests -p 'test_launcher.py'
npm run check
npm run lint
npm test
npm run build
npm run test:e2e
