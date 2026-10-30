# Review Sidebar — UX Requirements

## Objective

Redesign the review sidebar around the player's learning workflow, not KataGo's analysis capabilities.

The primary purpose of review is to identify mistakes, understand better moves, and explore alternative continuations.

The sidebar must make it easy to answer:

1. Where did I make mistakes?
2. What should I have played instead?
3. What would have happened if I'd played differently?

Do not organize the interface as a collection of independent engine features.

## 1. Game overview

Keep the existing score-estimate and win-probability graphs, but make them more compact.

Prefer combining both into one chart with two distinguishable series, with each metric remaining readable and correctly scaled.

Requirements:

- Design on the left side:
  - Header: eg W +0.9
  - Subtitle: Estimated lead
  - Scale: eg -10 +10
  - Color: green
- Design on the right side:
  - Header: eg 38%
  - Subtitle: `player_s_color` win probability
  - Scale: 0% 100%
  - Color: blue
- Show the evolution of both metrics throughout the game.
- Clicking a chart position navigates to that move, click+hold allows a to find the right move number more easily.
- No need to keep `chart-navigation`.
- Visually identify the currently selected move.
- Maintain a consistent player/color perspective.
- This section is sticky and is always on top.

This section is an overview, not the primary review interaction.

## 2. Mistake-first review

Immediately below the overview, display **Your mistakes**.

Default behavior:

- Show only the human player's moves.
- Displays Blunders and Mistakes by default.
- Sort by move order or severity depending on the dropdown choice.
- Exclude Inaccuracies, Good moves and AI moves by default.
- Allow filtering to display Inaccuracies if desired.

Every mistake entry must show:

- Move number.
- Classification in a badge, with its associated color.
- Estimated points lost.
- The winrate.

Clicking a mistake navigates to its position and selects it for detailed inspection.

Do not make this section scrollable.White to move

## 3. Selected move and suggested correction

When a move is selected, show a focused comparison from the previous move perspective:

- Move actually played.
- Best alternatives according to KataGo.
- Is displayed on top, below the diagrams:
  - Estimated score loss.
  - Win probability before and after the move, when available.
- The player's move and alternative on the board are displayed with the classification color: eg the player's blunder is a red circle, the top AI alternative is a blue one, the other are green.

The board should visually distinguish the actual move from the suggested alternative:

- Actual move: marked using the existing last-move indicator: colored dark background, white text with move number.
- Suggested move: a lighter background, with A, B, C, etc.

## 4. Interactive exploration

Exploration is a central feature, not an advanced tool.

When exploring a suggested move:

- Show the alternative position on the board.
- Display KataGo's suggested continuation.
- Allow the player to advance through the suggested line.
- Allow the player to play different moves at any point.
- Preserve all created variations.
- Reanalyze newly explored positions asynchronously.
- Allow returning to the original game in one action.

The sidebar should display the evaluation of the explored position, making it easy to compare the alternative with the original continuation.

Exploration must never modify the completed game's result or rating.

## 5. Candidate lines

Do not display a permanent list of five candidate lines by default.

The selected move's best alternative is already visible in the correction panel.

Additional candidate moves should be accessible through a secondary **Other suggestions** disclosure, ordered by engine evaluation.

Candidate lines remain selectable and explorable.

## 6. Variation tree

The variation tree is necessary for navigation but should not dominate the review sidebar.

- Keep it compact, and at the bottom.
- Clearly distinguish the original played continuation from exploratory branches.
- Preserve the selected-position indicator.
- Allow switching between branches without losing navigation context.
- Avoid duplicating move navigation controls unnecessarily.

A more elaborate graphical variation tree can be considered later.

## 7. Engine diagnostics

Move the following out of the primary review hierarchy:

- Analysis provenance.
- Engine and model identifiers.
- Visit counts.
- Technical confidence details.
- Analysis cancellation controls.

These should remain available through an expandable **Analysis details** section or the persistent engine status bar.

Retain **Analyze deeper** as a contextual action for the selected position.

Do not display lengthy explanations of the classification algorithm in the main sidebar.

## 8. Analysis progress

Analysis runs asynchronously and persists its results.

- Analyze the user's move right after it's played, asynchronously, and store the analysis alonside the move when it's ready. That way, the data is ready as soon as the game completes.
- Display incomplete evaluations as Pending.
- Provide unobtrusive analysis progress feedback.
- Prioritize selected positions and exploratory continuations over background analysis.

## 9. Visual and interaction principles

- The board remains the primary visual element.
- The mistake list is the primary review navigation element.
- Use progressive disclosure for advanced engine information.
- Avoid redundant labels, explanations, and technical metadata.
- Do not require multiple clicks to find the best alternative to a mistake.
- Keep the interface consistent with the application's restrained, high-contrast light theme.
- Prefer concise actions and visual comparisons over explanatory prose.

## Acceptance criteria

The redesign is complete when the player can:

1. Open a completed game and immediately see their most significant mistakes.
2. Click a mistake and see the move played and the recommended alternatives.
3. Start exploring the better move with one click on the position.
4. Follow the engine's continuation or play their own alternatives.
5. Compare the resulting evaluation with the original position.
6. Return to the original game without losing exploratory work.
7. Access advanced analysis details without those details cluttering the default interface.
