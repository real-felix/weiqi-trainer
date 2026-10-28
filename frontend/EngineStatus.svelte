<script lang="ts">
  import { api, type State } from './lib';
  import { ChevronDown } from '@lucide/svelte';
  let {
    appState,
    connected = true,
    onsettings,
  }: { appState: State; connected?: boolean; onsettings: () => void } = $props();
  let message = $state('');
  const workers = $derived([
    ['Human opponent', appState.engine.human],
    ['Strong analyzer', appState.engine.analysis],
  ] as const);
  async function restart() {
    try {
      await api('/engine/restart', {});
      message = 'Restarting both workers…';
    } catch (e) {
      message = (e as Error).message;
    }
  }
</script>

<footer class="engine-bar">
  <details>
    <summary>
      <span class="engine-label">LOCAL ENGINE</span>
      {#each workers as [label, worker] (label)}
        <span class="worker"
          ><span
            class="status-dot"
            class:ready={worker.state === 'ready'}
            class:failed={worker.state === 'error'}
            class:pulse={!['ready', 'error'].includes(worker.state)}
          ></span>{label}<span class="worker-state"
            >{worker.state}{worker.visits && worker.state === 'analyzing'
              ? ` · ${worker.visits} visits`
              : ''}</span
          ></span
        >
      {/each}
      <span class="offline" class:reconnecting={!connected}
        >{connected ? 'Offline by design' : 'Reconnecting…'}</span
      >
      <ChevronDown size={16} class="disclosure-icon" aria-hidden="true" />
    </summary>
    <div class="diagnostics">
      {#each workers as [label, worker] (label)}
        <section>
          <h3>{label}</h3>
          <p>{worker.detail}</p>
          {#if worker.error}<p class="error">{worker.error}</p>{/if}
          <p class="small muted">
            PID {worker.pid ?? '—'} · {worker.version || 'Version not yet checked'}
          </p>
          <pre>{worker.recent_log.join('\n') || 'No process output yet.'}</pre>
        </section>
      {/each}
      <p class="small muted diagnostic-paths">
        Executable: {appState.settings.executable || 'Not found'}<br />Human: {appState.settings
          .human_model || 'Not configured'}<br />Strong: {appState.settings.analysis_model ||
          'Not configured'}
      </p>
      <div class="button-row">
        <button onclick={restart}>Restart / retry engines</button><button onclick={onsettings}
          >Open settings</button
        ><span role="status">{message}</span>
      </div>
    </div>
  </details>
</footer>

<style>
  .engine-bar {
    position: sticky;
    bottom: 0;
    background: #f9f9f5f5;
    border-top: 1px solid var(--line);
    z-index: 10;
    backdrop-filter: blur(12px);
    font-size: var(--text-xs);
  }
  summary {
    display: flex;
    align-items: center;
    gap: 28px;
    min-height: 48px;
    padding: 10px 34px;
    list-style: none;
    cursor: pointer;
    font-size: var(--text-xs);
  }
  .engine-label {
    font-size: 0.75rem;
    letter-spacing: 0.13em;
    color: var(--muted);
  }
  .worker {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .worker-state {
    color: var(--muted);
    margin-left: 3px;
  }
  .offline {
    margin-left: auto;
    color: var(--muted);
    font-size: var(--text-xs);
  }
  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #b29e66;
  }
  .status-dot.ready {
    background: #5c8b5e;
  }
  .status-dot.failed {
    background: #b8363b;
  }
  .pulse {
    animation: pulse 1.5s infinite;
  }
  .diagnostics {
    padding: 20px 34px;
    max-height: 60vh;
    overflow: auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .diagnostics section {
    min-width: 0;
  }
  pre {
    max-height: 150px;
    overflow: auto;
    font-size: var(--text-xs);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    background: var(--paper);
    padding: 12px;
    border-radius: 5px;
  }
  h3 {
    font-size: 1rem;
  }
  .button-row,
  .diagnostic-paths {
    grid-column: 1/-1;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }
  @media (max-width: 800px) {
    summary {
      padding: 12px 16px;
      gap: 12px;
      flex-wrap: wrap;
    }
    .engine-label,
    .offline {
      display: none;
    }
    .offline.reconnecting {
      display: inline;
    }
    .worker-state {
      font-size: var(--text-xs);
    }
    .diagnostics {
      grid-template-columns: 1fr;
      padding: 16px;
    }
  }
</style>
