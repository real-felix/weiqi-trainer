<script lang="ts">
  import { api, type Settings } from './lib';
  import { ArrowUpRight, ChevronDown } from '@lucide/svelte';
  let { value, dataDir, onsaved }: { value: Settings; dataDir: string; onsaved: () => void } =
    $props();
  function initialDraft() {
    return structuredClone($state.snapshot(value));
  }
  let draft = $state<Settings>(initialDraft());
  let saving = $state(false);
  let message = $state('');
  let failed = $state(false);
  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    message = '';
    try {
      await api('/settings', draft, 'PUT');
      message = 'Saved. Validating both local workers…';
      failed = false;
      onsaved();
    } catch (e) {
      message = (e as Error).message;
      failed = true;
    } finally {
      saving = false;
    }
  }
</script>

<section class="settings-panel panel" aria-label="Engine settings">
  <div class="section-heading">
    <h2 class="card-title">Local engine</h2>
    <span class="eyebrow">Your machine, your data</span>
  </div>
  <p class="muted">
    KataGo plays through its human SL model and reviews through its strong model. Set each path
    independently.
  </p>
  <form onsubmit={save}>
    <label
      >KataGo executable<input
        bind:value={draft.executable}
        placeholder="/opt/homebrew/bin/katago"
        spellcheck="false"
      /></label
    >
    <label
      >Human SL model<input
        bind:value={draft.human_model}
        placeholder="/absolute/path/b18c384nbt-humanv0.bin.gz"
        spellcheck="false"
      /></label
    >
    <label
      >Strong analysis model<input
        bind:value={draft.analysis_model}
        placeholder="/absolute/path/kata1-….bin.gz"
        spellcheck="false"
      /></label
    >
    <label
      >Analysis configuration<input
        bind:value={draft.config}
        placeholder="/absolute/path/analysis.cfg"
        spellcheck="false"
      /></label
    >
    <details>
      <summary
        >Analysis & resignation<ChevronDown
          size={17}
          class="disclosure-icon"
          aria-hidden="true"
        /></summary
      >
      <div class="settings-grid">
        <label
          >Quick visits<input
            type="number"
            min="1"
            max="4096"
            bind:value={draft.quick_visits}
            required
          /></label
        >
        <label
          >Deep visits<input
            type="number"
            min="64"
            max="20000"
            bind:value={draft.deep_visits}
            required
          /></label
        >
        <label
          >Timeout (seconds)<input
            type="number"
            min="10"
            max="600"
            bind:value={draft.timeout_seconds}
            required
          /></label
        >
        <label class="check"
          ><input type="checkbox" bind:checked={draft.resignation_enabled} />Allow conservative AI
          resignation</label
        >
        <label
          >Maximum AI win probability<input
            type="number"
            min="0"
            max="0.01"
            step="0.001"
            bind:value={draft.resignation_winrate}
            required
          /></label
        >
        <label
          >Minimum points behind<input
            type="number"
            min="20"
            bind:value={draft.resignation_score}
            required
          /></label
        >
        <label
          >Minimum game moves<input
            type="number"
            min="60"
            bind:value={draft.resignation_min_moves}
            required
          /></label
        >
        <label
          >Consecutive confirmations<input
            type="number"
            min="3"
            max="10"
            bind:value={draft.resignation_confirmations}
            required
          /></label
        >
      </div>
    </details>
    <details>
      <summary
        >Move classification calibration<ChevronDown
          size={17}
          class="disclosure-icon"
          aria-hidden="true"
        /></summary
      >
      <p class="muted small">
        These are adjustable heuristics, not validated measures of human skill. Score or win
        probability loss can trigger a label; weaker ranks get a small tolerance. Low-visit
        positions stay unclassified.
      </p>
      <div class="settings-grid">
        <label
          >Algorithm / calibration version<input
            bind:value={draft.classification.version}
            required
          /></label
        >
        <label
          >Minimum visits<input
            type="number"
            min="32"
            bind:value={draft.classification.minimum_visits}
            required
          /></label
        >
        {#each ['Inaccuracy', 'Mistake', 'Blunder'] as label, i (label)}
          <label
            >{label}: points lost<input
              type="number"
              min="0.1"
              step="0.1"
              bind:value={draft.classification.score_thresholds[i]}
              required
            /></label
          >
          <label
            >{label}: probability lost<input
              type="number"
              min="0.001"
              max="1"
              step="0.001"
              bind:value={draft.classification.winrate_thresholds[i]}
              required
            /></label
          >
        {/each}
      </div>
    </details>
    {#if message}<p class:error={failed} role="status">{message}</p>{/if}
    <div class="button-row">
      <button class="primary" disabled={saving}>{saving ? 'Saving…' : 'Save & validate'}</button><a
        href="https://github.com/lightvector/KataGo/releases/tag/v1.15.0"
        target="_blank"
        rel="noreferrer">Human model & setup<ArrowUpRight size={16} aria-hidden="true" /></a
      >
    </div>
  </form>
  <p class="small muted data-path">
    Data directory <code>{dataDir}</code><br />For a consistent backup, stop the app and copy this
    folder. No models are downloaded automatically.
  </p>
</section>

<style>
  form {
    display: grid;
    gap: 20px;
  }
  label {
    display: grid;
    gap: 7px;
    font-size: 1rem;
    font-weight: 550;
  }
  input {
    width: 100%;
  }
  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    margin-top: 15px;
  }
  .check {
    display: flex;
    align-items: center;
    font-weight: 400;
  }
  .check input {
    width: auto;
  }
  .data-path {
    border-top: 1px solid var(--line);
    padding-top: 20px;
    margin-top: 24px;
    line-height: 1.8;
    overflow-wrap: anywhere;
  }
  .button-row {
    margin-top: 5px;
  }
  @media (max-width: 600px) {
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
