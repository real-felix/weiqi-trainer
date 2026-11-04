# 004 — Direct ranks with half-step game conditions

2026-10-30. Accepted per the user's revised difficulty requirements.

Replace the continuous Elo profile and hysteresis with an integer half-step ladder. A rank coordinate is -20 for 20k, -1 for 1k, 0 for 1d, and 8 for 9d. The level is twice that coordinate plus a phase flag. Use Euclidean division so negative half-steps retain the weaker actual model profile: -9 means harder 5k, not regular 4k. Clamp progression to regular 20k through regular 9d.

Every regular game uses 7.5 komi and an empty board. Every half-step uses the same actual human SL rank with 0.5 komi. Human White starts empty; human Black faces two White setup stones at Q16 and D4, with Black to play. These are immutable initial conditions, not move nodes, so root positions, superko, captures, move numbers, previews and branches remain consistent. The user clarified that Manual always starts a regular game. A finalized result sets Auto one half-step above or below the level actually played; the profile before finalization is recorded separately for audit. Imports never progress the ladder.

Komi and initial stones belong to the authoritative game and flow into both strong and human-policy queries. KataGo receives `initialStones`, `initialPlayer: "B"` and the game's komi. Set `whiteHandicapBonus: "0"` for the White setup so no additional handicap compensation changes the requested conditions. This is verified against the installed 1.18.2 Metal engine for both player colors. Protocol references: [analysis API](https://github.com/lightvector/KataGo/blob/v1.18.2/docs/Analysis_Engine.md) and [handicap compensation values](https://github.com/lightvector/KataGo/blob/v1.18.2/docs/GTP_Extensions.md).

SQLite migration 002 seeds the ladder from the previous Auto rank, removes the live Elo profile, and adds unique per-game rank events in the existing finalization transaction. Historic rating events remain unchanged. Legacy game JSON defaults to regular conditions, preserving saved play and cached analysis. The API keeps historical audit responses; new audits identify `rank-half-step-v1`. Back up before migration because the previous executable rejects schema v2.

The navbar shows only the actual model rank and komi, never a half-rank number. SGF exports komi and `PL[B]AW[...]` root setup; import supports these conditions and existing even-game trees. No general handicap editor or rules selector is added. Half-step strength is a training convention, not a calibrated claim of an external rating equivalent.
