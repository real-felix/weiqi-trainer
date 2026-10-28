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

One lean screen for both play and review: Go board, unobtrusive move controls, variation navigator, and a persistent engine status bar. Reserve a clear margin between coordinates and outer stones, and keep the board hit targets aligned with the intersections. Show selected/total move counters (for example 6/13), using the active played line for the total rather than counting review branches. Omit generic history/exploration guidance and the idle “Take your time”/“No clock” text. Align the Play turn label with the board topline and leave space between sidebar content and scrollbars. Put the opponent rank/player color and the **Play / Review** switch for ongoing games in the navbar; omit the opponent-mode eyebrow and visible keyboard instruction footer. The original branding is a Home link with consistent pointer, hover and keyboard focus behavior. Keep the board fully within the viewport in both modes; scroll the sidebar independently. Align board navigation and the captures/export footer with the same bottom padding; keep the sidebar footer visible while its contents scroll. The Play sidebar fills the available height, and the variation list follows newly selected/played moves without disturbing an older review selection. No evaluation hints in Play mode.

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

- Analyze played positions asynchronously while the game progresses, with move generation taking priority. Store evaluations so review is quick; queue missing positions on demand.
- Display score estimate and win-rate history, suggested moves, principal variations, and a list of moves sortable/filterable by severity. Clicking a move navigates to it.
- Classify moves using score loss, win-rate loss, player/opponent strength context, and analysis quality. The labels and color semantics are fixed:
  - **AI move — blue**
  - **Good move — green**
  - **Inaccuracy — yellow**
  - **Mistake — orange**
  - **Blunder — red**
- Use accessible text/icons as well as color. Exact classification thresholds are configurable/calibratable heuristics; do not claim they are objectively validated. Reserve **AI move** for a sufficiently analyzed engine-preferred move.
- Preserve raw evaluations, visits, model/configuration identity, and classification version so classifications can be recalculated without rerunning inference.
- Prioritize selected positions and branches for deeper analysis. Avoid starving opponent move generation.

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
