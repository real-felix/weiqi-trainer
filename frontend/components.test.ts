import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { it, expect, vi, afterEach } from 'vitest';
import History from './History.svelte';
import Board from './Board.svelte';
import ReviewPanel from './ReviewPanel.svelte';
import EngineStatus from './EngineStatus.svelte';
import App from './App.svelte';
import GameList from './GameList.svelte';
import { fixtureGame, fixtureView, fixtureState } from './fixtures';
afterEach(() => {
  vi.unstubAllGlobals();
  history.replaceState(null, '', '/');
});
it('uses one lazy cached image per Home game and changes its URL only with the active head', async () => {
  const onopen = vi.fn();
  const rendered = render(GameList, {
    title: 'Ongoing games',
    games: [fixtureGame],
    emptyMessage: 'No games',
    onopen,
  });
  const image = rendered.container.querySelector('img')!;
  const url = image.getAttribute('src');
  expect(url).toContain(`head=${fixtureGame.active_head}`);
  expect(image).toHaveAttribute('loading', 'lazy');
  expect(rendered.container.querySelectorAll('svg.board, [role="gridcell"]')).toHaveLength(0);
  await fireEvent.click(screen.getByRole('button', { name: /vs Human SL/ }));
  expect(onopen).toHaveBeenCalledWith(fixtureGame.id);
  await rendered.rerender({
    games: [{ ...fixtureGame, updated_at: 'later', nodes: { ...fixtureGame.nodes } }],
  });
  expect(image.getAttribute('src')).toBe(url);
  await rendered.rerender({ games: [{ ...fixtureGame, active_head: 'new-head' }] });
  expect(image.getAttribute('src')).toContain('head=new-head');
});
it('navigates variations with active and selected cues, hiding labels outside review', async () => {
  const onselect = vi.fn();
  const rendered = render(History, {
    game: fixtureGame,
    selected: 'fork',
    reviews: fixtureView.reviews,
    onselect,
  });
  expect(screen.queryByText('Blunder')).not.toBeInTheDocument();
  await fireEvent.click(screen.getByRole('button', { name: /1.*D16/ }));
  expect(onselect).toHaveBeenCalledWith('a');
  await rendered.rerender({
    game: fixtureGame,
    selected: 'fork',
    reviews: fixtureView.reviews,
    review: true,
    onselect,
  });
  expect(screen.getByText('Blunder')).toBeInTheDocument();
  expect(screen.getByRole('button', { current: 'step' })).toHaveTextContent('E16');
});
it('supports roving board keyboard focus and blocks duplicate occupied moves', async () => {
  const onplay = vi.fn();
  render(Board, { board: fixtureView.position.board, onplay });
  const occupied = screen.getByRole('gridcell', { name: 'D16, black stone' });
  await fireEvent.click(occupied);
  expect(onplay).not.toHaveBeenCalled();
  await fireEvent.keyDown(occupied, { key: 'ArrowRight' });
  expect(screen.getByRole('gridcell', { name: 'E16, empty' })).toHaveFocus();
  await fireEvent.click(screen.getByRole('gridcell', { name: 'E16, empty' }));
  expect(onplay).toHaveBeenCalledWith(61);
});
it('puts the correction before human mistakes and hides engine details', async () => {
  const oncandidate = vi.fn();
  const view = structuredClone(fixtureView);
  view.selected = 'a';
  view.evaluations.root = { ...view.evaluations.b };
  view.evaluations.a = { ...view.evaluations.b };
  render(ReviewPanel, { view, onselect: vi.fn(), oncandidate, ondeep: vi.fn(), oncancel: vi.fn() });
  expect(screen.getByRole('region', { name: 'Selected move' })).toBeInTheDocument();
  const mistake = screen.getByRole('region', { name: 'Your mistakes' }).querySelector('.badge');
  expect(mistake).toHaveStyle({ color: '#b8363b' });
  expect(screen.getByText('Cancel analysis')).not.toBeVisible();
  expect(screen.queryByText('Analysis provenance')).not.toBeInTheDocument();
  expect(screen.queryByLabelText('Analyzed positions')).not.toBeInTheDocument();
  await fireEvent.click(screen.getByTitle('Explore E16 → F16'));
  expect(oncandidate).toHaveBeenCalledWith(view.evaluations.root.candidates[0]);
  await fireEvent.click(screen.getByText('Analysis details', { exact: true }));
  expect(screen.getByText('Cancel analysis')).toBeVisible();
});
it('distinguishes bounded low-confidence analysis from pending work', () => {
  const view = structuredClone(fixtureView);
  view.selected = 'a';
  view.reviews[0].label = null;
  view.reviews[0].confidence = '32 visits · low confidence · search limit reached';
  view.analysis.queued = 0;
  render(ReviewPanel, {
    view,
    onselect: vi.fn(),
    oncandidate: vi.fn(),
    ondeep: vi.fn(),
    oncancel: vi.fn(),
  });
  const progress = screen
    .getByRole('region', { name: 'Your mistakes' })
    .querySelector('.analysis-progress');
  expect(progress).toHaveTextContent(/0\/1 moves reviewed\s*·\s*1 low confidence/);
  expect(progress).not.toHaveTextContent('Pending');
  expect(screen.getByText('Low confidence', { exact: true })).toBeInTheDocument();
});
it('shows separate human and analyzer failures', () => {
  const appState = structuredClone(fixtureState);
  appState.engine.human.error = 'Missing human model';
  appState.engine.human.state = 'error';
  render(EngineStatus, { appState, onsettings: vi.fn() });
  expect(screen.getByText('Missing human model')).toBeInTheDocument();
  expect(screen.getByRole('heading', { name: 'Strong analyzer' })).toBeInTheDocument();
});
it('requires explicit Review and never displays evaluations in Play', async () => {
  const view = structuredClone(fixtureView);
  view.game.human = 'W';
  vi.stubGlobal(
    'EventSource',
    class {
      addEventListener() {}
      close() {}
    },
  );
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string) => ({
      ok: true,
      json: async () =>
        url.endsWith('/state') ? fixtureState : url.includes('/analysis') ? { queued: true } : view,
    })),
  );
  history.replaceState(null, '', '#game/abc');
  render(App);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Review' })).toBeInTheDocument());
  expect(screen.getByText('Your opponent', { exact: true })).toBeInTheDocument();
  expect(screen.getByRole('img', { name: 'Black stone' })).not.toHaveClass('white');
  expect(screen.queryByRole('region', { name: 'Your mistakes' })).not.toBeInTheDocument();
  expect(screen.queryByText('Black win probability')).not.toBeInTheDocument();
  await fireEvent.click(screen.getByRole('button', { name: 'Review' }));
  expect(screen.getByText('Your turn', { exact: true })).toBeInTheDocument();
  expect(screen.getByRole('img', { name: 'White stone' })).toHaveClass('white');
  expect(screen.getByRole('region', { name: 'Your mistakes' })).toBeInTheDocument();
});
