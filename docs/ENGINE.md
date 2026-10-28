# Installed-engine validation

Validated **2026-10-27** on this Mac. The executable reports **KataGo v1.18.2, Metal**; startup logs identify **Apple M2 Max / MPSGraph GPU-only mode**. These measurements are local observations, not cross-machine performance guarantees.

## Assets and launch

- Executable: `/opt/homebrew/bin/katago`.
- Strong model: `/opt/homebrew/share/katago/kata1-b18c384nbt-s9996604416-d4316597426.bin.gz`.
- Human model supplied by the user: `~/Library/Application Support/KataGo/model/b18c384nbt-humanv0.bin.gz` (about 95 MB).
- Configuration: `config/analysis.cfg`, bounded to one analysis query and two search threads per separately managed process. Two workers can be resident, but opponent generation preempts background search. Each neural cache is capped at 2^18 entries; max batch size is 4.

The actual installed CLI help confirmed `analysis`, `-config`, `-model`, `-human-model`, `-override-config` and `-quit-without-waiting`. Both workers use newline-delimited JSON. Only the human worker receives `-human-model`. Every fresh process is checked through `query_models`: the primary model must report `usesHumanSLProfile=false`; the human worker must additionally report a model with `usesHumanSLProfile=true`. This rejects an ordinary strong model mistakenly supplied as the human model.

Human queries set `humanSLProfile=rank_5k` (or the selected profile) and `ignorePreRootHistory=false` inside `overrideSettings`, with `includePolicy=true`. Profiles are probed with one visit. Actual opponent turns use at least 64 strong-model visits to judge passing, while board moves still sample directly proportional to positive legal **humanPolicy** entries. The last of the 362 entries is pass; its raw probability is ignored in favor of strong passing judgment. Human search-blending exploration settings are explicitly zeroed, so the passing judgment uses the regular strong model.

After a human pass, at least 256 visits check ownership and score uncertainty. On a confidently settled board, an additional forced-pass search tests whether cleanup is effectively equivalent to passing. The versioned reasoning and conservative limits are in ADR 001. This was verified against the user's actual 326-move endgame, where both searches predicted the same result within approximately 0.06 points; repeated real adapter calls passed rather than continuing dead-stone cleanup. The saved game was not modified by those tests. `tests/fixtures/endgame-pass.json` retains only anonymous evaluation fields for the regression.

The real adapter integration test successfully queried **every profile from rank_20k through rank_9d**, sampled a legal move, obtained a strong evaluation, restarted both workers and queried again. Representative recorded responses live in `tests/fixtures/` and include the actual root/candidate response fields. The runtime forces `reportAnalysisWinratesAs=BLACK`; score/win probability are normalized before move loss calculation.

## Observed latency

| Request | Elapsed |
| --- | ---: |
| Strong cold startup + 16 visits | 3.221 s |
| Strong warm 64 visits | 0.360 s |
| Strong warm 256 visits | 1.362 s |
| Human cold startup (strong + human networks) + 1 visit | 3.096 s |
| Human warm rank_5k | 0.016 s |
| Human warm rank_1d | 0.010 s |
| Human warm rank_9d | 0.008 s |

These policy-only probe timings support initial **64 quick / 256 deep** budgets, which remain editable. Actual opponent turns now include the 64-visit passing search; after a human pass a settled-position comparison may require two 256-visit queries. The real-browser smoke also ran both resident workers, generated an opponent response, added a review branch, and recovered the game and stored evaluations across a backend restart. No broad contention or long-game benchmark is claimed; tune budgets on the user’s actual hardware and workload.

## Failure and lifecycle checks

Fixture tests cover missing paths, malformed JSON, protocol errors, crashes, timeouts, cancellation during startup, and rejection of a non-rank-conditioned human model. Status includes executable/backend output, PID, visits, recent stderr and useful errors. Requests are identified, stale active-head replies are discarded, analysis is queued/deduplicated, and cancellation terminates the analyzer process before a clean lazy restart. Worker restart cancels pending work, and Ctrl-C closes SSE streams and kills/reaps both processes. Recovery revalidates models and resumes interrupted opponent turns.

The restricted coding sandbox cannot create a Metal device. Hardware tests were run with host GPU access. Ordinary fixture/unit tests do not require GPU access.

## Sources

- [Version-pinned analysis protocol and human SL guide](https://github.com/lightvector/KataGo/blob/v1.18.2/docs/Analysis_Engine.md)
- [Version-pinned reference configuration](https://github.com/lightvector/KataGo/blob/v1.18.2/cpp/configs/analysis_example.cfg)

Severity and rank/rating calibration remain transparent MVP heuristics, as specified in the PRD. Broader playing-strength validation requires representative completed games and is not implied by successful protocol validation.
