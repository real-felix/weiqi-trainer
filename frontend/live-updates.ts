// Replace sleep-broken streams explicitly instead of relying only on EventSource's retries.
export function watchLiveUpdates({
  onSync,
  onConnection,
}: {
  onSync: () => void | Promise<void>;
  onConnection: (connected: boolean) => void;
}) {
  let source: EventSource | null = null;
  let stopped = false;
  let retries = 0;
  let startedAt = Date.now();
  let lastTick = Date.now();
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let syncTimer: ReturnType<typeof setTimeout> | undefined;
  let resumeTimer: ReturnType<typeof setTimeout> | undefined;

  function sync() {
    clearTimeout(syncTimer);
    syncTimer = setTimeout(() => {
      if (!stopped) void onSync();
    }, 90);
  }

  function connect() {
    if (stopped) return;
    clearTimeout(retryTimer);
    source?.close();
    source = null;
    onConnection(false);
    startedAt = Date.now();
    const current = new EventSource('/api/events');
    source = current;
    const isCurrent = () => !stopped && source === current;
    current.addEventListener('sync', () => {
      if (isCurrent()) sync();
    });
    current.onopen = () => {
      if (!isCurrent()) return;
      retries = 0;
      onConnection(true);
      sync();
    };
    current.onerror = () => {
      if (!isCurrent()) return;
      clearTimeout(syncTimer);
      onConnection(false);
      current.close();
      source = null;
      retryTimer = setTimeout(connect, Math.min(1_000 * 2 ** Math.min(retries++, 5), 30_000));
    };
  }

  function resume() {
    if (stopped || document.visibilityState === 'hidden') return;
    // Browsers often deliver focus, visibility and online together after waking.
    clearTimeout(resumeTimer);
    resumeTimer = setTimeout(() => {
      if (stopped) return;
      connect();
      sync();
    }, 0);
  }
  const visibility = () => {
    if (document.visibilityState === 'visible') resume();
  };
  const watchdog = setInterval(() => {
    const now = Date.now();
    const slept = now - lastTick > 30_000;
    const stalled = source?.readyState === EventSource.CONNECTING && now - startedAt >= 30_000;
    lastTick = now;
    if (slept || stalled || source?.readyState === EventSource.CLOSED) resume();
  }, 15_000);

  window.addEventListener('focus', resume);
  window.addEventListener('online', resume);
  window.addEventListener('pageshow', resume);
  document.addEventListener('visibilitychange', visibility);
  connect();

  return () => {
    stopped = true;
    clearInterval(watchdog);
    clearTimeout(retryTimer);
    clearTimeout(syncTimer);
    clearTimeout(resumeTimer);
    source?.close();
    source = null;
    window.removeEventListener('focus', resume);
    window.removeEventListener('online', resume);
    window.removeEventListener('pageshow', resume);
    document.removeEventListener('visibilitychange', visibility);
  };
}
