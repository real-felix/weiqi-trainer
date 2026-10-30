# Weiqi Trainer — Product Requirements

**Subtitle:** Local go partner

**Status:** Implementation-ready v1.0  
**Platform:** Local web app, macOS/Apple Silicon first  
**Purpose:** A quiet, offline Go sparring partner with human-like strength and immediate, useful review.

## 1. Product principles

- Training, not competition. The estimated rating is an internal difficulty-selection mechanism, not an achievement or anti-cheating system.
- Three destinations: **Home**, **Game**, and a dedicated **Settings** page. Avoid dashboards, intrusive controls, and separate Play/Review pages.
- All games, variations, ratings, models, and analyses remain on the user's machine. No runtime network dependency or telemetry.
- Non-destructive game history: changing a move preserves the previous continuation.
- Strong AI analyzes; a human-trained AI plays. Never confuse the two.

## 2. Home

Home contains a **New game** action, current **opponent** rank/mode, and two separate cards: **Ongoing games** first (resume), then **Review games** (finished and imported records). Settings has its own page, accessible from the navbar. Games appear as clickable board-preview tiles, with date, opponent rank, color, result/status, and a useful last-played indicator. Use cached, compact SVG images of the active played position, generated on demand; avoid an interactive board or complete game-tree fetch for each tile. Keep ongoing previews current when generation is cheap, and reuse unchanged images across analysis and review-only branch updates. Desktop cards accommodate three board previews per row. No separate player-rank display is required.

The Home introduction is two lines: **Find your next move.** followed by **Play against a human-like opponent. Learn from every game.** Use a 16px base text size, a compact navbar with default-size links, consistent card padding, and Lucide UI icons. Use the green accent consistently for hover/focus feedback, including Settings and board previews, and the two-stone brand image as the favicon. Preserve the original two-stone brand mark, “Weiqi Trainer” wordmark and “Local go partner” subtitle (displayed in uppercase) as the Home button; do not add a separate Home link. Card titles (Human SL rank, Ongoing games, Review games and Local engine) are `h2` elements with one shared style. Do not add decorative introduction artwork, an opponent eyebrow on the New game card, or a promotional footer.

### New game

- **Auto**: choose the human-model rank using an internal continuous player rating. Show the selected **opponent** rank only. If no rating exists, initialize from a simple starting rank choice (default 5k) and allow Auto immediately.
- **Manual**: select a supported human-model rank (actual range determined by the installed model). Do **not** reset or disable the internal rating.
- Both modes contribute to the internal rating on completed games.
- Defaults: 19×19, AGA rules, 7.5 komi, no clock. No rules selector. Alternate player color by default; color selection can be a lightweight option.

## 3. Game workspace

One lean screen for both play and review: Go board, unobtrusive move controls, variation navigator, and a persistent engine status bar. Reserve a clear margin between coordinates and outer stones, and keep the board hit targets aligned with the intersections. Show selected/total move counters (for example 6/13), using the active played line for the total rather than counting review branches. Omit generic history/exploration guidance and the idle “Take your time”/“No clock” text. Leave space between sidebar content and scrollbars. Put the opponent rank, turn indicator and the **Play / Review** switch for ongoing games in the navbar. In both modes, show the current board position’s stone color using `mini-stone`, followed by **Your turn** or **Your opponent** according to the player’s color. In Review, this includes the previous position used to inspect a selected move. Remove the information row above the board and the duplicate Play sidebar turn label; keep selected/total move counters in the controls below. Omit the opponent-mode eyebrow and visible keyboard instruction footer. The original branding is a Home link with consistent pointer, hover and keyboard focus behavior. Keep the board fully within the viewport in both modes; scroll the sidebar independently. Align board navigation and the captures/export footer with the same bottom padding; keep the sidebar footer visible while its contents scroll. The Play sidebar fills the available height, and the variation list follows newly selected/played moves without disturbing an older review selection. No evaluation hints in Play mode. Show the return icon whenever the selected node differs from the active played head, in both modes; the end-of-line control remains separate.

### Play mode (ongoing games)

- User places legal stones, passes, or resigns. Opponent responds using KataGo's human SL model at the game's selected rank.
- Navigating backward is always permitted. Playing a different move creates or selects a variation, makes it the active continuation, and the AI responds from there. Preserve the previous continuation and its analysis.
- The interface follows the active continuation. Do not ask for confirmation when changing a move.
- The AI may resign only under a conservative, configurable policy informed by strong analysis, with safeguards against early or transient evaluations.
- During opponent generation, show a spinner and status; prevent duplicate move submission, but allow non-destructive navigation.
- Persist every accepted move and active-continuation change immediately.

### Review mode (ongoing and finished games)

- Toggle Review explicitly on ongoing games; finished games open in Review and cannot return to Play.
- Navigate any node, inspect engine evaluations, and place alternative moves to create exploratory variations. Such variations do not alter the active played result.
- Explore KataGo candidate lines and see refreshed analysis for the resulting positions.
- Provide a compact, comprehensible branch UI showing main continuation, forks, current node, and an easy way to return to the active line. The UI may start as an indented move list rather than a complex graph.
- Keyboard shortcuts for previous/next move and returning to active continuation.

### Completion and scoring

- Resignation finalizes the game. Two passes initiate an AGA-compatible scoring/stone-status confirmation flow; game finishes only once the result is confirmed. Handle dead-stone disagreement or resumption explicitly rather than silently guessing.
- **Finalization is irreversible:** the game result and rating update cannot be changed. Review branches remain editable and never change that result.
- Only the final active continuation counts toward the game result. Earlier branches are preserved but ignored for rating.
- Rating updates **exactly once** per finalized game. Undoing moves, replaying a position, or changing the active continuation does not disqualify the game.

## 4. Analysis and move review

The review sidebar follows [Review-Sidebar-UX-Requirements.md](Review-Sidebar-UX-Requirements.md), with the user's clarified order: sticky game overview → selected move/correction → Your mistakes → compact variation tree. Unlisted sections are removed from Review, including the captures/export footer.

- Combine score lead and the human player's win probability in one compact chart with separate green/blue scales. Score signs use the human player's perspective; leader labels retain Black/White notation. Clicking or dragging the timeline previews a move and navigates on release; keyboard navigation is supported. Mark the selected move; omit the old move-number button strip.
- Selecting a played move shows the authoritative **previous position**. Mark the actual move with its classification color and number; mark alternative moves A/B/C, with the top alternative blue and others green. Show score loss, win probability before/after and the best suggested correction immediately below the overview. Additional candidates are behind **Other suggestions**.
- **Your mistakes** lists only human moves on the played continuation, initially Mistakes/Blunders. Allow adding Inaccuracies and sorting by move order or severity. Each row shows move number, colored classification badge, points lost and the player's win probability. The list has no separate scroll area.
- Clicking an already played move continues along the existing line. Reuse matching existing children before creating variations; selecting an existing suggestion does not append unused engine moves. For a new alternative, clicking the board or correction saves its suggested line as immutable review nodes and opens its first move. Advance with the existing move controls or place different moves anywhere along it. Show the explored evaluation alongside the original. **Return to played game** restores the original inspected move in one action; existing branches, result and rating are preserved.
- Keep the variation tree in a compact bottom disclosure with selected, played-line and exploration indicators. Do not duplicate return controls.
- Place engine/model identity, visits, provenance, confidence details and cancellation under **Analysis details**. Keep **Analyze deeper** contextual. Omit lengthy classification explanations from the main sidebar.
- Analyze accepted moves asynchronously and store evaluations. Report Pending/incomplete work unobtrusively; prioritize both the selected position and its parent over background work. Recover missing and insufficient-confidence analysis after restart; progressively refine weak comparisons asynchronously with a bounded search budget. Show uncertainty explicitly when the limit is reached instead of leaving idle work marked Pending.
- Classifications remain versioned heuristics derived from normalized score/win-rate loss, rank and confidence. Fixed semantics: **AI move/blue**, **Good move/green**, **Inaccuracy/yellow**, **Mistake/orange**, **Blunder/red**. Preserve raw evaluations and accessible non-color labels.

## 5. Rating and adaptive opponents

- Maintain an internal continuous rating with a transparent, deterministic update algorithm (Elo is sufficient for MVP).
- Convert the rating to the nearest supported human-model rank; use hysteresis to prevent frequent oscillation between adjacent ranks.
- Auto uses that rank; Manual overrides it **for that game only**. All finalized games update rating against the selected opponent rank.
- Human-model rank labels are approximate, not guaranteed equivalent to OGS or real-world Elo. Do not show a numerical player Elo in the primary UI.
- Store the opponent rank and rating change with each finalized game for audit/debugging.

## 6. Engine status and settings

- Persistent, unobtrusive status bar, with human opponent and strong analyzer tracked separately. Display meaningful states: locating executable, starting, loading model, ready, generating move, analyzing (with progress when known), and error.
- Expandable diagnostic details: executable path, model paths, process/backend, recent error, and restart/retry controls.
- Resolve KataGo from the backend environment's PATH (equivalent to `which katago`), then common Homebrew locations, then a user-selected absolute path. A GUI-launched app may not inherit the terminal PATH.
- Validate executable, configuration, model compatibility, and launch mode independently. Explicitly configured paths override discovery and persist locally.
- Do not silently download, install, or replace KataGo/models. Offer clear, user-initiated setup guidance.

## 7. Persistence and portability

- SQLite is the source of truth for games, nodes, active continuation, result, internal rating, analysis, and engine settings.
- Preserve all branches across restarts; survive unexpected process termination without corrupting game history.
- Support SGF import/export including variations, at least by the end of MVP. Engine analysis metadata may remain application-specific.
- Keep the data directory discoverable and backup-friendly.

## 8. Non-goals

Online play, accounts, matchmaking, tournaments, clocks, multiple rule sets, opening databases, joseki encyclopedias, LLM explanations, cloud sync, telemetry, and mobile-specific design.

## 9. Acceptance criteria

1. Configure a Homebrew-installed KataGo through automatic discovery or an explicit path; see useful diagnostics when setup fails.
2. Start a 19×19 AGA game against a human-like rank-5k opponent, entirely offline.
3. Observe engine thinking/loading states without freezing the interface.
4. Navigate backward during play, change a move, and continue against the AI while preserving the old branch.
5. Switch an ongoing game to Review; explore variations without modifying the active game.
6. Complete a game by resignation or confirmed scoring; the result is permanent and rating updates once, even if moves were revised.
7. Review moves with score/win-rate graphs, five exact classification labels/colors, sortable mistakes, and alternative KataGo lines.
8. Reopen the application and recover ongoing games, finished games, branches, analysis, and rating.
9. Start Manual and Auto games; both influence subsequent Auto opponent selection.
10. Export and reimport an SGF with meaningful move variations.

## 10. Milestones

- **M0 — Engine proof:** validate installed KataGo binary, human SL model/profile sampling, strong analysis, process restart, and Apple Silicon performance.
- **M1 — Playable vertical slice:** Home/New Game, board/rules, local persistence, human-like opponent, status bar.
- **M2 — Branching:** history navigation, active continuation, Review toggle, variation UI, durable branches.
- **M3 — Review:** queued strong analysis, charts, candidate lines, severity labels and filtering.
- **M4 — Finish and adapt:** AGA scoring confirmation, finalization, resignation, internal rating and Auto selection.
- **M5 — Release quality:** SGF import/export, error recovery, accessibility, offline install/run documentation, integration tests.

## 11. Implementation assumptions to validate, not new product decisions

- Exact supported human SL profiles and inference protocol depend on the installed KataGo/model version.
- Find and assess an existing Rust Go rules library; otherwise isolate and comprehensively test a rules module, especially AGA scoring/superko.
- Benchmark analysis visits and resource contention before choosing fixed quick/deep budgets.
- Initial severity thresholds and rank-to-rating calibration are heuristics and can be tuned after real games.
