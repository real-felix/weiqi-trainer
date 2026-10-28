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
it('filters reviewed moves and exposes candidate lines and provenance', async () => {
  const oncandidate = vi.fn();
  render(ReviewPanel, {
    view: fixtureView,
    onselect: vi.fn(),
    oncandidate,
    ondeep: vi.fn(),
    oncancel: vi.fn(),
  });
  expect(
    screen.getByRole('button', { name: /Blunder/ }).querySelector('.classification'),
  ).toHaveStyle({ color: '#b8363b' });
  await fireEvent.change(screen.getByLabelText('Filter by severity'), {
    target: { value: 'Good move' },
  });
  expect(screen.queryByRole('button', { name: /Blunder/ })).not.toBeInTheDocument();
  await fireEvent.click(screen.getByTitle('Explore E16 → F16'));
  expect(oncandidate).toHaveBeenCalledWith(fixtureView.evaluations.b.candidates[0]);
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
        url.endsWith('/state')
          ? fixtureState
          : url.includes('/analysis')
            ? { queued: true }
            : fixtureView,
    })),
  );
  history.replaceState(null, '', '#game/abc');
  render(App);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Review' })).toBeInTheDocument());
  expect(screen.queryByText('Position analysis')).not.toBeInTheDocument();
  expect(screen.queryByText('Black win probability')).not.toBeInTheDocument();
  await fireEvent.click(screen.getByRole('button', { name: 'Review' }));
  expect(screen.getByText('Position analysis')).toBeInTheDocument();
});
