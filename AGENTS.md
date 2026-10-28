# Local Go Trainer — Engineering Guidelines

This file governs coding-agent contributions. Read `PRD.md` first. Prefer the smallest implementation that satisfies a documented requirement; do not invent product features.

## Stack

- **Frontend:** Svelte 5 runes, TypeScript `strict`, Vite; static SPA served locally.
- **Backend:** Rust, Axum, Tokio.
- **Storage:** SQLite with versioned migrations.
- **Engine:** external KataGo executable; human SL for moves, strong analysis for review.
- **Transport:** HTTP for commands/queries, SSE for engine and analysis updates. Add WebSockets only if a concrete need emerges.
- No SSR, SvelteKit, mandatory Docker, cloud services, or third-party runtime APIs.

## Architectural boundaries

1. **Domain:** Go rules, game-tree operations, game lifecycle, rating, classification. Pure functions wherever possible.
2. **Persistence:** transactional repositories for games/nodes, active continuation, results, analysis, rating events, settings.
3. **Engine adapters:** process lifecycle, human-policy move sampling, strong analysis, protocol decoding, timeouts and restart. No raw KataGo JSON outside adapters.
4. **HTTP/SSE:** thin handlers with typed request/response contracts and meaningful error types.
5. **Frontend:** view state and user interaction; backend remains authoritative for moves, scores, game completion and rating.

Keep domain types independent of Svelte and HTTP. Avoid speculative generic interfaces, plugins, CQRS/event sourcing, and microservices.

## Game-tree invariants

- Nodes are immutable moves with stable IDs and parent links. Branch creation never destroys an existing node.
- Store a separate **active continuation/head** for ongoing play, distinct from the currently selected UI node.
- Review-created branches never alter the active continuation or a finalized result.
- Playing from history changes the active continuation to the chosen/new branch and resumes the AI from there.
- Once finalized, the played result and rating event are immutable; review nodes may still be added.
- A game is finalized transactionally with a unique rating event (one update per game). Use idempotent endpoints for retries.
- Cache derived board positions if needed, but never allow caches to become the source of truth.

## Go rules

- AGA rules, 19×19, komi 7.5. Test captures, suicide, ko/superko, pass, handicap-free starts, scoring, dead-stone confirmation, and resume-after-disagreement behavior.
- Prefer a maintained, well-tested Rust rules library after assessing its ruleset support. If inadequate, isolate a fully tested rules engine rather than embedding logic in handlers or UI.
- Frontend may preview interactions; server validates every accepted move.

## KataGo integration

- Resolve the executable via PATH lookup, then `/opt/homebrew/bin/katago` and `/usr/local/bin/katago`, then user-selected absolute path. Never assume the Finder/GUI PATH equals a terminal PATH.
- Store the explicit executable, human-model, analysis-model, and configuration paths separately. Never silently download or overwrite them.
- Validate real installed versions/protocol and supported human SL profiles in M0. Do not guess configuration keys or rely on an untested command-line example.
- Human move generation samples from the appropriate rank-conditioned **human policy**. Do not fake human strength by limiting strong-search visits or randomly choosing poor strong-engine moves.
- Keep human generation and strong analysis in separately managed workers/processes. Prioritize opponent moves, queue/deduplicate/cancel analysis, and cap resource usage.
- Handle startup, model loading, protocol errors, crashes, timeouts, restart, and shutdown explicitly. Surface useful status events.
- Record engine/model identity, parameters, visits, and timestamps with evaluations. Prefer deterministic test fixtures and separate real-engine integration tests.
- Use strong analysis for resignation policy, not the human-policy model's raw probability. Conservative thresholds and repeated confirmation are required.

## Analysis and classification

- Store raw score/win-rate estimates and candidate variations independently from display labels.
- Normalize evaluation perspective before comparing positions. A move's loss compares the pre-move best alternative to the actual continuation, not arbitrary raw opposite-to-move win rates.
- Support quick background analysis and prioritized deeper interactive analysis; benchmark visit budgets rather than hardcoding performance assumptions.
- Fixed labels/colors: **AI move/blue; Good move/green; Inaccuracy/yellow; Mistake/orange; Blunder/red**. Include accessible non-color cues.
- Classification uses score loss, win-rate loss, rank context and analysis confidence. Keep thresholds and algorithm versioned/testable.

## Svelte/UI

- Use Svelte 5 runes consistently; do not mix legacy Svelte 4 state patterns.
- No large state-management dependency by default. Use framework-independent TypeScript for tree traversal and presentation transforms.
- Normalize nodes by ID; selected node, active head, and review variations are distinct state values.
- Prefer SVG for the 19×19 board. Make stone placement, variation navigation, keyboard controls, and status messages accessible.
- Follow the GNOME-like minimalism in `PRD.md`: two destinations, restrained visual hierarchy, progressive disclosure, visible engine status, no unnecessary dialogs.
- Never leak review evaluations into Play mode.
- UI remains usable while analysis runs. Handle stale async responses with request/node IDs.

## Storage and API

- SQLite migrations are mandatory; use transactions for move/branch/finalization mutations.
- Persist after every accepted move. Define stable typed API contracts and explicit schema versions when appropriate.
- Expose engine status and analysis progress over SSE, with reconnect and initial-state synchronization.
- Avoid storing only SGF as the live mutable model; support SGF import/export as an interchange format.
- Data directory must be documented and straightforward to back up.

## Testing and quality gates

- Domain logic: unit tests first, including branching, active-head switching, finalization immutability, and one-rating-event invariants.
- Engine adapter: fixtures for protocol decoding and failures; real-engine smoke/integration tests gated by available local models.
- Persistence: migration and restart/recovery tests using temporary databases.
- UI: component tests for mode changes, variation navigation, statuses, and classification colors; at least one end-to-end play/review flow.
- Run `cargo fmt`, `cargo clippy`, `cargo test`, frontend typecheck, lint, and tests before considering a milestone complete.
- Prefer small, reviewable vertical slices. No unrelated refactoring. Document consequential decisions in short ADRs.

## Suggested first tasks

1. Create repository layout and minimal build/test commands.
2. Probe Homebrew KataGo and document version, supported backend, human SL configuration, and analysis response shape.
3. Implement engine adapters with real smoke tests and reproducible fixtures.
4. Implement rules and tree invariants with tests.
5. Deliver a single playable, persisted 19×19 game before building elaborate review UI.

## Open technical validations (not blockers to starting)

- Verify exact KataGo human-model profile/range and the recommended sampling method against installed engine source/docs.
- Select a Rust Go rules implementation with correct AGA behavior.
- Measure performance on the target Mac and tune engine concurrency/analysis budgets.
- Calibrate rating mapping and severity thresholds after collecting representative games.
