/** Benchmark the production UI against read-only API snapshots; no live game mutations. */
import { chromium } from '@playwright/test';
import { readFile, writeFile } from 'node:fs/promises';

const [statePath, gamePath, outputPath = '/tmp/weiqi-navigation-benchmark.json'] =
  process.argv.slice(2);
if (!statePath || !gamePath)
  throw new Error(
    'Usage: node tests/benchmarks/navigation.mjs STATE_JSON GAME_VIEW_JSON [OUTPUT_JSON]',
  );
const appState = JSON.parse(await readFile(statePath, 'utf8'));
const view = JSON.parse(await readFile(gamePath, 'utf8'));
const liveReads = process.env.WEIQI_BENCH_LIVE_READS === '1';
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1360, height: 1000 } });
  await page.addInitScript(() => {
    // Isolate navigation/rendering from engine notifications and automatic reconnects.
    window.EventSource = class extends EventTarget {
      static CONNECTING = 0;
      static OPEN = 1;
      static CLOSED = 2;
      readyState = 1;
      constructor() {
        super();
        window.benchmarkEventSource = this;
      }
      close() {
        this.readyState = 2;
      }
    };
  });
  await page.route('**/api/**', async (route) => {
    const pathname = new URL(route.request().url()).pathname;
    if (
      liveReads &&
      route.request().method() === 'GET' &&
      (pathname === '/api/state' ||
        pathname === `/api/games/${view.game.id}` ||
        pathname.endsWith('/preview.svg'))
    ) {
      await route.continue();
      return;
    }
    if (pathname.endsWith('/preview.svg')) {
      await route.fulfill({
        contentType: 'image/svg+xml',
        body: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 400"><rect width="400" height="400" fill="#e6c991"/></svg>',
      });
      return;
    }
    const body =
      pathname === '/api/state' ? appState : pathname === `/api/games/${view.game.id}` ? view : {};
    await route.fulfill({ json: body });
  });
  await page.goto(process.env.WEIQI_BENCH_URL ?? 'http://127.0.0.1:3000');
  await page.getByRole('heading', { name: 'Find your next move.' }).waitFor();
  const samples = [];
  const client = await page.context().newCDPSession(page);
  await client.send('Profiler.enable');
  await client.send('Profiler.start');
  for (let i = 0; i < 3; i++) {
    const open = await page.evaluate(async (gameId) => {
      const button = Array.from(document.querySelectorAll('.game-row')).find(
        (element) =>
          element.textContent.includes('vs Human SL') &&
          element.closest('[aria-label="Review games"]'),
      );
      if (!button) throw new Error(`Review card missing for ${gameId}`);
      const start = performance.now();
      button.click();
      while (!document.querySelector('.review-panel')) await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
      return performance.now() - start;
    }, view.game.id);
    const home = await page.evaluate(async () => {
      const start = performance.now();
      document.querySelector('.brand').click();
      while (!document.querySelector('.home-intro')) await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
      return performance.now() - start;
    });
    samples.push({ review_ms: Math.round(open), home_ms: Math.round(home) });
  }
  // A click arriving during an analysis refresh must not wait for a long render.
  await page.getByRole('region', { name: 'Review games' }).locator('.game-row').first().click();
  await page.locator('.review-panel').waitFor();
  const refreshHome = await page.evaluate(async () => {
    const start = performance.now();
    window.benchmarkEventSource.dispatchEvent(new MessageEvent('sync'));
    await new Promise((resolve) => setTimeout(resolve, 150));
    document.querySelector('.brand').click();
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    return performance.now() - start - 150;
  });
  const { profile } = await client.send('Profiler.stop');
  await writeFile(outputPath.replace(/\.json$/, '-profile.json'), JSON.stringify(profile));
  const median = (key) => samples.map((sample) => sample[key]).sort((a, b) => a - b)[1];
  const result = {
    transport: liveReads ? 'local HTTP (GET only)' : 'API snapshots',
    nodes: Object.keys(view.game.nodes).length,
    evaluations: Object.keys(view.evaluations).length,
    samples,
    median_review_ms: median('review_ms'),
    median_home_ms: median('home_ms'),
    home_during_refresh_ms: Math.round(refreshHome),
  };
  await writeFile(outputPath, JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify(result, null, 2));
} finally {
  await browser.close();
}
