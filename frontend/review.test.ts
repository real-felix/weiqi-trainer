import { expect, it } from 'vitest';
import {
  correction,
  mistakeRows,
  originalMove,
  playerWinrate,
  playerScore,
  reviewCandidates,
  suggestionColor,
  suggestionLabel,
} from './review';
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
it('does not promote a weak alternative after excluding the already played AI recommendation', () => {
  const view = structuredClone(fixtureView);
  view.selected = 'b';
  view.game.human = 'W';
  view.game.nodes.b.point = 64; // H16, the actual preferred move.
  view.game.nodes.d15 = { id: 'd15', parent: 'a', color: 'W', point: 79, created_at: 'later' };
  view.reviews.push(
    { ...view.reviews[0], node: 'b', label: 'Good move' },
    { ...view.reviews[0], node: 'd15', label: 'Mistake' },
  );
  view.evaluations.a = {
    ...fixtureEvaluation,
    score_black: -17.265,
    visits: 257,
    candidates: [
      {
        ...fixtureEvaluation.candidates[0],
        point: 64,
        order: 0,
        visits: 237,
        score_black: -17.278,
      },
      { ...fixtureEvaluation.candidates[0], point: 79, order: 1, visits: 8, score_black: -10.568 },
      { ...fixtureEvaluation.candidates[0], point: 78, order: 2, visits: 3, score_black: -11.687 },
    ],
  };
  view.candidate_reviews = {
    a: [
      { point: 64, category: 'Good move', provisional: false, confidence: 237 },
      { point: 79, category: 'Blunder', provisional: true, confidence: 8 },
      { point: 78, category: 'Mistake', provisional: true, confidence: 3 },
    ],
  };
  const focus = correction(view);
  expect(focus.bestPlayed).toBe(true);
  expect(focus.alternatives.map((candidate) => candidate.point)).toEqual([79, 78]);
  const [d15, unreviewed] = focus.alternatives;
  expect(d15.preferred).toBe(false);
  expect(d15.rank).toBe(2);
  expect(d15.provisional).toBe(false); // The cached explored move overrides a rough estimate.
  expect(suggestionLabel(d15)).toBe('Mistake');
  expect(suggestionColor(d15)).toBe('#a64c10');
  expect(unreviewed.rank).toBe(3);
  expect(unreviewed.provisional).toBe(true);
  expect(suggestionLabel(unreviewed)).toBe('Estimated Mistake');
  expect(suggestionColor(unreviewed)).toBe('#a64c10');
  const preferred = reviewCandidates(view, 'a', 'W')[0];
  expect(preferred.point).toBe(64);
  expect(preferred.preferred).toBe(true);
  expect(suggestionColor(preferred)).toBe('#3069ad');
});
