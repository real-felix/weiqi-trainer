<script lang="ts">
  import { onMount } from 'svelte';
  import Board from './Board.svelte';
  import History from './History.svelte';
  import SettingsPanel from './Settings.svelte';
  import EngineStatus from './EngineStatus.svelte';
  import ReviewPanel from './ReviewPanel.svelte';
  import GameList from './GameList.svelte';
  import {
    Settings as SettingsIcon,
    Plus,
    X,
    ChevronLeft,
    ChevronRight,
    ChevronsLeft,
    ChevronsRight,
    LoaderCircle,
    Download,
  } from '@lucide/svelte';
  import { watchLiveUpdates } from './live-updates';
  import {
    api,
    rankLabel,
    ranks,
    path,
    groupAt,
    type State,
    type GameView,
    type Candidate,
    type Scoring,
  } from './lib';

  // Server snapshots are replaced as a whole, never edited in place. Avoid deep
  // proxies and subscriptions for every node/evaluation in a long review.
  let appState = $state.raw<State | null>(null);
  let view = $state.raw<GameView | null>(null);
  let gameId = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let follow = $state(true);
  let mode = $state<'play' | 'review'>('play');
  let settingsPage = $state(false);
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
  const totalMoves = $derived(view ? path(view.game.nodes, view.game.active_head).length - 1 : 0);
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
    settingsPage = false;
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
    setLocation(`#game/${id}`);
    await loadGame();
    if (view && review) queueAnalysis();
  }
  function home() {
    settingsPage = false;
    gameId = null;
    view = null;
    selected = null;
    navVersion++;
    setLocation('');
    void loadState();
  }
  function openSettings() {
    settingsPage = true;
    gameId = null;
    view = null;
    selected = null;
    navVersion++;
    setLocation('#settings');
    void loadState();
  }
  function setLocation(hash: string) {
    if (location.hash !== hash) history.pushState(null, '', hash || location.pathname);
  }
  function navigateLink(event: MouseEvent, action: () => void) {
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)
      return;
    event.preventDefault();
    action();
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
    const stopUpdates = watchLiveUpdates({
      onSync: () => refresh(false),
      onConnection: (value) => (connected = value),
    });
    const route = () => {
      const match = location.hash.match(/^#game\/([a-f0-9-]+)$/);
      if (location.hash === '#settings') openSettings();
      else if (match) void openGame(match[1]);
      else home();
    };
    route();
    window.addEventListener('hashchange', route);
    return () => {
      stopUpdates();
      window.removeEventListener('hashchange', route);
    };
  });
</script>

<svelte:window onkeydown={keyboard} />
<div class="app-shell" class:game-open={!!gameId}>
  <header class="app-header">
    <a
      class="brand"
      href="/"
      onclick={(event) => navigateLink(event, home)}
      aria-label="Weiqi Trainer home"
      ><img class="brand-mark" src="/brand.svg" width="34" height="37" alt="" /><span
        class="brand-name">Weiqi Trainer<span class="brand-subtitle">LOCAL GO PARTNER</span></span
      ></a
    >
    {#if gameId && view}
      <div class="navbar-game">
        <div>
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
    {/if}
    <a
      href="#settings"
      class="settings-button"
      aria-current={settingsPage ? 'page' : undefined}
      onclick={(event) => navigateLink(event, openSettings)}
      ><SettingsIcon size={19} aria-hidden="true" /><span>Settings</span></a
    >
  </header>
  {#if error}<div class="error-banner" role="alert">
      <span>{error}</span><button
        class="text-button"
        onclick={() => (error = '')}
        aria-label="Dismiss error"><X size={19} aria-hidden="true" /></button
      >
    </div>{/if}
  {#if !appState}<main class="loading">
      <LoaderCircle class="spinner" size={19} aria-hidden="true" /> Opening Weiqi Trainer…
    </main>
  {:else if settingsPage}
    <main class="settings-main">
      <header class="page-intro">
        <h1>Settings</h1>
        <p>Configure your local engines and analysis.</p>
      </header>
      <SettingsPanel
        value={appState.settings}
        dataDir={appState.data_dir}
        onsaved={() => void loadState()}
      />
    </main>
  {:else if !gameId}
    <main class="home-main">
      <section class="home-intro">
        <div>
          <h1>Find your next move.</h1>
          <p class="intro-copy">Play against a human-like opponent. Learn from every game.</p>
        </div>
      </section>
      <section class="new-game-card">
        <div>
          <h2 class="card-title">Human SL {rankLabel(appState.opponent_rank)}</h2>
          <p>Adaptive strength · 19 × 19 · AGA · 7.5 komi</p>
        </div>
        <button class="primary new-game" onclick={() => (newGameOpen = !newGameOpen)}
          >New game <Plus size={19} aria-hidden="true" /></button
        >
      </section>
      {#if newGameOpen}
        <form class="new-game-form panel" onsubmit={start} aria-label="New game options">
          <div class="section-heading">
            <h2 class="card-title">Make room for a game</h2>
            <button
              type="button"
              class="text-button"
              onclick={() => (newGameOpen = false)}
              aria-label="Close new game options"><X size={19} aria-hidden="true" /></button
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
      <GameList
        title="Ongoing games"
        games={ongoing}
        emptyMessage="No ongoing games."
        onopen={(id) => void openGame(id)}
      />
      <GameList
        title="Review games"
        games={finished}
        emptyMessage="No games to review yet."
        onopen={(id) => void openGame(id)}
        onimport={importSgf}
        {busy}
      />
    </main>
  {:else if !view}<main class="loading">
      <LoaderCircle class="spinner" size={19} aria-hidden="true" /> Opening game…
    </main>
  {:else}
    <main class="game-main">
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
            ><span>Move {currentPath.length - 1}/{totalMoves} <i>·</i> 19 × 19 <i>·</i> AGA</span>
          </div>
          <div class="board-area">
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
          </div>
          {#if isScoring}
            <p class="board-hint">Select whole groups on the board to mark them dead.</p>
          {/if}
          <div class="board-navigation">
            <div class="history-controls">
              <button
                onclick={() => selectNode(view!.game.root)}
                aria-label="First move"
                title="First position"><ChevronsLeft size={21} aria-hidden="true" /></button
              ><button
                onclick={() => navigate(-1)}
                aria-label="Previous move"
                title="Previous move (←)"><ChevronLeft size={21} aria-hidden="true" /></button
              ><span
                aria-label={`Move ${currentPath.length - 1} of ${totalMoves} on the active line`}
                >{currentPath.length - 1}/{totalMoves}</span
              ><button onclick={() => navigate(1)} aria-label="Next move" title="Next move (→)"
                ><ChevronRight size={21} aria-hidden="true" /></button
              ><button
                onclick={toActive}
                aria-label="Return to active continuation"
                title="Active continuation (End)"
                ><ChevronsRight size={21} aria-hidden="true" /></button
              >
            </div>
            <button class="text-button" onclick={toActive}>Return to active line</button>
          </div>
        </section>
        <aside class="game-sidebar" class:reviewing={review}>
          <div class="sidebar-content">
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
                {#if isScoring}
                  <h2>Settle the board.</h2>
                  <p class="muted small">
                    Agree on dead stones, then confirm the final score. Resume play if either side
                    disagrees.
                  </p>
                {:else if view.thinking}
                  <h2>Considering a move…</h2>
                  <p class="muted small">
                    The human model is choosing a move. You can still explore the history.
                  </p>
                {:else if view.position.turn !== view.game.human}
                  <h2>Ready to continue?</h2>
                {/if}
                {#if view.thinking}<div class="thinking-indicator" role="status">
                    <LoaderCircle class="spinner" size={18} aria-hidden="true" /> Human SL {rankLabel(
                      view.game.rank,
                    )} is thinking
                  </div>{/if}
                {#if isScoring}
                  <button
                    class="primary"
                    onclick={() => checkScore()}
                    disabled={busy || scorePending}
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
          </div>
          <div class="game-metadata">
            <span>Captures · B {view.position.captures[0]} / W {view.position.captures[1]}</span><a
              href={`/api/games/${gameId}/sgf`}
              download><Download size={16} aria-hidden="true" /> Export SGF</a
            >
          </div>
        </aside>
      </div>
    </main>
  {/if}
  {#if appState}<EngineStatus {appState} {connected} onsettings={openSettings} />{/if}
</div>
