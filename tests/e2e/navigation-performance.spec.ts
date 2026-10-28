import { test, expect } from '@playwright/test';
import { fixtureGame, fixtureState, fixtureView } from '../../frontend/fixtures';
import type { Node, Review } from '../../frontend/lib';

test('long reviews and analysis refreshes keep navigation responsive', async ({ page }) => {
  // Rendering benchmark only: synthetic server snapshots, no engine or saved-game writes.
  const nodes: Record<string, Node> = { root: { ...fixtureGame.nodes.root } };
  const reviews: Review[] = [];
  const evaluations = { root: fixtureView.evaluations.b };
  let parent = 'root';
  for (let move = 1; move <= 400; move++) {
    const id = `move-${move}`;
    nodes[id] = {
      ...fixtureGame.nodes.a,
      id,
      parent,
      color: move % 2 ? 'B' : 'W',
      point: move % 361,
    };
    reviews.push({ ...fixtureView.reviews[0], node: id });
    Object.assign(evaluations, { [id]: fixtureView.evaluations.b });
    parent = id;
  }
  const game = {
    ...fixtureGame,
    nodes,
    root: 'root',
    active_head: parent,
    status: 'finished',
    result: 'W+R',
  };
  const view = { ...fixtureView, game, selected: parent, reviews, evaluations };
  await page.addInitScript(() => {
    Object.defineProperty(window, 'EventSource', {
      value: class extends EventTarget {
        static CONNECTING = 0;
        static OPEN = 1;
        static CLOSED = 2;
        readyState = 1;
        constructor() {
          super();
          window.addEventListener('benchmark-sync', () =>
            this.dispatchEvent(new MessageEvent('sync')),
          );
        }
        close() {
          this.readyState = 2;
        }
      },
    });
  });
  let gameRequests = 0;
  await page.route('**/api/**', async (route) => {
    const pathname = new URL(route.request().url()).pathname;
    if (pathname.endsWith('/preview.svg')) {
      await route.fulfill({
        contentType: 'image/svg+xml',
        body: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 400"><rect width="400" height="400" fill="#e6c991"/></svg>',
      });
      return;
    }
    if (pathname === `/api/games/${game.id}`) gameRequests++;
    await route.fulfill({
      json:
        pathname === '/api/state'
          ? { ...fixtureState, games: [{ ...game, nodes: {} }] }
          : pathname === `/api/games/${game.id}`
            ? view
            : {},
    });
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Find your next move.' })).toBeVisible();
  const openMs = await page.evaluate(async () => {
    const start = performance.now();
    (document.querySelector('.game-row') as HTMLButtonElement).click();
    while (!document.querySelector('.review-panel')) await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    return performance.now() - start;
  });
  expect(openMs).toBeLessThan(1000);
  await expect(page.locator('.review-list button')).toHaveCount(400);
  await page.getByLabel('Sort reviewed moves').selectOption('move');
  await expect(page.locator('.review-list button').first()).toContainText('1. B');
  await expect(page.locator('.review-list button').last()).toContainText('400. W');

  const requestsBeforeRefresh = gameRequests;
  const homeMs = await page.evaluate(async () => {
    const start = performance.now();
    window.dispatchEvent(new Event('benchmark-sync'));
    // Simulate an input arriving while the 90ms-debounced analysis refresh renders.
    await new Promise((resolve) => setTimeout(resolve, 150));
    (document.querySelector('.brand') as HTMLAnchorElement).click();
    while (!document.querySelector('.home-intro')) await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    return performance.now() - start - 150;
  });
  expect(homeMs).toBeLessThan(1000);
  expect(gameRequests).toBeGreaterThan(requestsBeforeRefresh);
  await expect(page.getByRole('heading', { name: 'Find your next move.' })).toBeVisible();
});
