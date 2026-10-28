<script lang="ts">
  import { onMount } from 'svelte';
  import Board from './Board.svelte';
  import History from './History.svelte';
  import SettingsPanel from './Settings.svelte';
  import EngineStatus from './EngineStatus.svelte';
  import ReviewPanel from './ReviewPanel.svelte';
  import { watchLiveUpdates } from './live-updates';
  import {
    api,
    rankLabel,
    ranks,
    shortDate,
    lastPlayed,
    path,
    groupAt,
    type State,
    type GameView,
    type Candidate,
    type Scoring,
  } from './lib';

  let appState = $state<State | null>(null);
  let view = $state<GameView | null>(null);
  let gameId = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let follow = $state(true);
  let mode = $state<'play' | 'review'>('play');
  let settingsOpen = $state(false);
  let newGameOpen = $state(false);
  let difficultyMode = $state('auto');
  let rank = $state(-5);
  let startingRank = $state(-5);
  let color = $state('alternate');
  let busy = $state(false);
  let error = $state('');
  let networkError = false;
  let connected = $state(true);
  let dead = $state<number[]>([]);
  let scoring = $state<Scoring | null>(null);
  let scoringMessage = $state('');
  let scorePending = $state(false);
  let scoreHead = $state('');
  let gameRequest = 0;
  let stateRequest = 0;
  let navVersion = 0;
  const ongoing = $derived(appState?.games.filter((g) => !g.result) ?? []);
  const finished = $derived(appState?.games.filter((g) => g.result) ?? []);
  const review = $derived(mode === 'review' || !!view?.game.result);
  const active = $derived(view?.game.active_head === view?.selected);
  const isScoring = $derived(!!view && view.game.status === 'scoring' && active && !review);
  const selectedNode = $derived(view?.game.nodes[view.selected]);
  const currentPath = $derived(view ? path(view.game.nodes, view.selected) : []);
  const playable = $derived(
    !!view &&
      !busy &&
      (review ||
        (!view.thinking &&
          view.game.status === 'playing' &&
          view.position.turn === view.game.human)),
  );
  const evaluation = $derived(view?.evaluations[view.selected]);

  function showError(value: unknown) {
    error = (value as Error).message;
    networkError = value instanceof TypeError;
  }
  async function loadState(reportError = true) {
    const request = ++stateRequest;
    try {
      const result = await api<State>('/state');
      if (request === stateRequest) {
        appState = result;
        if (networkError) {
          error = '';
          networkError = false;
        }
      }
    } catch (e) {
      if (reportError && request === stateRequest) showError(e);
    }
  }
  async function loadGame(reportError = true) {
    if (!gameId) return;
    const id = gameId;
    const request = ++gameRequest;
    const revision = navVersion;
    try {
      const result = await api<GameView>(
        `/games/${id}${!follow && selected ? `?node=${selected}` : ''}`,
      );
      if (request !== gameRequest || revision !== navVersion || id !== gameId) return;
      view = result;
      selected = result.selected;
      if (result.game.result) mode = 'review';
      if (scoreHead && scoreHead !== result.game.active_head) {
        dead = [];
        scoring = null;
        scorePending = false;
        scoreHead = '';
      }
    } catch (e) {
      if (reportError && id === gameId && request === gameRequest && revision === navVersion)
        showError(e);
    }
  }
  async function refresh(reportError = true) {
    await Promise.all([loadState(reportError), loadGame(reportError)]);
  }
  async function openGame(id: string) {
    gameId = id;
    view = null;
    selected = null;
    follow = true;
    mode = 'play';
    dead = [];
    scoring = null;
    error = '';
    scorePending = false;
    navVersion++;
    if (location.hash !== `#game/${id}`) history.replaceState(null, '', `#game/${id}`);
    await loadGame();
    if (view && review) queueAnalysis();
  }
  function home() {
    gameId = null;
    view = null;
    selected = null;
    navVersion++;
    history.replaceState(null, '', location.pathname);
    void loadState();
  }
  function openSettings() {
    home();
    settingsOpen = true;
    setTimeout(
      () => document.getElementById('settings')?.scrollIntoView({ behavior: 'smooth' }),
      0,
    );
  }
  async function selectNode(id: string) {
    selected = id;
    follow = false;
    navVersion++;
    scoring = null;
    await loadGame();
    if (review) queueAnalysis();
  }
  function queueAnalysis() {
    if (view)
      void api(`/games/${view.game.id}/analysis`, { node: view.selected, deep: true }).catch((e) =>
        showError(e),
      );
  }
  async function switchMode(next: 'play' | 'review') {
    mode = next;
    if (next === 'review') queueAnalysis();
    else {
      dead = [];
      scoring = null;
    }
  }
  async function toActive() {
    follow = true;
    navVersion++;
    await loadGame();
    if (review) queueAnalysis();
  }
  function navigate(direction: number) {
    if (!view) return;
    if (direction < 0) {
      const parent = view.game.nodes[view.selected].parent;
      if (parent) void selectNode(parent);
    } else {
      const activePath = path(view.game.nodes, view.game.active_head);
      const i = activePath.findIndex((n) => n.id === view!.selected);
      const next =
        activePath[i + 1]?.parent === view.selected
          ? activePath[i + 1]
          : Object.values(view.game.nodes)
              .filter((n) => n.parent === view!.selected)
              .sort((a, b) => a.created_at.localeCompare(b.created_at))[0];
      if (next) void selectNode(next.id);
    }
  }
  function keyboard(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if (
      target.closest('input,select,textarea,.board-frame') ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey ||
      !view
    )
      return;
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      navigate(-1);
    }
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      navigate(1);
    }
    if (event.key === 'End') {
      event.preventDefault();
      void toActive();
    }
  }
  async function start(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = '';
    try {
      const result = await api<{ id: string }>('/games', {
        mode: difficultyMode,
        rank,
        starting_rank: startingRank,
        color: color === 'alternate' ? null : color,
      });
      newGameOpen = false;
      await loadState();
      await openGame(result.id);
    } catch (e) {
      showError(e);
    } finally {
      busy = false;
    }
  }
  async function play(point: number | null) {
    if (!view || busy) return;
    if (isScoring && point !== null) {
      const group = groupAt(view.position.board, point);
      if (!group.length) return;
      dead = dead.includes(point)
        ? dead.filter((p) => !group.includes(p))
        : [...new Set([...dead, ...group])];
      scoring = null;
      scorePending = false;
      return;
    }
    if (!playable) return;
    busy = true;
    error = '';
    try {
      const result = await api<{ node: string }>(`/games/${view.game.id}/moves`, {
        parent: view.selected,
        point,
        review,
        expected_head: view.game.active_head,
      });
      selected = result.node;
      follow = !review;
      navVersion++;
      await loadGame();
    } catch (e) {
      showError(e);
    } finally {
      busy = false;
    }
  }
  async function continueHere(resume = false) {
    if (!view) return;
    busy = true;
    error = '';
    try {
      await api(`/games/${view.game.id}/continue`, {
        node: view.selected,
        expected_head: view.game.active_head,
        resume,
      });
      follow = true;
      navVersion++;
      dead = [];
      scoring = null;
      scorePending = false;
      await loadGame();
    } catch (e) {
      showError(e);
    } finally {
      busy = false;
    }
  }
  async function resign() {
    if (!view) return;
    busy = true;
    error = '';
    try {
      await api(`/games/${view.game.id}/resign`, { head: view.game.active_head });
      await refresh();
    } catch (e) {
      showError(e);
    } finally {
      busy = false;
    }
  }
  async function candidateLine(candidate: Candidate) {
    if (!view || busy) return;
    busy = true;
    error = '';
    const id = view.game.id;
    let parent = view.selected;
    try {
      for (const point of candidate.pv.slice(0, 12)) {
        const result = await api<{ node: string }>(`/games/${id}/moves`, {
          parent,
          point,
          review: true,
        });
        parent = result.node;
      }
      selected = parent;
      follow = false;
      navVersion++;
      await loadGame();
    } catch (e) {
      showError(e);
      selected = parent;
      follow = false;
      await loadGame();
    } finally {
      busy = false;
    }
  }
  async function checkScore(confirm = false) {
    if (!view || busy) return;
    busy = true;
    error = '';
    scorePending = false;
    scoreHead = view.game.active_head;
    try {
      const result = await api<{
        pending?: boolean;
        message?: string;
        scoring?: Scoring;
        finished?: boolean;
      }>(`/games/${view.game.id}/score`, { head: scoreHead, dead, confirm });
      scoring = result.scoring ?? null;
      scorePending = !!result.pending;
      scoringMessage = result.message ?? result.scoring?.message ?? '';
      if (result.finished) {
        dead = [];
        scoring = null;
        await refresh();
      }
    } catch (e) {
      showError(e);
    } finally {
      busy = false;
    }
  }
  $effect(() => {
    if (
      scorePending &&
      !busy &&
      view?.game.active_head === scoreHead &&
      (view.evaluations[scoreHead]?.visits ?? 0) >= (appState?.settings.deep_visits ?? 256)
    )
      void checkScore();
  });
  async function importSgf(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    busy = true;
    error = '';
    try {
      if (file.size > 1_000_000) throw new Error('SGF exceeds 1 MB');
      const result = await api<{ id: string }>('/games/import', { sgf: await file.text() });
      await loadState();
      await openGame(result.id);
    } catch (e) {
      showError(e);
    } finally {
      busy = false;
      input.value = '';
    }
  }
  onMount(() => {
    void loadState();
    const match = location.hash.match(/^#game\/([a-f0-9-]+)$/);
    if (match) void openGame(match[1]);
    const stopUpdates = watchLiveUpdates({
      onSync: () => refresh(false),
      onConnection: (value) => (connected = value),
    });
    const route = () => {
      const match = location.hash.match(/^#game\/([a-f0-9-]+)$/);
      if (match) void openGame(match[1]);
      else home();
    };
    window.addEventListener('hashchange', route);
    return () => {
      stopUpdates();
      window.removeEventListener('hashchange', route);
    };
  });
</script>

<svelte:window onkeydown={keyboard} />
<div class="app-shell">
  <header class="app-header">
    <button class="brand" onclick={home} aria-label="Weiqi home"
      ><span class="brand-mark"><i></i><i></i></span><span
        >weiqi<span class="brand-subtitle">LOCAL GO TRAINER</span></span
      ></button
    >
    <nav aria-label="Main navigation">
      <button class:current={!gameId} onclick={home}>Home</button>{#if gameId}<span
          class="nav-divider">/</span
        ><span class="current-game">Game</span>{/if}
    </nav>
    <button class="settings-button" onclick={openSettings} aria-label="Settings"
      >⚙ <span>Settings</span></button
    >
  </header>
  {#if error}<div class="error-banner" role="alert">
      <span>{error}</span><button
        class="text-button"
        onclick={() => (error = '')}
        aria-label="Dismiss error">×</button
      >
    </div>{/if}
  {#if !appState}<main class="loading">
      <span class="spinner"></span> Opening your local trainer…
    </main>
  {:else if !gameId}
    <main class="home-main">
      <section class="home-intro">
        <div>
          <p class="eyebrow">A LITTLE PRACTICE, EVERY DAY</p>
          <h1>Find your next move.</h1>
          <p class="intro-copy">
            A quiet place to play, explore, and improve.<br />A human-like opponent. A stronger
            perspective.
          </p>
        </div>
        <div class="intro-art" aria-hidden="true">
          <svg viewBox="0 0 180 160"
            ><path
              d="M20 30h140M20 70h140M20 110h140M20 150h140M30 20v135M70 20v135M110 20v135M150 20v135"
              stroke="#b4c1ac"
              stroke-width="1"
            /><circle cx="70" cy="70" r="18" fill="#30382d" /><circle
              cx="110"
              cy="110"
              r="18"
              fill="#fafaf5"
              stroke="#b5c1ad"
            /><circle cx="110" cy="30" r="18" fill="#30382d" /><circle
              cx="30"
              cy="110"
              r="18"
              fill="#fafaf5"
              stroke="#b5c1ad"
            /></svg
          >
        </div>
      </section>
      <section class="new-game-card">
        <div>
          <span class="eyebrow">YOUR NEXT OPPONENT</span>
          <h2>Human SL <span>{rankLabel(appState.opponent_rank)}</span></h2>
          <p>Adaptive strength · 19 × 19 · AGA · 7.5 komi</p>
        </div>
        <button class="primary new-game" onclick={() => (newGameOpen = !newGameOpen)}
          >New game <span>↗</span></button
        >
      </section>
      {#if newGameOpen}
        <form class="new-game-form panel" onsubmit={start} aria-label="New game options">
          <div class="section-heading">
            <h2>Make room for a game</h2>
            <button
              type="button"
              class="text-button"
              onclick={() => (newGameOpen = false)}
              aria-label="Close new game options">×</button
            >
          </div>
          <div class="form-row">
            <label
              >Opponent mode<select bind:value={difficultyMode}
                ><option value="auto">Auto · adaptive</option><option value="manual"
                  >Manual · choose rank</option
                ></select
              ></label
            >
            {#if difficultyMode === 'manual'}<label
                >Opponent rank<select bind:value={rank}
                  >{#each appState.engine.human.supported_ranks as r (r)}<option value={r}
                      >{rankLabel(r)}</option
                    >{/each}</select
                ></label
              >{/if}
            {#if !appState.initialized}<label
                >Starting difficulty<select bind:value={startingRank}
                  >{#each ranks as r (r)}<option value={r}>{rankLabel(r)}</option>{/each}</select
                ></label
              >{/if}
            <label
              >Your color<select bind:value={color}
                ><option value="alternate">Alternate</option><option value="B">Black</option><option
                  value="W">White</option
                ></select
              ></label
            >
          </div>
          <div class="button-row">
            <button class="primary" disabled={busy || !appState.engine.human.supported_ranks.length}
              >{busy ? 'Starting…' : 'Start game'}</button
            ><span class="small muted"
              >{appState.engine.human.supported_ranks.length
                ? 'Human-model ranks are approximate. Both modes adapt future Auto games.'
                : 'Configure and validate the human model in Settings to play.'}</span
            >
          </div>
        </form>
      {/if}
      <div class="library-heading">
        <h2>Your games <span>{appState.games.length}</span></h2>
        <label class="import-button"
          >Import SGF <span>↓</span><input
            type="file"
            accept=".sgf,application/x-go-sgf"
            onchange={importSgf}
            disabled={busy}
          /></label
        >
      </div>
      {#if !appState.games.length}<section class="empty-appState">
          <span class="empty-board" aria-hidden="true">＋</span>
          <h3>Every game is a fresh start.</h3>
          <p>Your games and every variation will be kept here.</p>
        </section>{/if}
      {#each [{ title: 'In progress', games: ongoing }, { title: 'Finished & imported', games: finished }] as group (group.title)}
        {#if group.games.length}<section class="game-group">
            <h3>{group.title}</h3>
            <div class="game-list">
              {#each group.games as game (game.id)}<button
                  class="game-row"
                  onclick={() => openGame(game.id)}
                  ><span class="game-icon" class:finished={!!game.result}
                    >{game.result ? '○' : '◉'}</span
                  ><span class="game-name"
                    ><strong
                      >{game.imported
                        ? 'Imported game'
                        : `vs Human SL ${rankLabel(game.rank)}`}</strong
                    ><small
                      >{shortDate(game.created_at)} · {game.human === 'B' ? 'Black' : 'White'} · {game.mode ===
                      'auto'
                        ? 'Auto'
                        : game.mode === 'manual'
                          ? 'Manual'
                          : 'SGF'}</small
                    ></span
                  ><span class="game-result"
                    >{game.result ?? (game.status === 'scoring' ? 'Scoring' : 'In progress')}</span
                  ><span class="last-played">{lastPlayed(game.updated_at)}</span><span
                    class="row-arrow">↗</span
                  ></button
                >{/each}
            </div>
          </section>{/if}
      {/each}
      <section id="settings" class="settings-section">
        <button
          class="settings-toggle"
          aria-expanded={settingsOpen}
          onclick={() => (settingsOpen = !settingsOpen)}
          ><span>⚙ &nbsp; Engine & settings</span><span>{settingsOpen ? '−' : '+'}</span></button
        >{#if settingsOpen}<SettingsPanel
            value={appState.settings}
            dataDir={appState.data_dir}
            onsaved={() => void loadState()}
          />{/if}
      </section>
      <p class="home-footnote">Saved locally. No accounts, no clocks, just Go.</p>
    </main>
  {:else if !view}<main class="loading"><span class="spinner"></span> Opening game…</main>
  {:else}
    <main class="game-main">
      <div class="game-heading">
        <div>
          <p class="eyebrow">
            {view.game.imported ? 'IMPORTED RECORD' : `${view.game.mode.toUpperCase()} OPPONENT`}
          </p>
          <h1>
            {view.game.imported ? 'Imported game' : `Human SL ${rankLabel(view.game.rank)}`}
            <span class="versus">·</span>
            <span class="game-you">You play {view.game.human === 'B' ? 'Black' : 'White'}</span>
          </h1>
        </div>
        <div class="mode-switch" aria-label="Game mode">
          {#if !view.game.result}<button
              class:chosen={!review}
              aria-pressed={!review}
              onclick={() => switchMode('play')}>Play</button
            >{/if}<button
            class:chosen={review}
            aria-pressed={review}
            onclick={() => switchMode('review')}>Review</button
          >
        </div>
      </div>
      <div class="workspace">
        <section class="board-column" aria-label="Go game">
          <div class="board-topline">
            <span
              ><span class="mini-stone" class:white={view.position.turn === 'W'}></span>{view.game
                .result
                ? `Result · ${view.game.result}`
                : isScoring
                  ? 'Confirm stone status'
                  : `${view.position.turn === 'B' ? 'Black' : 'White'} to move`}</span
            ><span>Move {currentPath.length - 1} <i>·</i> 19 × 19 <i>·</i> AGA</span>
          </div>
          <Board
            board={view.position.board}
            turn={view.position.turn}
            last={selectedNode?.point ?? null}
            disabled={isScoring ? busy : !playable}
            scoring={isScoring}
            {dead}
            candidates={review ? (evaluation?.candidates ?? []) : []}
            onplay={(p) => void play(p)}
          />
          <div class="board-navigation">
            <div class="history-controls">
              <button
                onclick={() => selectNode(view!.game.root)}
                aria-label="First move"
                title="First position">|‹</button
              ><button
                onclick={() => navigate(-1)}
                aria-label="Previous move"
                title="Previous move (←)">‹</button
              ><span>{currentPath.length - 1}</span><button
                onclick={() => navigate(1)}
                aria-label="Next move"
                title="Next move (→)">›</button
              ><button
                onclick={toActive}
                aria-label="Return to active continuation"
                title="Active continuation (End)">›|</button
              >
            </div>
            <button class="text-button" onclick={toActive}>Return to active line</button>
          </div>
          <p class="board-hint">
            {isScoring
              ? 'Select whole groups on the board to mark them dead.'
              : review
                ? 'Place a stone to explore a variation. Your played game stays preserved.'
                : !active
                  ? 'You’re viewing history. A different move preserves the previous line.'
                  : 'Arrow keys on the board to move · Enter to place a stone'}
          </p>
        </section>
        <aside class="game-sidebar">
          {#if !review}
            <section class="turn-panel">
              <p class="eyebrow">
                {isScoring
                  ? 'TWO PASSES'
                  : view.thinking
                    ? 'OPPONENT’S TURN'
                    : view.position.turn === view.game.human
                      ? 'YOUR TURN'
                      : 'OPPONENT’S TURN'}
              </p>
              <h2>
                {isScoring
                  ? 'Settle the board.'
                  : view.thinking
                    ? 'Considering a move…'
                    : view.position.turn === view.game.human
                      ? 'Take your time.'
                      : 'Ready to continue?'}
              </h2>
              <p class="muted small">
                {isScoring
                  ? 'Agree on dead stones, then confirm the final score. Resume play if either side disagrees.'
                  : view.thinking
                    ? 'The human model is choosing a move. You can still explore the history.'
                    : 'No clock. Find a move that feels right.'}
              </p>
              {#if view.thinking}<div class="thinking-indicator" role="status">
                  <span class="spinner"></span> Human SL {rankLabel(view.game.rank)} is thinking
                </div>{/if}
              {#if isScoring}
                <button class="primary" onclick={() => checkScore()} disabled={busy || scorePending}
                  >{scorePending ? 'Checking stone status…' : 'Check score with opponent'}</button
                >
                {#if scoringMessage}<p class="small" role="status">{scoringMessage}</p>{/if}
                {#if scoring}<div class="score-preview">
                    Black {scoring.score.black.toFixed(1)} · White {scoring.score.white.toFixed(
                      1,
                    )}<strong>{scoring.score.result}</strong>
                  </div>
                  {#if !scoring.agreed && scoring.suggested_dead.length}<button
                      onclick={() => {
                        dead = [...scoring!.suggested_dead];
                        scoring = null;
                      }}>Use opponent’s dead-stone suggestion</button
                    >{/if}{#if scoring.agreed}<button
                      class="primary"
                      onclick={() => checkScore(true)}
                      disabled={busy}>Confirm final result</button
                    >
                    <p class="small muted">
                      This result and its rating update are permanent.
                    </p>{/if}{/if}
                <button onclick={() => continueHere(true)} disabled={busy}
                  >Disagree / resume play</button
                >
              {:else}<div class="play-actions">
                  <button disabled={!playable} onclick={() => play(null)}>Pass</button><button
                    class="subtle-danger"
                    disabled={busy}
                    onclick={resign}>Resign</button
                  >
                </div>
                {#if !active || view.position.turn !== view.game.human}<button
                    class="wide"
                    disabled={busy || view.thinking}
                    onclick={() => continueHere()}>Continue play from here</button
                  >{/if}
                {#if appState.engine.human.state === 'error'}<p class="error small">
                    {appState.engine.human.error}
                  </p>
                  <button
                    onclick={() => api(`/games/${gameId}/retry`, {}).catch((e) => showError(e))}
                    >Retry opponent move</button
                  >{/if}
              {/if}
            </section>
          {:else}<ReviewPanel
              {view}
              onselect={(id) => void selectNode(id)}
              oncandidate={(c) => void candidateLine(c)}
              ondeep={queueAnalysis}
              oncancel={() => void api(`/games/${gameId}/analysis/cancel`, {})}
            />{/if}
          <section class="variation-panel">
            <div class="section-heading">
              <h2>Variations</h2>
              <span class="small muted">{Object.keys(view.game.nodes).length - 1} moves</span>
            </div>
            <History
              game={view.game}
              selected={view.selected}
              {review}
              reviews={view.reviews}
              onselect={(id) => void selectNode(id)}
            />
          </section>
          <div class="game-metadata">
            <span>Captures · B {view.position.captures[0]} / W {view.position.captures[1]}</span><a
              href={`/api/games/${gameId}/sgf`}
              download>Export SGF ↗</a
            >
          </div>
        </aside>
      </div>
    </main>
  {/if}
  {#if appState}<EngineStatus {appState} {connected} onsettings={openSettings} />{/if}
</div>
