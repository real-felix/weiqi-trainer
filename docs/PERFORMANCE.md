# Navigation performance

Measured on the target Mac on 2026-10-28 with the production Vite bundle, Chromium at 1360×1000, and a read-only snapshot of the reported completed game: 337 nodes and 337 evaluations. No saved moves, results or ratings were changed.

## Results

The same browser benchmark and CPU profiler ran before and after the change. Navigation timings include rendering and two animation frames. Review opening and idle Home timings are medians of three runs. The analysis-refresh measurement schedules a Home click 150ms after an SSE notification, allowing the normal 90ms debounce to start the refresh; it reports the delay beyond that scheduled input time.

| Operation | Before | After |
| --- | ---: | ---: |
| Open completed review | 5,066 ms | 116 ms |
| Return Home while idle | 47 ms | 47 ms |
| Home input during an analysis refresh | 5,142 ms | 19 ms |

These comparisons replay identical API snapshots to isolate browser work. Before the fix, actual local HTTP GET measurements were 12ms for `/api/state`, 2ms for the short ongoing game, and 45ms for the completed review. The five-second bottleneck was frontend computation, rather than model inference or SQLite. A follow-up run allowing real local HTTP GETs measured 116ms median review opening and 50ms median idle Home navigation after the fix; Home input during refresh was delayed by 8ms in that run.

## Cause and fix

The `path` helper enumerated every node on every step while walking parent links. Review rows each walked their entire ancestry to display a move number; sorting by move repeated those walks in the comparator. With deeply reactive game snapshots, those enumerations also went through Svelte proxy traps. The browser profile was dominated by `path` and proxy enumeration. Analysis updates repeated the expensive render, blocking subsequent Home input even though Home itself rendered quickly.

- Parent traversal now follows only the visited ancestors, with a set for cycle protection.
- Review move numbers are indexed once per game snapshot and reused for row display and sorting, including branches.
- API snapshots use `$state.raw`: loading or refreshing replaces the entire object, as it already did. Editable settings retain a separate reactive draft. This avoids unnecessary deep tracking of immutable server data. See [Svelte's raw-state contract](https://svelte.dev/docs/svelte/$state#$state.raw).

## Reproduce

Save read-only responses from `/api/state` and `/api/games/<completed-game-id>` as JSON outside version control, then run:

```sh
npm run build
node tests/benchmarks/navigation.mjs /tmp/state.json /tmp/game-view.json /tmp/navigation.json
```

The benchmark loads the running production SPA at `http://127.0.0.1:3000`, replays API snapshots and suppresses engine commands and live notifications. `WEIQI_BENCH_URL` can select another local server. It writes timing JSON and a Chrome CPU profile beside it. Choose a state snapshot whose first Review game is the corresponding completed game.

To include real local HTTP reads, set `WEIQI_BENCH_LIVE_READS=1`. Only state and selected-game GET requests reach the server; mutation/analysis requests remain intercepted. This mode measures the updated UI with transport/backend latency included, rather than serving snapshots.

The browser suite has a synthetic 400-move review regression with a one-second budget for opening and for Home input during an SSE refresh. It also checks move-order sorting. A unit regression verifies that long parent traversals do not enumerate unrelated nodes and that cycles terminate. Timings are machine-specific; the budget detects the original multi-second freeze without depending on a 116ms target.

## Home board previews

The Home gallery uses cached external SVGs rather than mounting interactive boards or fetching full game views. Release-mode reconstruction and SVG generation/hash for the same 337-node snapshot took 0.607ms median over 50 samples, producing 10,684 bytes. Cache hits avoid rules replay and SQLite; unchanged URLs reuse the browser image cache. Run the read-only benchmark with:

```sh
WEIQI_PREVIEW_GAME_JSON=/tmp/game-view.json cargo test --release --offline preview::tests::benchmark_saved_game_preview -- --ignored --nocapture
```

See [the cache design](adr/002-home-board-previews.md). The navigation benchmark also supplies preview images, and live-read mode permits preview GETs while continuing to intercept all mutations.

With the gallery enabled and real local GETs, the three-run navigation median was 101ms for the same completed review and 48ms for idle Home; Home input during the refresh was delayed by 8ms. Both real saved-game previews decoded successfully, with no browser errors.
