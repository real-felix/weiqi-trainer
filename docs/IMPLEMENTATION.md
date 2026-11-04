# Implementation plan

The PRD is the scope. Routine engineering decisions are autonomous; engine/model assets are never downloaded or replaced by the application.

| Milestone | Deliverable | Verification | Status |
| --- | --- | --- | --- |
| M0 | Build layout, installed-engine probe, rules assessment | Real protocol fixtures, query_models metadata, all 29 profiles, benchmark, restart | Complete |
| M1 | Home, new game, legal 19×19 play, SQLite, human worker, status | Rules/persistence tests; real 5k browser smoke | Complete |
| M2 | Immutable branches, active head, Review, navigation | Domain/component/browser invariants; durable review branch | Complete |
| M3 | Prioritized analysis, charts, candidates, severity | White-perspective loss, queue cancellation/deduplication, UI filtering | Complete |
| M4 | Scoring agreement/resumption, immutable result, rating | Dead-status agreement tests; browser confirmation/resume; transactional retries | Complete |
| M5 | SGF, restart/recovery, accessibility, documentation | All quality gates; browser workflows; real backend restart and visual QA | Complete |

## Initial machine evidence

2026-10-27: `/opt/homebrew/bin/katago version` reports **v1.18.2, Metal**. Homebrew includes strong b18, b20 and b40 models and sample configurations. The user supplied the human SL model under the conventional macOS KataGo model directory. Both actual models and all modern human ranks were validated; see ENGINE.md. No assets were downloaded or replaced by the application.

## Validation sources

- [KataGo v1.18.2 analysis protocol](https://github.com/lightvector/KataGo/blob/v1.18.2/docs/Analysis_Engine.md)
- [KataGo v1.18.2 config](https://github.com/lightvector/KataGo/blob/v1.18.2/cpp/configs/analysis_example.cfg)
- [Goban rules library](https://github.com/Sagebati/goban)

See ADRs for consequential choices and ENGINE.md for exact probe evidence and remaining hardware validation.

## Acceptance mapping

| PRD criterion | Evidence |
| --- | --- |
| 1: Homebrew/explicit setup and diagnostics | Settings form, query_models checks, fixture failures, real installed-engine startup |
| 2: Offline 5k play | Real browser smoke: legal D4, actual human SL opponent response |
| 3: Responsive thinking/loading states | Separate SSE worker states; navigation remains enabled while thinking |
| 4: Change history without deletion | Browser flow preserves both old and new play branches |
| 5: Review branches preserve active game | Component/domain/browser assertions, including real-engine flow |
| 6: Permanent result, once-only rating | Transaction/retry/restart test; resignation and scoring browser flows |
| 7: Graphs, five labels, severity, candidate lines | Component tests for exact semantics/filtering/provenance, normalized White-loss test, visual QA |
| 8: Restart recovery | SQLite reopen test and actual server stop/restart preserving game, branches and evaluations |
| 9: Manual/Auto adaptive selection | Both modes exercised in browser suite; deterministic half-step progression and migration tests |
| 10: SGF variations | Parser/export unit round-trip plus browser export/reimport with six preserved nodes |

Final standard gate: **46 Rust tests, 21 frontend tests, 10 browser flows pass**; `cargo fmt --check`, warning-free Clippy, Svelte/TypeScript checks, ESLint and production build pass. The separately gated real-engine adapter test passes, as does the real browser/restart smoke. The initial dependency audit reported zero known vulnerabilities after updating the test runner.

2026-10-28 recovery update: wake/focus/visibility/online handlers replace stale SSE connections and resynchronize without changing selection or mode. A watchdog detects suspended timers and stuck connection attempts; explicit retries use bounded backoff. Unit tests simulate overnight suspension, stale callbacks and cleanup. The browser regression disconnects, creates missed moves, reconnects without reloading, and verifies preserved Review selection and domain errors.

2026-10-28 endgame update: replaced raw human-policy passing with strong search-based passing while preserving human probabilities for board moves. After a human pass, a confident comparison against an explicitly searched pass recognizes equivalent cleanup. The actual reported endgame passed five consecutive real-engine regressions on a temporary snapshot; no saved moves were changed. Unit tests cover premature passes, insufficient confidence, valuable remaining moves, both score perspectives, and the root/child visit-count difference. Scoring browser flows now use a fixture human policy that never chooses pass, so they verify the safeguard rather than depending on raw policy luck.

2026-10-28 design update: per the user's revised requirements, Home now has the exact two-line introduction, an opponent/New game card, then separate Ongoing games and Review games cards with one shared padding token. The navbar is 64px with 16px links, the text base is 16px, and UI icons use the official locally bundled `@lucide/svelte` package. Settings is a dedicated hash-route page, overriding the original two-destination constraint; it supports refresh and browser back/forward. Decorative introduction artwork, the New game eyebrow, Home settings disclosure, and promotional footer were removed. Desktop/mobile Home, Settings and Review screenshots were visually checked; the added browser flow verifies card separation and Settings navigation.

Design follow-up: restored the original two-stone branding, wordmark and subtitle as the Home button, removed the separate Home link, and gave all card titles a shared `h2.card-title` style (20px, weight 550) on desktop and narrow screens. Settings' Local engine card uses the same heading style as Home's cards.

2026-10-28 game workspace update: branding is a native Home link with explicit pointer/hover/focus styling. Opponent/player information and Play/Review controls now live in the navbar; the opponent-mode eyebrow and keyboard instruction footer are removed. The workspace fits the dynamic viewport using the board container’s actual remaining dimensions, with independent sidebar scrolling and a full-height Play variation list. Positioning the variation list also contains its screen-reader labels, preventing invisible Review content from extending the document. Selected-move scrolling changes only the variation list, follows active play, and preserves an older Review selection during background updates. An 84-move browser regression covers desktop, short windows, mobile portrait/landscape, sidebar scrolling, diagnostic overlays, branding navigation, and live move following.

2026-10-28 navigation performance update: reproduced the reported 337-position review at a median 5,066ms. Repeated whole-tree enumeration inside parent traversal and per-row ancestry calculation, amplified by deep reactive proxies, blocked rendering and Home input during analysis refreshes. Linear parent traversal, one move-number index per snapshot, and raw immutable API snapshots reduce review opening to 116ms and Home input delay during refresh to 19ms. A 400-move browser performance regression and a long/cyclic traversal unit regression pass. See [benchmark method, results and reproduction](PERFORMANCE.md).

2026-10-28 Home board gallery: game tiles use one lazy cached SVG each, generated from the immutable active head with capture-correct rules replay. An LRU holds 128 images; immutable URLs and content ETags enable browser reuse/304, independent of analysis or exploratory branches. Cold generation of the reported 337-node game measured 0.607ms and 10,684 bytes in release mode, so active previews update on demand after every move. API, cache, rules/image and browser regressions cover freshness and reuse. See [the preview decision](adr/002-home-board-previews.md).

2026-10-28 visual polish: Home is up to 1040px wide with an explicit three-column gallery at desktop widths (900px and up). Shared accent tokens drive hover/focus feedback across buttons, links, fields and previews; Settings uses the accent at rest, and whole game tiles receive the shared accent background on hover, with a small inset around the board and metadata plus an accent outline. The navbar and favicon share one two-stone SVG asset. Board navigation and captures/export use matching 48px footer rows, with contextual hints above navigation and sidebar contents scrolling independently above its pinned footer. Existing browser workflows verify three-column layout at 1360px/1024px, accent feedback on every tile and Settings, favicon delivery, and footer alignment in Play and Review.

2026-10-28 board readability follow-up: increased the SVG board margin from 32 to 48 units, moved coordinates clear of outer stones, and derived the native hit-grid inset from the same geometry. Header/navigation counters show selected move over active-line total, independent of review branches; generic history/review hints and idle encouragement are removed, retaining scoring and engine-state guidance. Play turn labels align with the board topline. Sidebar, variation and move-review scroll areas reserve extra right padding. Browser checks cover coordinate clearance, stone/hit-target alignment, changing move totals during historical review, turn/footer alignment, and desktop/mobile viewport containment.

Branding follow-up: the app is **Weiqi Trainer**, subtitle **Local go partner** (uppercase in the navbar). Home link accessibility labels, the opening message, documentation, and SGF download filenames use the current name. The browser title, package/binary identifiers, SGF application identity and data directory already match. Existing browser workflows verify the title, navbar branding, Home navigation and export filename.

2026-10-29 review sidebar redesign: implements the supplied learning-workflow requirements and both clarified choices. The sticky combined overview uses the human perspective, scrubbing and selection markers; selected moves show the previous authoritative board with colored actual/correction markers. Human-only mistake navigation precedes a compact tree, while engine details and other suggestions are disclosures. Full candidate lines are saved transactionally and can be advanced, forked and restored to the original move without changing result/rating. Both comparison positions are prioritized and analysis progress is exposed. The dedicated browser workflow covers chart drag, one-click board corrections, manual branches, return, unchanged audit/result, disclosures and sticky behavior. See [the review design decision](adr/003-review-learning-workflow.md).

The UI has desktop and narrow-screen visual checks, native buttons/forms, board grid keyboard focus, history shortcuts, non-color classifications, and reduced-motion behavior. This is not a claim of a formal accessibility audit.

## Deliberate MVP bounds

- One ruleset: 19×19, AGA situational superko; regular games use 7.5 komi, half-steps use 0.5 komi with two White setup stones when the human plays Black.
- SGF supports even-game and half-step move trees, root White setup and variations; other setup/rules/komi are rejected explicitly. Unknown annotations are not preserved.
- Ownership confidence controls the automated opponent’s stone-status agreement. Ambiguous cases require further play; no silent result inference.
- Half-step strength, classification thresholds and model rank equivalences remain documented heuristics. Long-game strength calibration and broad resource tuning require representative games.
- The local executable and models must remain installed. No runtime network dependency or automatic installation.

Serena-enabled contributors can run `serena memories check` from the repository root to sanity-check project-memory references.

2026-10-30 review recovery update: reproduce the 36/42 reviewed stall as six idle, completed 257-visit positions whose best candidates had only 27–59 visits. Bounded adaptive background refinement preserves the 64-visit confidence threshold and resumes cached weak comparisons, including after restart. Existing board moves and suggestions now follow saved nodes before invoking branch creation. Regression coverage exercises autonomous multi-step refinement, bounded uncertainty, priority/budget retention, played-line clicks, branch reuse, and result/rating immutability. The updated live review reached 42/42 reviewed with no Pending moves; a read-only browser check advanced an existing played move from 26 to 27 with zero move writes. All four saved games retained their heads, node counts, results and rating audits across the graceful backend update.

2026-10-30 both-player analysis follow-up: apply the same background refinement to human and bot moves, recover missing results throughout existing branches in Review, and reuse persisted results after restart. The review progress counter covers both players; Your mistakes continues to show human moves only. Tests exercise a real opponent reply followed by autonomous two-player analysis, recovery of both colors on archived branches, cache reuse, and progress/list separation.

2026-10-30 Review marker follow-up: unify the category-colored, numbered last-stone marker across the main line, suggested variations and manual review moves. Browser coverage verifies AI, Good, Inaccuracy, Mistake and Blunder colors, move numbers and variation navigation using deterministic presentation evaluations.

2026-10-30 recommendation provenance fix: reproduce move 68’s false AI alternative (H16 was already the best played move; D15 was a low-visit second candidate). Preserve the preferred flag before filtering, share candidate presentation across board/sidebar, and use authoritative cached categories or neutral styling for other candidates. Unit coverage retains the White-perspective reported values; browser coverage checks that excluding an already played best move never promotes a known blunder to AI and that its category agrees when explored. A read-only live browser check on the reported game confirmed D15 is marked Mistake before and after exploration, the original/alternative leads remain W +17.2/W +11.4, and no moves were written.

2026-10-30 candidate ranking follow-up: restore colored categories for unplayed alternatives from backend candidate estimates, mark insufficient-confidence categories Estimated, and show the original engine rank. Confirmed branch grades override estimates. Regression coverage checks Black/White normalization, unchanged confidence thresholds, ranking before filtering, and colored unplayed candidates in the browser.

2026-10-30 rank ladder update: replaced the live Elo/hysteresis profile with direct rank progression, one half-step up for a win and down otherwise, based on the game played. Regular and Manual games use 7.5 komi; half-steps retain the actual human SL rank and use 0.5 komi, with two White setup stones at Q16/D4 when the human plays Black. Game conditions flow through rules, scoring, both engine workers, cached previews and SGF. The navbar shows actual rank and komi without half-rank labels. Schema v2 preserves existing games and historical audits, seeding the ladder from the previous Auto rank. Domain/persistence tests cover boundaries, setup superko/captures, immutable conditions, migration and finalization retries. The ninth browser flow covers two wins, a loss, both colors, Manual regular conditions and SGF setup round-tripping. The installed Metal engine passes both half-step setups; the real play/review/restart smoke passes. See [the rank ladder decision](adr/004-rank-ladder.md).

2026-11-03 local launcher: added a generated macOS app bundle for double-click/Dock launching and a serialized local controller for start/open/status/stop/update. Daily use runs a detached server with no terminal or login service. Updates build isolated matching frontend/backend releases, take a consistent SQLite backup and gracefully replace only this checkout’s service. Five launcher unit tests and the fixture-engine process lifecycle smoke verify failed-build preservation, ownership/data-directory guards, WAL backup integrity, duplicate-free reopening, saved-game/branch/result/audit preservation and clean shutdown. The standard 42 Rust, 21 frontend and nine browser tests pass. See [the launcher decision](adr/005-local-launcher.md).

2026-11-03 teacher-friendly SGF exports: added a persisted Player name/handle card in Settings with a dedicated save endpoint that preserves engine settings and running workers. Export filenames use the current saved name, actual game rank and recorded start date, with a safe ASCII fallback and Unicode browser download support. SGF player names are escaped, the bot rank is written to the correct BR/WR property, and DT records the original date. Existing settings default safely without a schema change. Tests cover persistence across reopening, name validation, special characters, both colors, half-step/kyu/dan metadata, original dates and real browser suggested filenames. All 45 Rust, 21 frontend, five launcher unit tests and ten browser flows pass.

2026-11-04 human-model readiness fix: reproduced an installed, loaded human worker reporting ready with an empty supported-rank list after saving a player name and making a subsequent engine request. The worker cache incorrectly included the entire Settings object, so player metadata triggered a lazy restart that cleared validated profiles. Reuse now depends only on the actual process inputs. A deterministic adapter regression first reproduced the empty list, then verified both worker PIDs and all 29 profiles survive the name change and subsequent move/analysis requests. The browser workflow now waits for the opponent reply, checks the retained human PID/rank list, and starts another game through Home. All 46 Rust, 21 frontend, five launcher unit tests and ten browser flows pass.
