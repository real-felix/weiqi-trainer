<script lang="ts">
  import { treeRows, coordinate, colors, type Game, type Review } from './lib';
  let {
    game,
    selected,
    review = false,
    reviews = [],
    onselect,
  }: {
    game: Game;
    selected: string;
    review?: boolean;
    reviews?: Review[];
    onselect: (id: string) => void;
  } = $props();
  const rows = $derived(treeRows(game));
  const byNode = $derived(Object.fromEntries(reviews.map((r) => [r.node, r])));
</script>

<nav class="history" aria-label="Move variations">
  {#each rows as row (row.node.id)}
    {@const classification = byNode[row.node.id]}
    <button
      class:selected={row.node.id === selected}
      class:active={row.active}
      style:padding-left={`${12 + row.depth * 15}px`}
      aria-current={row.node.id === selected ? 'step' : undefined}
      onclick={() => onselect(row.node.id)}
    >
      <span class="move-number">{row.move || '—'}</span>
      {#if row.node.color}<span class:white={row.node.color === 'W'} class="mini-stone"></span>{/if}
      <span
        >{row.node.color ? coordinate(row.node.point) : 'Starting position'}{row.fork
          ? ' ↳'
          : ''}</span
      >
      {#if row.node.id === game.active_head}<span class="tag">active</span>{/if}
      {#if review && classification?.label}<span
          class="classification-dot"
          style:background={colors[classification.label]}
          title={classification.label}
        ></span><span class="sr-only">{classification.label}</span>{/if}
    </button>
  {/each}
</nav>

<style>
  .history {
    max-height: 250px;
    overflow: auto;
    display: grid;
    gap: 2px;
    padding: 3px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
    border: 0;
    background: transparent;
    padding: 8px 10px;
    font-size: 12px;
    color: var(--muted);
    min-height: 34px;
    box-shadow: none;
  }
  button.active {
    color: var(--ink);
  }
  button.selected {
    background: var(--sage);
    color: var(--green);
  }
  .move-number {
    width: 20px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .tag {
    margin-left: auto;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }
  .classification-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    margin-left: auto;
  }
</style>
