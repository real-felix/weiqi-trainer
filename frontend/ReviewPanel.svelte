<script lang="ts">
  import Charts from './Charts.svelte';
  import { labels, colors, coordinate, path, type GameView, type Candidate } from './lib';
  let {
    view,
    onselect,
    oncandidate,
    ondeep,
    oncancel,
  }: {
    view: GameView;
    onselect: (id: string) => void;
    oncandidate: (c: Candidate) => void;
    ondeep: () => void;
    oncancel: () => void;
  } = $props();
  let severity = $state('All moves');
  let sort = $state('severity');
  const evaluation = $derived(view.evaluations[view.selected]);
  const rows = $derived(
    view.reviews
      .filter((r) => severity === 'All moves' || r.label === severity)
      .sort((a, b) =>
        sort === 'severity'
          ? labels.indexOf(b.label as (typeof labels)[number]) -
              labels.indexOf(a.label as (typeof labels)[number]) ||
            (b.score_loss ?? 0) - (a.score_loss ?? 0)
          : path(view.game.nodes, a.node).length - path(view.game.nodes, b.node).length,
      ),
  );
</script>

<section class="review-panel" aria-label="Position analysis">
  <div class="section-heading">
    <h2>Position analysis</h2>
    <button class="text-button" onclick={ondeep}>Analyze deeper</button>
  </div>
  {#if evaluation}
    <div class="evaluation">
      <div>
        <strong
          >{evaluation.score_black >= 0 ? 'B' : 'W'} +{Math.abs(evaluation.score_black).toFixed(
            1,
          )}</strong
        ><span>estimated lead</span>
      </div>
      <div>
        <strong>{(evaluation.winrate_black * 100).toFixed(1)}%</strong><span
          >Black win probability</span
        >
      </div>
    </div>
    <Charts game={view.game} selected={view.selected} evaluations={view.evaluations} {onselect} />
    <h3>Candidate lines</h3>
    <div class="candidates">
      {#each evaluation.candidates.slice(0, 5) as c, i (i)}
        <button onclick={() => oncandidate(c)} title={`Explore ${c.pv.map(coordinate).join(' → ')}`}
          ><span class="candidate-letter">{String.fromCharCode(65 + i)}</span><span
            ><strong>{coordinate(c.point)}</strong><small
              >{c.pv.slice(1, 5).map(coordinate).join(' → ')}</small
            ></span
          ><span class="candidate-score"
            >{c.score_black >= 0 ? 'B' : 'W'} +{Math.abs(c.score_black).toFixed(1)}<small
              >{c.visits} visits</small
            ></span
          ></button
        >
      {/each}
    </div>
    <details class="analysis-identity">
      <summary>Analysis provenance</summary>
      <p>{evaluation.visits} visits · {new Date(evaluation.created_at).toLocaleString()}</p>
      <code>{evaluation.engine}<br />{evaluation.model}<br />{evaluation.config}</code>
    </details>
  {:else}<p class="muted small">
      Analysis is queued locally. You can keep navigating and exploring.
    </p>{/if}
  <div class="section-heading review-heading">
    <h3>Move review</h3>
    <button class="text-button" onclick={oncancel}>Cancel analysis</button>
  </div>
  <div class="review-filters">
    <select aria-label="Filter by severity" bind:value={severity}
      ><option>All moves</option>{#each labels as label (label)}<option>{label}</option
        >{/each}</select
    ><select aria-label="Sort reviewed moves" bind:value={sort}
      ><option value="severity">Severity</option><option value="move">Move order</option></select
    >
  </div>
  <div class="review-list">
    {#each rows as r (r.node)}
      {@const n = view.game.nodes[r.node]}
      <button
        onclick={() => onselect(r.node)}
        class:current={view.selected === r.node}
        title={`${r.confidence} · ${r.version}`}
      >
        <span class="move-index"
          >{path(view.game.nodes, n.id).length - 1}. {n.color} {coordinate(n.point)}</span
        ><span class="classification" style:color={colors[r.label ?? ''] ?? 'var(--muted)'}
          >{r.label ? '● ' : '◌ '}{r.label ?? 'Pending'}</span
        ><span class="loss">{r.score_loss === null ? '—' : `−${r.score_loss.toFixed(1)}`}</span>
      </button>
    {/each}
  </div>
  <p class="muted small heuristic">
    Labels are rank-aware heuristics. Low-confidence moves stay pending. “AI move” requires a deeply
    analyzed preferred move.
  </p>
</section>

<style>
  .evaluation {
    display: flex;
    justify-content: space-between;
    padding: 13px 0 4px;
  }
  .evaluation div {
    display: grid;
    gap: 5px;
  }
  .evaluation strong {
    font-size: 23px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .evaluation span {
    font-size: 10px;
    color: var(--muted);
  }
  h3 {
    font-size: 12px;
    font-weight: 550;
    margin: 16px 0 10px;
  }
  .candidates {
    display: grid;
    gap: 3px;
  }
  .candidates button {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 0;
    background: var(--paper);
    padding: 9px;
    text-align: left;
    font-size: 12px;
  }
  .candidate-letter {
    background: #e7edf5;
    border-radius: 50%;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    color: #3673ba;
    font-size: 11px;
  }
  .candidates strong {
    font-weight: 500;
  }
  small {
    display: block;
    font-size: 9px;
    color: var(--muted);
    margin-top: 4px;
  }
  .candidate-score {
    margin-left: auto;
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-size: 11px;
  }
  .analysis-identity {
    font-size: 10px;
    color: var(--muted);
    margin-top: 12px;
  }
  .analysis-identity code {
    font-size: 9px;
    overflow-wrap: anywhere;
  }
  .review-heading {
    margin-top: 20px;
  }
  .review-filters {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }
  .review-filters select {
    min-width: 0;
    width: 50%;
    font-size: 11px;
    padding: 7px;
  }
  .review-list {
    max-height: 200px;
    overflow: auto;
    display: grid;
    gap: 2px;
  }
  .review-list button {
    display: flex;
    gap: 7px;
    align-items: center;
    padding: 8px;
    border: 0;
    background: transparent;
    text-align: left;
    font-size: 10px;
  }
  .review-list button.current {
    background: var(--sage);
  }
  .classification {
    margin-left: auto;
    font-size: 10px;
  }
  .loss {
    width: 30px;
    text-align: right;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .heuristic {
    font-size: 10px;
    line-height: 1.7;
    margin-bottom: 0;
  }
</style>
