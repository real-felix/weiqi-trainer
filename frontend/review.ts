import {
  colors,
  labels,
  path,
  type Color,
  type Evaluation,
  type GameView,
  type Candidate,
} from './lib';
export const playerWinrate = (
  evaluation: Pick<Evaluation, 'winrate_black'> | undefined,
  color: Color,
) =>
  evaluation ? (color === 'B' ? evaluation.winrate_black : 1 - evaluation.winrate_black) : null;
export const playerScore = (evaluation: Pick<Evaluation, 'score_black'>, color: Color) =>
  evaluation.score_black * (color === 'B' ? 1 : -1);
export const lead = (evaluation: Pick<Evaluation, 'score_black'> | undefined) =>
  evaluation
    ? `${evaluation.score_black >= 0 ? 'B' : 'W'} +${Math.abs(evaluation.score_black).toFixed(1)}`
    : 'Pending';
export const percent = (value: number | null) =>
  value === null ? 'Pending' : `${(value * 100).toFixed(1)}%`;
export const suggestionColor = (index: number) =>
  index === 0 ? colors['AI move'] : colors['Good move'];
export function orderedCandidates(candidates: Candidate[], turn: Color) {
  const sign = turn === 'B' ? 1 : -1;
  return [...candidates].sort(
    (a, b) => a.order - b.order || (b.score_black - a.score_black) * sign || b.visits - a.visits,
  );
}
export function correction(view: GameView, id = view.selected) {
  const node = view.game.nodes[id];
  const before = node?.parent ? view.evaluations[node.parent] : undefined;
  const after = view.evaluations[id];
  const review = view.reviews.find((review) => review.node === id);
  const candidates = orderedCandidates(
    (node?.parent ? before : after)?.candidates ?? [],
    node?.color ?? view.position.turn,
  );
  const actual = after ?? before?.candidates.find((candidate) => candidate.point === node?.point);
  return {
    node,
    before,
    after,
    actual,
    review,
    bestPlayed: !!node?.color && candidates[0]?.point === node.point,
    alternatives: candidates.filter((candidate) => !node?.color || candidate.point !== node.point),
  };
}
export function mistakeRows(view: GameView, inaccuracies: boolean, sort: 'move' | 'severity') {
  const byNode = new Map(view.reviews.map((review) => [review.node, review]));
  const result = path(view.game.nodes, view.game.active_head).flatMap((node, move) => {
    const review = byNode.get(node.id);
    if (
      node.color !== view.game.human ||
      !review?.label ||
      !['Blunder', 'Mistake', ...(inaccuracies ? ['Inaccuracy'] : [])].includes(review.label)
    )
      return [];
    const actual =
      view.evaluations[node.id] ??
      (node.parent
        ? view.evaluations[node.parent]?.candidates.find(
            (candidate) => candidate.point === node.point,
          )
        : undefined);
    return [{ node, move, review, winrate: playerWinrate(actual, view.game.human) }];
  });
  return result.sort((a, b) =>
    sort === 'move'
      ? a.move - b.move
      : labels.indexOf(b.review.label as (typeof labels)[number]) -
          labels.indexOf(a.review.label as (typeof labels)[number]) ||
        (b.review.score_loss ?? 0) - (a.review.score_loss ?? 0) ||
        a.move - b.move,
  );
}
export function originalMove(view: GameView, selected: string) {
  const played = path(view.game.nodes, view.game.active_head);
  const playedIds = new Set(played.map((node) => node.id));
  const first = path(view.game.nodes, selected).find((node) => !playedIds.has(node.id));
  return first
    ? (played[played.findIndex((node) => node.id === first.parent) + 1]?.id ?? first.parent!)
    : selected;
}
