import { expect, it } from 'vitest';
import { correction, mistakeRows, originalMove, playerWinrate, playerScore } from './review';
import { fixtureView, fixtureEvaluation } from './fixtures';
it('uses the human perspective consistently for White evaluations', () => {
  expect(playerWinrate(fixtureEvaluation, 'W')).toBe(0.6);
  expect(playerScore(fixtureEvaluation, 'W')).toBe(2);
  expect(playerWinrate(undefined, 'W')).toBeNull();
});
it('lists only played human mistakes and optionally inaccuracies, in deterministic order', () => {
  const view = structuredClone(fixtureView);
  view.reviews.push(
    { ...view.reviews[0], node: 'b', label: 'Blunder' },
    { ...view.reviews[0], node: 'fork', label: 'Blunder' },
  );
  expect(mistakeRows(view, false, 'move').map((row) => row.node.id)).toEqual(['a']);
  view.reviews[0].label = 'Inaccuracy';
  expect(mistakeRows(view, false, 'severity')).toEqual([]);
  expect(mistakeRows(view, true, 'severity').map((row) => row.move)).toEqual([1]);
  view.reviews[0].label = null;
  expect(mistakeRows(view, true, 'move')).toEqual([]);
});
it('gets corrections from the parent position and excludes the actual move', () => {
  const view = structuredClone(fixtureView);
  view.selected = 'a';
  view.evaluations.root = {
    ...fixtureEvaluation,
    candidates: [
      { ...fixtureEvaluation.candidates[0], point: 60, order: 1 },
      { ...fixtureEvaluation.candidates[0], order: 0 },
    ],
  };
  expect(correction(view).before).toBe(view.evaluations.root);
  expect(correction(view).alternatives.map((candidate) => candidate.point)).toEqual([61]);
  expect(originalMove(view, 'fork')).toBe('a');
  expect(correction(view).bestPlayed).toBe(false);
  view.evaluations.root.candidates[0].order = -1;
  expect(correction(view).bestPlayed).toBe(true);
});
