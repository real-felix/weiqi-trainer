<script lang="ts">
  import { ArrowRight, Upload } from '@lucide/svelte';
  import { rankLabel, shortDate, lastPlayed, type Game } from './lib';
  let {
    title,
    games,
    emptyMessage,
    onopen,
    onimport,
    busy = false,
  }: {
    title: string;
    games: Game[];
    emptyMessage: string;
    onopen: (id: string) => void;
    onimport?: (event: Event) => void;
    busy?: boolean;
  } = $props();
</script>

<section class="game-card panel" aria-label={title}>
  <div class="card-heading">
    <h2 class="card-title">{title}<span class="game-count">{games.length}</span></h2>
    {#if onimport}
      <label class="import-button">
        <Upload size={17} aria-hidden="true" /> Import SGF
        <input type="file" accept=".sgf,application/x-go-sgf" onchange={onimport} disabled={busy} />
      </label>
    {/if}
  </div>
  {#if games.length}
    <div class="game-list">
      {#each games as game (game.id)}
        <button class="game-row" data-game-id={game.id} onclick={() => onopen(game.id)}>
          <img
            class="game-preview"
            src={`/api/games/${game.id}/preview.svg?head=${game.active_head}&v=1`}
            alt=""
            width="400"
            height="400"
            loading="lazy"
            decoding="async"
          />
          <span class="game-name">
            <strong
              >{game.imported ? 'Imported game' : `vs Human SL ${rankLabel(game.rank)}`}</strong
            >
            <ArrowRight size={18} class="row-arrow" aria-hidden="true" />
          </span>
          <span class="game-details"
            >{shortDate(game.created_at)} · {game.human === 'B' ? 'Black' : 'White'} · {game.mode ===
            'auto'
              ? 'Auto'
              : game.mode === 'manual'
                ? 'Manual'
                : 'SGF'}</span
          >
          <span class="game-outcome">
            <span class="game-result"
              >{game.result ?? (game.status === 'scoring' ? 'Scoring' : 'In progress')}</span
            >
            <span class="last-played">{lastPlayed(game.updated_at)}</span>
          </span>
        </button>
      {/each}
    </div>
  {:else}
    <p class="empty-games">{emptyMessage}</p>
  {/if}
</section>
