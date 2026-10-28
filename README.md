# Weiqi Trainer

*Local go partner.*

A local Svelte 5 + Rust web app for human-like Go sparring and strong-engine review. All play, branches, analysis, ratings and settings stay in SQLite on your machine. The runtime makes no external requests.

Home separates ongoing games from finished/imported games ready for review. Settings is a dedicated page at `/#settings`. The interface uses a 16px text base and locally bundled Lucide icons.

## Install and run

Requires a current Rust toolchain, Node.js 22.12+ (or 24+), and KataGo with a supported local backend. The validated machine uses Homebrew KataGo **1.18.2 / Metal** on Apple Silicon.

```sh
npm ci
cargo fetch --locked
npm run build
./scripts/start.sh
```

Open **http://127.0.0.1:3000**. The first release build may take a minute. Subsequent runs are offline (`cargo run --offline --release`). Development uses `cargo run` and, in another terminal, `npm run dev` at http://127.0.0.1:5173; Vite proxies the local API and SSE connection.

If port 3000 is occupied, set `WEIQI_PORT=43128 ./scripts/start.sh` and open that port. `WEIQI_DATA_DIR=/absolute/path` selects another data folder. `WEIQI_DIST_DIR` selects a different static build directory. Run from the repository root, or use the start script.

## Engine setup

Open **Settings**, enter the four independent paths, and choose **Save & validate**:

- **Executable:** discovered from the backend PATH, then `/opt/homebrew/bin/katago`, then `/usr/local/bin/katago`. An explicitly saved absolute path takes precedence.
- **Human SL model:** `b18c384nbt-humanv0.bin.gz`. The app discovers the conventional macOS path `~/Library/Application Support/KataGo/model/b18c384nbt-humanv0.bin.gz` if present.
- **Strong analysis model:** a regular KataGo model, such as the Homebrew b18 model. It is loaded separately from human SL.
- **Configuration:** the absolute path to this repository’s `config/analysis.cfg`, or your own compatible analysis configuration.

The app does not install, download, or replace binaries, configurations or models. If you need the human model, obtain it yourself from the [official KataGo release](https://github.com/lightvector/KataGo/releases/tag/v1.15.0). Homebrew’s [KataGo package](https://formulae.brew.sh/formula/katago) includes strong models. Model files should remain at stable paths after setup.

Validation launches both workers, queries the strong model and tests all modern profiles **20k through 9d** against the supplied human model. Expand the persistent engine bar for version/backend output, paths, PID and recent errors. Restart/retry is explicit after failure; an interrupted opponent turn is recovered on restart. See [engine validation](docs/ENGINE.md) for measured timings and protocol details.

The adapter forces Black-perspective reporting and caps each worker to one query / two search threads with a bounded neural cache. Quick/deep visit budgets and conservative resignation safeguards are editable in Settings. Custom configuration paths are preserved; runtime overrides are recorded with evaluations.

Live updates reconnect and fetch the latest state after laptop sleep, returning to the tab, or a network reconnection. Failed connections retry with bounded backoff. Catching up preserves the selected move and Play/Review mode; temporary network errors clear after a successful refresh.

## Use

- **Home:** ongoing games and reviews are board tiles. Their SVG previews show the active played position and use browser/server caches; analysis and exploratory branches leave the preview unchanged. Images load on demand and update when active play advances.
- **New game:** Auto starts at your chosen initial difficulty (5k by default), then uses the internal continuous rating and hysteresis. Manual overrides only that game’s opponent. Both completed modes update rating. Colors alternate unless selected explicitly.
- **Game workspace:** opponent rank, your color and Play/Review controls live in the navbar. The board stays within the window while the sidebar scrolls independently. Variations use the available sidebar height and keep the selected move visible as play advances; reviewing an older move preserves that selection. The branding links back to Home.
- **Play:** select an empty intersection or Pass. Moves are server-validated under 19×19 AGA rules, komi 7.5, no handicap or clock. Navigate history freely; playing a different legal human move preserves the old line and changes the active continuation. “Continue play from here” lets the opponent move when the selected position is its turn.
- **Review:** explicitly switch on an ongoing game. Finished games open here. Place alternatives or choose a candidate to add its line, without changing the active continuation or the final result. The variation list distinguishes the active line, forks and selected node. Missing analysis queues in the background; selected positions get priority.
- **Keyboard:** on the board, arrows move focus and Enter/Space plays. Outside the board and form controls, Left/Right navigate history and End returns to the active continuation. Every classification includes text as well as color.
- **Finish:** Resign finalizes immediately. Two passes enter stone-status confirmation. The bot uses strong search to judge passing while keeping ordinary moves human-like. After you pass on a settled board, it checks whether passing is equivalent to further cleanup instead of indefinitely capturing already-dead stones. Mark entire dead groups, check agreement with the strong analyzer, then confirm the permanent result. Disagreement or uncertain status requires comparing the suggested groups or resuming play. The app uses AGA area counting; no final result is inferred silently from a raw estimate.
- **Review labels:** AI move / blue; Good move / green; Inaccuracy / yellow; Mistake / orange; Blunder / red. These are versioned, adjustable heuristics using score loss, win probability loss, rank context and visit confidence, not validated claims about playing strength. Human rank labels are approximate and do not promise OGS-equivalent ratings.
- **SGF:** export from Game; import from Home. Even 19×19 AGA move records with 7.5 komi, passes and variations are supported. Setup stones, handicap, other rules, other komi, collections and oversized records are rejected clearly. Imports are review records and do not update training rating. Application-specific properties preserve the active head and scoring resumptions. Unrecognized annotations are not retained.

## Data and backup

Default macOS location: **`~/Library/Application Support/Weiqi Trainer`**. Other Unix systems use `~/.local/share/weiqi-trainer`. Settings displays the actual absolute location.

Stop the server with Ctrl-C, then copy the entire directory for backup. SQLite uses WAL, foreign keys and versioned transactional migrations. Every accepted move is saved immediately; branches are immutable, while the active head is separate. Finalization saves the played result and one unique rating event in the same transaction. A retry cannot change that result or update rating twice. The debug/audit endpoint `/api/games/{id}/audit` exposes the deterministic Elo K=40 calculation, opponent rank and rating delta. Numerical player rating is kept out of the primary interface.

The service binds IPv4 loopback only and checks browser origins. No accounts, telemetry, cloud services, CDN fonts or external runtime APIs are used. Installation downloads build dependencies; offline runtime requires dependencies and models to have been installed already.

## Verification

```sh
npx playwright install chromium   # once, for browser tests
./scripts/check.sh

# Optional real hardware integration, requires installed models and GPU access
WEIQI_HUMAN_MODEL="/absolute/path/b18c384nbt-humanv0.bin.gz" \
  cargo test real_engine_smoke -- --ignored --nocapture
cargo build --offline
npm run test:real
```

The normal suite uses deterministic protocol fixtures and temporary databases. The real adapter test validates all 29 profiles and restart. The real browser smoke plays against KataGo, adds a review-only branch, and checks persistence after restarting the server. It uses temporary data, port 43128, and writes screenshots under `.local/`. Do not run it while that port is occupied.

For the implementation and acceptance mapping see [milestones](docs/IMPLEMENTATION.md), [API contracts](docs/API.md), and the [architecture decision](docs/adr/001-domain-and-engine.md).

Navigation benchmarks and their reproduction commands are documented in [docs/PERFORMANCE.md](docs/PERFORMANCE.md).
