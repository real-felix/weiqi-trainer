<script lang="ts">
  import { coordinate, type Candidate } from './lib';
  let {
    board,
    turn = 'B',
    last = null,
    disabled = false,
    scoring = false,
    dead = [],
    candidates = [],
    onplay,
  }: {
    board: number[];
    turn?: string;
    last?: number | null;
    disabled?: boolean;
    scoring?: boolean;
    dead?: number[];
    candidates?: Candidate[];
    onplay: (point: number) => void;
  } = $props();
  let cursor = $state(60);
  let hovered = $state<number | null>(null);
  const rows = Array.from({ length: 19 }, (_, i) => i);
  const stars = [3, 9, 15];
  const step = 32;
  const gridStart = 48;
  const gridEnd = gridStart + 18 * step;
  const boardSize = gridEnd + gridStart;
  const x = (p: number) => gridStart + (p % 19) * step;
  const y = (p: number) => gridStart + Math.floor(p / 19) * step;
  function key(event: KeyboardEvent, point: number) {
    const offsets: Record<string, number> = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -19,
      ArrowDown: 19,
    };
    if (event.key in offsets) {
      event.preventDefault();
      event.stopPropagation();
      cursor = Math.max(0, Math.min(360, point + offsets[event.key]));
      document.getElementById(`intersection-${cursor}`)?.focus();
    }
  }
</script>

<div class="board-frame">
  <svg
    class="board"
    viewBox={`0 0 ${boardSize} ${boardSize}`}
    role="img"
    aria-label="19 by 19 Go board"
  >
    <defs>
      <linearGradient id="wood" x2="0.8" y2="1"
        ><stop stop-color="#e6c991" /><stop offset="1" stop-color="#d9b579" /></linearGradient
      >
      <radialGradient id="black" cx="32%" cy="25%" r="75%"
        ><stop stop-color="#50514c" /><stop offset="0.6" stop-color="#262723" /><stop
          offset="1"
          stop-color="#171914"
        /></radialGradient
      >
      <radialGradient id="white" cx="32%" cy="25%" r="75%"
        ><stop stop-color="#fffef9" /><stop offset="0.7" stop-color="#f4f1e7" /><stop
          offset="1"
          stop-color="#d8d6cb"
        /></radialGradient
      >
    </defs>
    <rect width={boardSize} height={boardSize} rx="5" fill="url(#wood)" />
    {#each rows as n (n)}
      <path
        d={`M${gridStart} ${y(n * 19)}H${gridEnd} M${x(n)} ${gridStart}V${gridEnd}`}
        stroke="#6e593a"
        stroke-width={n === 0 || n === 18 ? 1.6 : 1}
        opacity="0.75"
      />
      <text x={x(n)} y="20" text-anchor="middle" class="coords">{'ABCDEFGHJKLMNOPQRST'[n]}</text>
      <text x="20" y={y(n * 19) + 4} text-anchor="middle" class="coords">{19 - n}</text>
    {/each}
    {#each stars as a (a)}{#each stars as b (b)}<circle
          cx={gridStart + a * step}
          cy={gridStart + b * step}
          r="3.5"
          fill="#59492f"
        />{/each}{/each}
    {#each board as stone, p (p)}
      {#if stone}
        <circle cx={x(p) + 1} cy={y(p) + 2} r="14.9" fill="#604c33" opacity="0.23" />
        <circle
          cx={x(p)}
          cy={y(p)}
          r="14.5"
          fill={stone === 1 ? 'url(#black)' : 'url(#white)'}
          stroke={stone === 1 ? '#20211c' : '#c8c4b8'}
          stroke-width="0.6"
          opacity={dead.includes(p) ? 0.35 : 1}
        />
        {#if dead.includes(p)}<path
            d={`M${x(p) - 5} ${y(p) - 5}l10 10m-10 0l10 -10`}
            stroke="#b8363b"
            stroke-width="2.5"
          />{/if}
        {#if p === last}<circle
            cx={x(p)}
            cy={y(p)}
            r="4.2"
            fill="none"
            stroke={stone === 1 ? '#faf8ee' : '#50574b'}
            stroke-width="1.7"
          />{/if}
      {/if}
    {/each}
    {#each candidates.slice(0, 5) as c, i (i)}
      {#if c.point !== null && !board[c.point]}
        <circle
          cx={x(c.point)}
          cy={y(c.point)}
          r="12"
          fill={i === 0 ? '#3069ad' : '#e9eee5'}
          stroke="#3069ad"
        />
        <text
          x={x(c.point)}
          y={y(c.point) + 4.5}
          text-anchor="middle"
          fill={i === 0 ? '#fff' : '#285184'}
          font-size="13"
          font-weight="600">{String.fromCharCode(65 + i)}</text
        >
      {/if}
    {/each}
    {#if hovered !== null && !board[hovered] && !disabled && !scoring}
      <circle
        cx={x(hovered)}
        cy={y(hovered)}
        r="14"
        fill={turn === 'B' ? '#252821' : '#fffdf3'}
        opacity="0.35"
      />
    {/if}
  </svg>
  <div
    class="intersections"
    style:inset={`${((gridStart - step / 2) / boardSize) * 100}%`}
    role="grid"
    aria-label="Board intersections. Arrow keys move, Enter places a stone."
  >
    {#each rows as row (row)}
      <div role="row" class="board-row">
        {#each rows as col (col)}
          {@const p = row * 19 + col}
          <button
            id={`intersection-${p}`}
            type="button"
            role="gridcell"
            tabindex={cursor === p ? 0 : -1}
            aria-label={`${coordinate(p)}, ${board[p] === 1 ? 'black stone' : board[p] === 2 ? 'white stone' : 'empty'}${dead.includes(p) ? ', marked dead' : ''}`}
            aria-disabled={disabled || (!scoring && !!board[p])}
            aria-selected={last === p}
            onkeydown={(e) => key(e, p)}
            onfocus={() => {
              cursor = p;
              hovered = p;
            }}
            onblur={() => (hovered = null)}
            onpointerenter={() => (hovered = p)}
            onpointerleave={() => (hovered = null)}
            onclick={() => {
              cursor = p;
              if (!disabled && (scoring || !board[p])) onplay(p);
            }}
          ></button>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .board-frame {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    filter: drop-shadow(0 8px 15px #342c1212);
  }
  .board {
    width: 100%;
    display: block;
  }
  .coords {
    font-size: 12px;
    fill: #5d4c2f;
    font-family: inherit;
  }
  .intersections {
    position: absolute;
    display: grid;
    grid-template-rows: repeat(19, 1fr);
  }
  .board-row {
    display: grid;
    grid-template-columns: repeat(19, 1fr);
    min-height: 0;
  }
  .board-row button {
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 50%;
    width: 100%;
    height: 100%;
    min-height: 0;
    cursor: crosshair;
    box-shadow: none;
  }
  .board-row button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
    background: #fff4;
  }
  .board-row button:hover:not(:disabled) {
    background: transparent;
  }
  .board-row button[aria-disabled='true'] {
    cursor: default;
  }
</style>
