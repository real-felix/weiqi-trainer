<script lang="ts">
  import { tick } from 'svelte';
  import { treeRows, coordinate, colors, type Game, type Review } from './lib';
  import { CornerDownRight } from '@lucide/svelte';
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
  let list = $state<HTMLElement>();
  $effect(() => {
    const target = selected;
    const element = list;
    let cancelled = false;
    void tick().then(() => {
      if (cancelled || !element) return;
      const row = Array.from(element.querySelectorAll<HTMLElement>('[data-node]')).find(
        (row) => row.dataset.node === target,
      );
      if (!row) return;
      // Scroll only this list, never an ancestor that could move the board or navbar.
      const box = element.getBoundingClientRect();
      const item = row.getBoundingClientRect();
      const top = box.top + element.clientTop;
      const bottom = top + element.clientHeight;
      if (item.top < top) element.scrollTop += item.top - top;
      else if (item.bottom > bottom) element.scrollTop += item.bottom - bottom;
    });
    return () => {
      cancelled = true;
    };
  });
</script>

<nav class="history" aria-label="Move variations" bind:this={list}>
  {#each rows as row (row.node.id)}
    {@const classification = byNode[row.node.id]}
    <button
      data-node={row.node.id}
      class:selected={row.node.id === selected}
      class:active={row.active}
      style:padding-left={`${12 + row.depth * 15}px`}
      aria-current={row.node.id === selected ? 'step' : undefined}
      onclick={() => onselect(row.node.id)}
    >
      <span class="move-number">{row.move || '—'}</span>
      {#if row.node.color}<span class:white={row.node.color === 'W'} class="mini-stone"></span>{/if}
      <span>{row.node.color ? coordinate(row.node.point) : 'Starting position'}</span>
      {#if row.fork}<CornerDownRight size={14} aria-hidden="true" />{/if}
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
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    display: grid;
    align-content: start;
    gap: 2px;
    padding: 3px 14px 3px 3px;
    scrollbar-gutter: stable;
  }
  button {
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
    border: 0;
    background: transparent;
    padding: 8px 10px;
    font-size: var(--text-sm);
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
    width: 26px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .tag {
    margin-left: auto;
    font-size: 0.75rem;
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
