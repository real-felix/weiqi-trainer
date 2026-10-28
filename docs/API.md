# Local HTTP contract — schema v1

JSON commands and queries under `/api`, with `{ "error": "…" }` on application errors. Domain conflicts return 409, unknown games 404, and invalid browser origin/Host 403. Axum rejects malformed JSON with 4xx. Node IDs are stable UUIDs; a null point is pass; points are row-major 0–360 from the top-left. Color is `B` or `W`. Timestamps are UTC RFC3339. The backend is authoritative.

| Route | Request | Response / behavior |
| --- | --- | --- |
| GET `/state` | — | schema version, game metadata, settings, separate worker statuses, opponent Auto rank, data directory |
| GET `/events` | — | SSE `sync` event immediately, then invalidations for status/data changes; fetch state after every reconnect; keepalives enabled |
| PUT `/settings` | Settings object | Saves paths/budgets/classification/resignation parameters; asynchronously restarts and validates workers |
| POST `/engine/restart` | `{}` | Restarts both workers, revalidates profiles, recovers pending turns and missing analysis |
| POST `/games` | `{mode:"auto"|"manual",rank?,starting_rank?,color?:"B"|"W"}` | `{id}`; requires a validated rank-conditioned human model |
| GET `/games/{id}?node={node}` | Optional selected node | game + normalized node map, selected board, analysis map, recalculated move reviews, thinking flag; selection is never persisted as active head |
| POST `/games/{id}/moves` | `{parent,point:number|null,review:boolean,expected_head?}` | `{node}`; Play requires matching active-head token, human turn and unfinished game; Review changes no active result; same move/parent reuses a node |
| POST `/games/{id}/continue` | `{node,expected_head,resume?:boolean}` | Switches active continuation and generates opponent move if needed; resume resets pass count at that node without deleting passes |
| POST `/games/{id}/retry` | `{}` | Retries a pending opponent turn; per-game generation is deduplicated |
| POST `/games/{id}/analysis` | `{node,deep?:boolean}` | Deduplicated priority queue request; data arrives via SSE invalidation and GET |
| POST `/games/{id}/analysis/cancel` | `{}` | Cancels queued work for game and interrupts the analyzer |
| POST `/games/{id}/resign` | `{head}` | Finalizes loss by resignation; retry returns existing immutable result |
| POST `/games/{id}/score` | `{head,dead:number[],confirm?:boolean}` | Queues deep ownership if missing, otherwise score + agreement/disagreement + suggested whole dead groups; only agreed `confirm:true` finalizes |
| GET `/games/{id}/audit` | — | Rating before/after, opponent rank, result, timestamp, algorithm, or null for unfinished/imported game |
| GET `/games/{id}/sgf` | — | SGF attachment including variations and active-head/resume metadata |
| POST `/games/import` | `{sgf:string}` | `{id}` for a review-only imported record, no rating event |

Finalization is transactionally idempotent by game ID. In-flight engine replies also carry an expected active head; stale responses cannot overwrite a changed or finalized continuation. The UI keeps its selected node and Play/Review state separately and rejects stale navigation responses. Evaluations are displayed only in Review (scoring has its separate explicit agreement flow).

Quick analysis defaults to 64 visits, deep to 256, based on the installed-engine probe. Model revisions are recorded with path, byte length and modification time; configurations use SHA-256. Evaluations retain the engine version/backend output, exact query, enforced worker overrides, visits, timestamp, candidate variations, ownership and classification version at inference. Display classifications are recalculated from raw stored evaluations using the current calibration version. The model identity is a local file identity rather than a content digest; retaining the model files is recommended for reproducibility.
