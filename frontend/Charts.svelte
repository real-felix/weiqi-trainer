<script lang="ts">
  import { path, type Game, type Evaluation } from './lib';
  let {
    game,
    selected,
    evaluations,
    onselect,
  }: {
    game: Game;
    selected: string;
    evaluations: Record<string, Evaluation>;
    onselect: (id: string) => void;
  } = $props();
  const activeLine = $derived(path(game.nodes, game.active_head));
  const line = $derived(
    activeLine.some((n) => n.id === selected) ? activeLine : path(game.nodes, selected),
  );
  const data = $derived(
    line.map((n, i) => ({ node: n.id, move: i, e: evaluations[n.id] })).filter((d) => d.e),
  );
  const scale = $derived(Math.max(10, ...data.map((d) => Math.abs(d.e.score_black))));
  const width = 300,
    height = 80;
  const x = (i: number) => 30 + (i / Math.max(1, line.length - 1)) * 260;
</script>

<div class="charts">
  {#each ['score', 'winrate'] as kind (kind)}
    <div class="chart-label">
      <span>{kind === 'score' ? 'Score estimate' : 'Win probability'}</span><span
        >Black’s perspective</span
      >
    </div>
    <svg
      viewBox={`0 0 ${width} ${height}`}
      role="img"
      aria-label={kind === 'score'
        ? 'Black score estimate by move'
        : 'Black win probability by move'}
    >
      <path d="M30 40H290" stroke="var(--line)" stroke-dasharray="3 4" />
      <text x="0" y="13" fill="var(--muted)" font-size="10"
        >{kind === 'score' ? `B+${scale.toFixed(0)}` : '100%'}</text
      >
      <text x="0" y="75" fill="var(--muted)" font-size="10"
        >{kind === 'score' ? `W+${scale.toFixed(0)}` : '0%'}</text
      >
      <polyline
        points={data
          .map(
            (d) =>
              `${x(d.move)},${kind === 'score' ? 40 - (d.e.score_black / scale) * 32 : 72 - d.e.winrate_black * 64}`,
          )
          .join(' ')}
        fill="none"
        stroke={kind === 'score' ? '#47765b' : '#647a9e'}
        stroke-width="2"
      />
      {#each data as d (d.node)}<circle
          cx={x(d.move)}
          cy={kind === 'score' ? 40 - (d.e.score_black / scale) * 32 : 72 - d.e.winrate_black * 64}
          r="2.5"
          fill={kind === 'score' ? '#47765b' : '#647a9e'}
          ><title
            >Move {d.move}: {kind === 'score'
              ? `${d.e.score_black.toFixed(1)} points`
              : `${(d.e.winrate_black * 100).toFixed(1)}%`}</title
          ></circle
        >{/each}
    </svg>
  {/each}
  <div class="chart-navigation" aria-label="Analyzed positions">
    {#each data as d (d.node)}<button
        title={`Review move ${d.move}`}
        aria-label={`Chart move ${d.move}`}
        onclick={() => onselect(d.node)}>{d.move}</button
      >{/each}
  </div>
</div>

<style>
  .chart-label {
    display: flex;
    justify-content: space-between;
    font-size: var(--text-sm);
    color: var(--muted);
    margin: 10px 0 0;
  }
  .chart-label span:last-child {
    font-size: 0.75rem;
  }
  svg {
    display: block;
    width: 100%;
    height: 96px;
  }
  .chart-navigation {
    display: flex;
    gap: 3px;
    overflow: auto;
    max-height: 35px;
  }
  .chart-navigation button {
    font-size: 0.75rem;
    border: 0;
    padding: 3px 5px;
    min-height: 20px;
    background: transparent;
    color: var(--muted);
  }
</style>
