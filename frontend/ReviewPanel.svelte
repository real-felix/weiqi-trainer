<script lang="ts">
  import Charts from './Charts.svelte';
  import { ChevronDown, ChevronRight, LoaderCircle } from '@lucide/svelte';
  import { colors, coordinate, path, type GameView, type Candidate } from './lib';
  import {
    correction,
    mistakeRows,
    lead,
    percent,
    playerWinrate,
    reviewCandidates,
    suggestionColor,
    suggestionTitle,
  } from './review';
  let {
    view,
    origin = null,
    onselect,
    oncandidate,
    ondeep,
    oncancel,
  }: {
    view: GameView;
    origin?: string | null;
    onselect: (id: string) => void;
    oncandidate: (candidate: Candidate) => void;
    ondeep: () => void;
    oncancel: () => void;
  } = $props();
  let inaccuracies = $state(false);
  let sort = $state<'move' | 'severity'>('move');
  const focus = $derived(correction(view));
  const rows = $derived(mistakeRows(view, inaccuracies, sort));
  const evaluation = $derived(view.evaluations[view.selected]);
  const candidates = $derived(
    origin ? reviewCandidates(view, view.selected, view.position.turn) : focus.alternatives,
  );
  const beforeWin = $derived(playerWinrate(focus.before, view.game.human));
  const afterWin = $derived(playerWinrate(focus.actual, view.game.human));
  const played = $derived(path(view.game.nodes, view.game.active_head));
  const reviewed = $derived(new Map(view.reviews.map((review) => [review.node, review])));
  const move = $derived(path(view.game.nodes, view.selected).length - 1);
  const moves = $derived(played.filter((node) => node.color !== null));
  const unresolved = $derived(moves.filter((node) => !reviewed.get(node.id)?.label));
  const lowConfidence = $derived(
    unresolved.filter((node) => reviewed.get(node.id)?.confidence.includes('search limit reached'))
      .length,
  );
  const pending = $derived(unresolved.length - lowConfidence);
  const progress = $derived(
    view.analysis ?? {
      completed: Object.keys(view.evaluations).length,
      total: Object.keys(view.game.nodes).length,
      queued: 0,
      running: false,
    },
  );
</script>

<Charts game={view.game} selected={view.selected} evaluations={view.evaluations} {onselect} />
<section class="review-panel" aria-label="Move review">
  <section class="correction" aria-label={origin ? 'Explored position' : 'Selected move'}>
    <div class="section-heading">
      <h2>{origin ? 'Exploring' : focus.node?.color ? `Move ${move}` : 'Starting position'}</h2>
      <button class="text-button" onclick={ondeep}>Analyze deeper</button>
    </div>
    {#if origin}
      <div class="comparison">
        <div>
          <span>Original</span><strong>{lead(view.evaluations[origin])}</strong><span
            >{percent(playerWinrate(view.evaluations[origin], view.game.human))}</span
          >
        </div>
        <div>
          <span>Alternative</span><strong>{lead(evaluation)}</strong><span
            >{percent(playerWinrate(evaluation, view.game.human))}</span
          >
        </div>
      </div>
    {:else if focus.node?.color}
      <div class="played-move">
        <strong>Played {coordinate(focus.node.point)}</strong><span
          class="badge"
          style:color={colors[focus.review?.label ?? ''] ?? 'var(--muted)'}
          >{focus.review?.label ??
            (focus.review?.confidence.includes('search limit reached')
              ? 'Low confidence'
              : 'Pending')}</span
        >
      </div>
      <div class="move-impact">
        <span
          >Points lost <strong
            >{focus.review?.score_loss == null
              ? 'Pending'
              : focus.review.score_loss.toFixed(1)}</strong
          ></span
        ><span
          >{view.game.human === 'B' ? 'Black' : 'White'} win probability
          <strong>{percent(beforeWin)} → {percent(afterWin)}</strong></span
        >
      </div>
    {/if}
    {#if !origin && focus.bestPlayed}<p class="small muted">Best move played</p>{/if}
    {#if candidates[0]}
      {#if origin || !focus.bestPlayed}<button
          class="suggestion"
          class:best={candidates[0].preferred}
          onclick={() => oncandidate(candidates[0])}
          title={`Explore ${candidates[0].pv.map(coordinate).join(' → ')}`}
        >
          <span
            class="candidate-letter"
            style:color={suggestionColor(candidates[0])}
            title={suggestionTitle(candidates[0])}>A</span
          ><span
            ><strong>{origin ? 'Continue' : 'Try'} {coordinate(candidates[0].point)}</strong><small
              >#{candidates[0].rank} · {candidates[0].pv.slice(1, 5).map(coordinate).join(' → ') ||
                'Suggested move'}</small
            ></span
          ><ChevronRight size={16} aria-hidden="true" />
        </button>{/if}
      {#if candidates.length > 1 || (!origin && focus.bestPlayed)}<details
          class="other-suggestions"
        >
          <summary
            >Other suggestions<ChevronDown
              size={15}
              class="disclosure-icon"
              aria-hidden="true"
            /></summary
          >
          <div class="suggestions">
            {#each candidates.slice(!origin && focus.bestPlayed ? 0 : 1, 5) as candidate, index (candidate.point)}<button
                class="suggestion"
                onclick={() => oncandidate(candidate)}
                title={`Explore ${candidate.pv.map(coordinate).join(' → ')}`}
                ><span
                  class="candidate-letter"
                  style:color={suggestionColor(candidate)}
                  title={suggestionTitle(candidate)}
                  >{String.fromCharCode((!origin && focus.bestPlayed ? 65 : 66) + index)}</span
                ><span
                  ><strong>{coordinate(candidate.point)}</strong><small
                    >#{candidate.rank} · {candidate.pv
                      .slice(1, 5)
                      .map(coordinate)
                      .join(' → ')}</small
                  ></span
                ><span class="candidate-evaluation"
                  >{lead(candidate)}<small
                    >{percent(playerWinrate(candidate, view.game.human))}</small
                  ></span
                ></button
              >{/each}
          </div>
        </details>{/if}
    {:else}<p class="pending small" role="status">
        {(origin ? evaluation : focus.node?.parent ? focus.before : evaluation)
          ? 'No alternative available.'
          : 'Suggestions pending'}
      </p>{/if}
  </section>
  <section class="mistakes" aria-label="Your mistakes">
    <div class="section-heading">
      <h2>Your mistakes</h2>
      <span class="muted small">{rows.length}</span>
    </div>
    <div class="review-filters">
      <label><input type="checkbox" bind:checked={inaccuracies} /> Include inaccuracies</label
      ><select aria-label="Sort mistakes" bind:value={sort}
        ><option value="move">Move order</option><option value="severity">Severity</option></select
      >
    </div>
    <div class="review-list">
      {#each rows as row (row.node.id)}<button
          class:current={view.selected === row.node.id}
          onclick={() => onselect(row.node.id)}
          aria-current={view.selected === row.node.id ? 'step' : undefined}
        >
          <span class="move-index">{row.move}. {coordinate(row.node.point)}</span><span
            class="badge"
            style:color={colors[row.review.label!]}>{row.review.label}</span
          ><span class="loss">−{row.review.score_loss?.toFixed(1) ?? 'Pending'} pts</span><span
            class="winrate">{percent(row.winrate)}</span
          >
        </button>{/each}
    </div>
    {#if !rows.length}<p class="small muted">
        {pending
          ? 'Mistakes will appear as analysis completes.'
          : lowConfidence
            ? 'Some moves remain uncertain after analysis.'
            : inaccuracies
              ? 'No mistakes or inaccuracies found.'
              : 'No mistakes or blunders found.'}
      </p>{/if}
    <p class="analysis-progress small muted" role="status">
      {#if progress.running || progress.queued}<LoaderCircle
          size={13}
          class="spinner"
          aria-hidden="true"
        /> Analyzing ·
      {/if}{moves.length - unresolved.length}/{moves.length} moves reviewed{#if pending}
        · {pending} Pending{/if}{#if lowConfidence}
        · {lowConfidence} low confidence{/if}
    </p>
  </section>
  <details class="analysis-details">
    <summary
      >Analysis details<ChevronDown size={15} class="disclosure-icon" aria-hidden="true" /></summary
    >
    <p>{progress.completed}/{progress.total} positions cached · {progress.queued} queued</p>
    {#if evaluation}<p>
        {evaluation.visits} visits · {new Date(evaluation.created_at).toLocaleString()}
      </p>
      <code>{evaluation.engine}<br />{evaluation.model}<br />{evaluation.config}</code>{/if}
    {#if focus.review}<p>{focus.review.confidence} · {focus.review.version}</p>{/if}
    <button class="text-button" onclick={oncancel}>Cancel analysis</button>
  </details>
</section>

<style>
  .review-panel button {
    scroll-margin-block: 180px 12px;
  }
  h2 {
    font-size: 1rem;
  }
  .section-heading {
    margin-bottom: 12px;
    gap: 8px;
  }
  .correction {
    padding-bottom: 18px;
    border-bottom: 1px solid var(--line);
  }
  .played-move {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: center;
    font-size: var(--text-sm);
  }
  .badge {
    padding: 3px 7px;
    border-radius: 4px;
    background: color-mix(in srgb, currentColor 10%, transparent);
    font-size: 0.75rem;
    font-weight: 550;
    white-space: nowrap;
  }
  .move-impact {
    display: grid;
    gap: 8px;
    margin: 12px 0;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .move-impact > span {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
  }
  .move-impact strong {
    color: var(--ink);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .comparison {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    font-size: var(--text-sm);
    margin-bottom: 12px;
  }
  .comparison > div {
    display: grid;
    gap: 6px;
  }
  .comparison span {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 9px;
    border: 0;
    text-align: left;
    background: var(--surface);
    font-size: var(--text-sm);
  }
  .best {
    margin-top: 12px;
  }
  .suggestion > :global(svg) {
    margin-left: auto;
  }
  .candidate-letter {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    color: var(--muted);
    background: color-mix(in srgb, currentColor 12%, var(--surface));
    flex-shrink: 0;
    font-weight: 550;
  }
  small {
    display: block;
    color: var(--muted);
    font-size: 0.75rem;
    margin-top: 4px;
  }
  .candidate-evaluation {
    text-align: right;
    margin-left: auto;
    font-size: 0.75rem;
  }
  .other-suggestions {
    margin-top: 12px;
  }
  .suggestions {
    display: grid;
    gap: 4px;
    margin-top: 8px;
  }
  .mistakes {
    padding-top: 18px;
  }
  .review-filters {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .review-filters label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 0.75rem;
  }
  input {
    min-height: auto;
    margin: 0;
    accent-color: var(--accent);
  }
  select {
    font-size: 0.75rem;
    padding: 6px;
    min-height: 34px;
  }
  .review-list {
    display: grid;
    gap: 4px;
  }
  .review-list button {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 7px;
    align-items: center;
    padding: 9px;
    border: 0;
    background: transparent;
    font-size: var(--text-xs);
    text-align: left;
  }
  .review-list button.current {
    background: var(--sage);
  }
  .loss,
  .winrate {
    color: var(--muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .winrate {
    text-align: right;
  }
  .analysis-progress {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
  }
  .analysis-details {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
    font-size: 0.75rem;
    color: var(--muted);
  }
  .analysis-details p {
    margin: 10px 0;
  }
  code {
    overflow-wrap: anywhere;
  }
</style>
