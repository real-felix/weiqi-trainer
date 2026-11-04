# Weiqi Trainer

*Local go partner.*

A local Svelte 5 + Rust web app for human-like Go sparring and strong-engine review. All play, branches, analysis, rank progression and settings stay in SQLite on your machine. The runtime makes no external requests.

Home separates ongoing games from finished/imported games ready for review. Settings is a dedicated page at `/#settings`. The interface uses a 16px text base and locally bundled Lucide icons.

## Install and run

Requires a current Rust toolchain, Node.js 22.12+ (or 24+), Python 3.9+, and KataGo with a supported local backend. The validated machine uses Homebrew KataGo **1.18.2 / Metal** on Apple Silicon.

On macOS, set up the launcher once:

```sh
npm ci
cargo fetch --locked
python3 scripts/app.py install
python3 scripts/app.py update
open "Weiqi Trainer.app"
```

Afterward, **double-click Weiqi Trainer.app** in this folder, or drag it to the Dock. Leave the app in the project folder; the Dock keeps a shortcut. It starts the local server in the background and opens **http://127.0.0.1:43187** in your default browser. Reopening reuses the same server. No terminal needs to stay open, and nothing is added to login items. The first build may take a minute; normal launches use the built release without recompiling.

For future changes, run `python3 scripts/app.py update`. It builds both frontend and backend into a separate release, makes a consistent SQLite backup under the data folder's `backups/` directory, gracefully stops this checkout's server, and starts the updated release. A failed build leaves the running release untouched. Refresh an already-open browser tab after an update. Games, branches, engine paths and settings remain in the same data directory. This command applies local source changes; it does not pull Git changes, download models or install dependencies.

Other controls: `python3 scripts/app.py open` (start and open the browser), `start` (background only), `status`, and `stop` (including both KataGo workers). Build/start/update operations are serialized. The controller refuses to stop a different program, a server from another checkout, or one using a different data directory. Server output is in `.local/app/server.log`; Finder launcher errors are in `.local/app/launcher.log` and appear in a macOS alert. Built releases live under `.local/app/releases/`.

For foreground terminal use, including other Unix systems, run `npm run build && ./scripts/start.sh` and leave that terminal open. Development uses `cargo run` and, in another terminal, `npm run dev` at http://127.0.0.1:5173; Vite proxies the local API and SSE connection.

If port 43187 is occupied, set `WEIQI_PORT=43128 python3 scripts/app.py open` and use that same variable for subsequent control commands. `WEIQI_DATA_DIR=/absolute/path` selects another data folder. Finder launches use the defaults. The foreground start script also supports these variables, plus `WEIQI_DIST_DIR` for a different static build directory. Run control commands from the repository root.

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

- **Player name:** in Settings → Player, save your name or handle for sharing games with your teacher. This applies to exports of existing games too and does not restart the engines. Downloads use names such as `felix-vs-5k-2026-11-03.sgf`, with the game's recorded start date. Spaces and filename separators become hyphens; accents are preserved. Without a name, filenames start with `player`. The SGF includes your player name, the bot's rank for the correct color, and the game date.
- **Home:** ongoing games and reviews are board tiles. Their SVG previews show the active played position and use browser/server caches; analysis and exploratory branches leave the preview unchanged. Images load on demand and update when active play advances.
- **New game:** Auto starts at your chosen initial rank (5k by default). Winning sets the next Auto opponent one half-step stronger than the game played; losing sets it one half-step weaker, within 20k–9d. For example: regular 5k → harder 5k → regular 4k. The harder game still uses the 5k model, with 0.5 komi: an empty board when you play White, or two White stones at Q16/D4 when you play Black (Black moves first). Manual always starts a regular game at the chosen rank with 7.5 komi and no setup stones; its result also sets the next Auto level relative to the rank played. Colors alternate unless selected explicitly. There is no Elo calculation.
- **Game workspace:** actual opponent rank, game komi, turn indicator and Play/Review controls live in the navbar. Half-rank numbers are hidden. The board stays within the window while the sidebar scrolls independently. Variations use the available sidebar height and keep the selected move visible as play advances; reviewing an older move preserves that selection. The branding links back to Home.
- **Play:** select an empty intersection or Pass. Moves are server-validated under 19×19 AGA rules with the game's saved komi and starting stones; regular games use 7.5 komi. There is no clock. Navigate history freely; playing a different legal human move preserves the old line and changes the active continuation. “Continue play from here” lets the opponent move when the selected position is its turn.
- **Review:** the sticky overview combines score and your color's win probability. **Your mistakes** lists your played Mistakes/Blunders, with optional Inaccuracies and severity/move-order sorting. Selecting a move shows the board before it, with the actual move numbered and corrections A/B/C. Click a correction to open a preserved alternative line, advance with the move controls, or play your own moves. **Return to played game** restores the inspected original move. **Other suggestions**, **Analysis details**, and the compact variation tree progressively disclose more information; both players’ moves are analyzed asynchronously and cached. Review fills in missing analysis for either color, including existing variations; its progress counter covers both players on the played continuation. Review never changes the played continuation, final result or rank level. Older quick analyses may initially remain Pending while stronger comparisons are prepared in the background; the move-review progress count tracks this upgrade.
- **Keyboard:** on the board, arrows move focus and Enter/Space plays. Outside the board and form controls, Left/Right navigate history and End returns to the active continuation. Every classification includes text as well as color.
- **Finish:** Resign finalizes immediately. Two passes enter stone-status confirmation. The bot uses strong search to judge passing while keeping ordinary moves human-like. After you pass on a settled board, it checks whether passing is equivalent to further cleanup instead of indefinitely capturing already-dead stones. Mark entire dead groups, check agreement with the strong analyzer, then confirm the permanent result. Disagreement or uncertain status requires comparing the suggested groups or resuming play. The app uses AGA area counting; no final result is inferred silently from a raw estimate.
- **Review labels:** AI move / blue; Good move / green; Inaccuracy / yellow; Mistake / orange; Blunder / red. These are versioned, adjustable heuristics using score loss, win probability loss, rank context and visit confidence, not validated claims about playing strength. Human rank labels are approximate and do not promise OGS-equivalent ratings.
- **SGF:** export from the Play or Review footer, or the documented SGF download API; import from Home. 19×19 AGA records support 7.5 or 0.5 komi, passes and variations. Half-step starting stones round-trip as root `AW` setup with Black to play. Other setup/handicap formats, rules, komi, collections and oversized records are rejected clearly. Imports are review records and do not update the rank ladder. Application-specific properties preserve the active head and scoring resumptions. Unrecognized annotations are not retained.

## Data and backup

Default macOS location: **`~/Library/Application Support/Weiqi Trainer`**. Other Unix systems use `~/.local/share/weiqi-trainer`. Settings displays the actual absolute location.

Stop the managed app with `python3 scripts/app.py stop` (or use Ctrl-C for the foreground script), then copy the entire directory for backup. SQLite uses WAL, foreign keys and versioned transactional migrations. Every accepted move is saved immediately; branches are immutable, while the active head is separate. Finalization saves the played result and one unique rank event in the same transaction. A retry cannot change that result or advance the ladder twice. The debug/audit endpoint `/api/games/{id}/audit` exposes the before/after level and the level played. Schema v2 seeds the new ladder from the previous Auto opponent rank and retains historical Elo audits unchanged; existing games keep their original 7.5 komi and empty starts. Back up before upgrading; an old executable cannot read the new schema.

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

# Optional launcher lifecycle integration, fixture engines and temporary data
python3 tests/launcher_smoke.py
```

The normal suite uses deterministic protocol fixtures and temporary databases. The real adapter test validates all 29 profiles and restart. The real browser smoke plays against KataGo, adds a review-only branch, and checks persistence after restarting the server. It uses temporary data, port 43128, and writes screenshots under `.local/`. Do not run it while that port is occupied.

For the implementation and acceptance mapping see [milestones](docs/IMPLEMENTATION.md), [API contracts](docs/API.md), and the [architecture decision](docs/adr/001-domain-and-engine.md).

Navigation benchmarks and their reproduction commands are documented in [docs/PERFORMANCE.md](docs/PERFORMANCE.md).
