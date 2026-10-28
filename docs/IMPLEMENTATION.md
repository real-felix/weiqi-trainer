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
| 9: Manual/Auto adaptive selection | Both modes exercised in browser suite; deterministic Elo/hysteresis domain tests |
| 10: SGF variations | Parser/export unit round-trip plus browser export/reimport with six preserved nodes |

Final standard gate: **24 Rust tests, 13 frontend tests, 3 browser flows pass**; `cargo fmt --check`, warning-free Clippy, Svelte/TypeScript checks, ESLint and production build pass. The separately gated real-engine adapter test passes, as does the real browser/restart smoke. The dependency audit reported zero known vulnerabilities after updating the test runner.

2026-10-28 recovery update: wake/focus/visibility/online handlers replace stale SSE connections and resynchronize without changing selection or mode. A watchdog detects suspended timers and stuck connection attempts; explicit retries use bounded backoff. Unit tests simulate overnight suspension, stale callbacks and cleanup. The browser regression disconnects, creates missed moves, reconnects without reloading, and verifies preserved Review selection and domain errors.

2026-10-28 endgame update: replaced raw human-policy passing with strong search-based passing while preserving human probabilities for board moves. After a human pass, a confident comparison against an explicitly searched pass recognizes equivalent cleanup. The actual reported endgame passed five consecutive real-engine regressions on a temporary snapshot; no saved moves were changed. Unit tests cover premature passes, insufficient confidence, valuable remaining moves, both score perspectives, and the root/child visit-count difference. Scoring browser flows now use a fixture human policy that never chooses pass, so they verify the safeguard rather than depending on raw policy luck.

The UI has desktop and narrow-screen visual checks, native buttons/forms, board grid keyboard focus, history shortcuts, non-color classifications, and reduced-motion behavior. This is not a claim of a formal accessibility audit.

## Deliberate MVP bounds

- One ruleset: 19×19, AGA situational superko, 7.5 komi, no handicap.
- SGF supports even-game move trees and variations; unsupported setup/rules/komi are rejected explicitly. Unknown annotations are not preserved.
- Ownership confidence controls the automated opponent’s stone-status agreement. Ambiguous cases require further play; no silent result inference.
- Ratings, classification thresholds and model rank equivalences remain documented heuristics. Long-game strength calibration and broad resource tuning require representative games.
- The local executable and models must remain installed. No runtime network dependency or automatic installation.

Serena-enabled contributors can run `serena memories check` from the repository root to sanity-check project-memory references.
