<script lang="ts">
  import { ArrowRight, BookOpen, CirclePlay, Upload } from '@lucide/svelte';
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
        <button class="game-row" onclick={() => onopen(game.id)}>
          <span class="game-icon" class:finished={!!game.result}>
            {#if game.result}<BookOpen size={21} aria-hidden="true" />{:else}<CirclePlay
                size={21}
                aria-hidden="true"
              />{/if}
          </span>
          <span class="game-name">
            <strong
              >{game.imported ? 'Imported game' : `vs Human SL ${rankLabel(game.rank)}`}</strong
            >
            <small
              >{shortDate(game.created_at)} · {game.human === 'B' ? 'Black' : 'White'} · {game.mode ===
              'auto'
                ? 'Auto'
                : game.mode === 'manual'
                  ? 'Manual'
                  : 'SGF'}</small
            >
          </span>
          <span class="game-result"
            >{game.result ?? (game.status === 'scoring' ? 'Scoring' : 'In progress')}</span
          >
          <span class="last-played">{lastPlayed(game.updated_at)}</span>
          <ArrowRight size={18} class="row-arrow" aria-hidden="true" />
        </button>
      {/each}
    </div>
  {:else}
    <p class="empty-games">{emptyMessage}</p>
  {/if}
</section>
