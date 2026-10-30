import { test,expect } from '@playwright/test';
test('play, fork, review, finalize, reload, and interchange SGF',async ({page,request}) => {
  const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto('/');await expect(page.getByRole('heading',{name:'Find your next move.'})).toBeVisible();
  await expect.poll(async()=> (await (await request.get('/api/state')).json()).engine.human.supported_ranks.length).toBe(29);
  await page.getByRole('button',{name:'New game'}).click();await page.getByLabel('Your color').selectOption('B');await page.getByRole('button',{name:'Start game',exact:true}).click();
  await expect(page.locator('.navbar-game').getByText('Your turn',{exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'Return to active continuation'})).toHaveCount(0);
  await page.getByRole('gridcell',{name:'D16, empty',exact:true}).click();
  await expect(page.getByRole('gridcell',{name:'Q4, white stone',exact:true})).toBeVisible();
  const gameId=page.url().split('/').pop()!;
  await page.getByRole('button',{name:'First move',exact:true}).click();
  await expect(page.getByRole('button',{name:'Return to active continuation'})).toBeVisible();
  await page.getByRole('button',{name:'Last move',exact:true}).click();
  await expect(page.locator('.history-controls > span')).toHaveText('2/2');
  await expect(page.getByRole('button',{name:'Return to active continuation'})).toHaveCount(0);
  await page.getByRole('button',{name:'First move',exact:true}).click();
  await page.getByRole('gridcell',{name:'E16, empty',exact:true}).click();
  await expect(page.getByRole('gridcell',{name:'Q4, white stone',exact:true})).toBeVisible();
  let view=await (await request.get(`/api/games/${gameId}`)).json();expect(Object.keys(view.game.nodes)).toHaveLength(5);const active=view.game.active_head;
  expect(await page.getByText('Position analysis',{exact:true}).count()).toBe(0);
  await page.getByRole('button',{name:'Review',exact:true}).click();await expect(page.getByRole('heading',{name:'Your mistakes'})).toBeVisible();
  await page.getByRole('button',{name:'First move',exact:true}).click();await page.getByRole('gridcell',{name:'F16, empty',exact:true}).click();
  view=await (await request.get(`/api/games/${gameId}`)).json();expect(view.game.active_head).toBe(active);expect(Object.keys(view.game.nodes)).toHaveLength(6);
  await expect(page.getByText('Black win probability',{exact:true})).toBeVisible();
  await page.screenshot({path:'test-results/review-desktop.png',fullPage:true});
  await page.getByRole('button',{name:'Play',exact:true}).click();await page.getByRole('button',{name:'Return to active continuation'}).click();await page.getByRole('button',{name:'Resign',exact:true}).click();
  await expect(page.getByRole('button',{name:'Play',exact:true})).toHaveCount(0);
  const audit=await (await request.get(`/api/games/${gameId}/audit`)).json();expect(audit.after).toBeLessThan(audit.before);
  await request.post(`/api/games/${gameId}/resign`,{data:{head:active}});expect(await (await request.get(`/api/games/${gameId}/audit`)).json()).toEqual(audit);
  await page.reload();await expect(page.getByRole('heading',{name:'Your mistakes'})).toBeVisible();
  const exported=await request.get(`/api/games/${gameId}/sgf`);expect(exported.headers()['content-disposition']).toContain(`filename=weiqi-trainer-${gameId}.sgf`);const sgf=await exported.text();expect(sgf).toContain('(;B[');
  const imported=await (await request.post('/api/games/import',{data:{sgf}})).json();const importedGame=await (await request.get(`/api/games/${imported.id}`)).json();expect(Object.keys(importedGame.game.nodes)).toHaveLength(6);
  await page.getByRole('link',{name:'Weiqi Trainer home',exact:true}).click();await expect(page.getByRole('heading',{name:/Review games/})).toBeVisible();
  await page.screenshot({path:'test-results/home-desktop.png',fullPage:true});
  await page.setViewportSize({width:390,height:844});await page.screenshot({path:'test-results/home-mobile.png',fullPage:true});
  expect(errors).toEqual([]);
});
test('two passes require explicit agreement and confirmation',async({page,request}) => {
  const created=await (await request.post('/api/games',{data:{mode:'manual',rank:-5,color:'B'}})).json();
  await page.goto(`/#game/${created.id}`);await expect(page.getByRole('button',{name:'Pass',exact:true})).toBeEnabled();
  await page.getByRole('button',{name:'Pass',exact:true}).click();await expect(page.getByRole('heading',{name:'Settle the board.'})).toBeVisible();
  await page.getByRole('button',{name:'Check score with opponent'}).click();await expect(page.getByRole('button',{name:'Confirm final result'})).toBeVisible();
  let v=await (await request.get(`/api/games/${created.id}`)).json();expect(v.game.result).toBeNull();
  await page.getByRole('button',{name:'Disagree / resume play'}).click();await expect(page.locator('.navbar-game').getByText('Your turn',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Pass',exact:true}).click();await expect(page.getByRole('heading',{name:'Settle the board.'})).toBeVisible();
  await page.getByRole('button',{name:'Check score with opponent'}).click();await page.getByRole('button',{name:'Confirm final result'}).click();
  await expect(page.getByRole('button',{name:'Play',exact:true})).toHaveCount(0);
  v=await (await request.get(`/api/games/${created.id}`)).json();expect(v.game.result).toBe('W+7.5');expect(v.game.resume_heads).toHaveLength(1);
});

test('reconnects after going offline and catches up without leaving the selected review position', async ({ page, request, context }) => {
  await expect.poll(async () => (await (await request.get('/api/state')).json()).engine.human.supported_ranks.length).toBe(29);
  const created = await (await request.post('/api/games', { data: { mode: 'manual', rank: -5, color: 'B' } })).json();
  let navigations = 0;
  page.on('framenavigated', frame => { if (frame === page.mainFrame()) navigations++; });
  await page.goto(`/#game/${created.id}`);
  await page.getByRole('button', { name: 'Review', exact: true }).click();
  await page.locator('.variation-panel > summary').click();
  await page.getByRole('button', { name: 'First move', exact: true }).click();
  const selectedPosition = page.getByRole('navigation', { name: 'Move variations' }).getByRole('button', { name: /Starting position/ });
  await expect(selectedPosition).toHaveAttribute('aria-current', 'step');
  await expect(page.getByText('Offline by design', { exact: true })).toBeVisible();
  await context.setOffline(true);
  try {
    // Going offline need not close an established stream; a wake/focus must replace it.
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await expect(page.getByText('Reconnecting…', { exact: true })).toBeVisible();
    const before = await (await request.get(`/api/games/${created.id}`)).json();
    await request.post(`/api/games/${created.id}/moves`, { data: { parent: before.game.root, point: 60, review: false, expected_head: before.game.active_head } });
    await expect.poll(async () => Object.keys((await (await request.get(`/api/games/${created.id}`)).json()).game.nodes).length).toBe(3);
    await context.setOffline(false);
    await page.evaluate(() => window.dispatchEvent(new Event('online')));
    await expect(page.getByText('Offline by design', { exact: true })).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Move variations' }).getByRole('button', { name: /D16/ })).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Move variations' }).getByRole('button', { name: /Q4/ })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Review', exact: true })).toHaveAttribute('aria-pressed', 'true');
    await expect(selectedPosition).toHaveAttribute('aria-current', 'step');
    await expect(page.getByRole('gridcell', { name: /D16, (suggestion.*empty|empty)/ })).toBeVisible();
    await expect(page.getByRole('alert')).toHaveCount(0);
    expect(navigations).toBe(1);
    // A successful catch-up clears transport errors, but must preserve domain errors.
    await page.route(`**/api/games/${created.id}/moves`, route => route.fulfill({ status: 409, contentType: 'application/json', body: JSON.stringify({ error: 'Illegal move fixture' }) }));
    await page.getByRole('gridcell', { name: 'T19, empty', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('Illegal move fixture');
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await expect(page.getByText('Offline by design', { exact: true })).toBeVisible();
    await expect(page.getByRole('alert')).toContainText('Illegal move fixture');
  } finally {
    await context.setOffline(false);
  }
});

test('keeps game libraries on Home and gives Settings a refreshable page with browser navigation', async ({ page, request }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Home', exact: true })).toHaveCount(0);
  await expect(page.locator('link[rel="icon"]')).toHaveAttribute('href', '/brand.svg');
  await expect(page.locator('.brand img')).toHaveAttribute('src', '/brand.svg');
  const favicon = await request.get('/brand.svg');
  expect(favicon.status()).toBe(200);
  expect(favicon.headers()['content-type']).toContain('image/svg+xml');
  const settingsLink = page.getByRole('link', { name: 'Settings', exact: true });
  await expect(settingsLink).toHaveCSS('color', 'rgb(63, 101, 73)');
  await settingsLink.hover();
  await expect(settingsLink).toHaveCSS('background-color', 'rgb(232, 237, 223)');
  await expect(settingsLink).toHaveCSS('color', 'rgb(49, 83, 59)');

  await expect(page.getByRole('link', { name: 'Weiqi Trainer home', exact: true })).toContainText('LOCAL GO PARTNER');
  await expect(page).toHaveTitle('Weiqi Trainer');
  await expect(page.locator('.brand-name')).toContainText('Weiqi Trainer');
  await expect(page.getByRole('heading', { level: 2, name: /^Human SL/ })).toBeVisible();
  const ongoing = page.getByRole('region', { name: 'Ongoing games' });
  const review = page.getByRole('region', { name: 'Review games' });
  await expect(ongoing).toBeVisible();
  await expect(review).toBeVisible();
  await expect(ongoing.getByRole('button', { name: /vs Human SL/ })).toHaveCount(1);
  await expect(review.getByRole('button', { name: /vs Human SL/ })).toHaveCount(2);
  await expect(page.getByLabel('KataGo executable')).toHaveCount(0);
  await page.screenshot({ path: 'test-results/design-home-desktop.png', fullPage: true });
  await page.getByRole('link', { name: 'Settings', exact: true }).click();
  await expect(page).toHaveURL(/#settings$/);
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await expect(page.getByLabel('KataGo executable')).toBeVisible();
  await expect(ongoing).toHaveCount(0);
  await page.screenshot({ path: 'test-results/design-settings-desktop.png', fullPage: true });
  await page.reload();
  await expect(page.getByLabel('KataGo executable')).toBeVisible();
  await page.getByRole('link', { name: 'Weiqi Trainer home', exact: true }).click();
  await expect(ongoing).toBeVisible();
  await page.goBack();
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await page.goForward();
  await expect(ongoing).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: 'test-results/design-home-mobile.png', fullPage: true });
  await page.getByRole('link', { name: 'Settings', exact: true }).click();
  await expect(page.getByLabel('KataGo executable')).toBeVisible();
  await page.screenshot({ path: 'test-results/design-settings-mobile.png', fullPage: true });
});

test('pins the board, fills the sidebar, and follows new moves without disturbing review navigation', async ({ page, request }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  const created = await (await request.post('/api/games', { data: { mode: 'manual', rank: -5, color: 'B' } })).json();
  const initial = await (await request.get(`/api/games/${created.id}`)).json();
  let head = initial.game.root;
  // Populate a real persisted tree without scheduling opponent replies for each setup move.
  for (let point = 0; point < 80; point++) {
    const response = await request.post(`/api/games/${created.id}/moves`, { data: { parent: head, point, review: true } });
    expect(response.ok()).toBeTruthy();
    head = (await response.json()).node;
  }
  const activated = await request.post(`/api/games/${created.id}/continue`, { data: { node: head, expected_head: initial.game.root } });
  expect(activated.ok()).toBeTruthy();
  await page.goto(`/#game/${created.id}`);
  const history = page.getByRole('navigation', { name: 'Move variations' });
  const board = page.locator('.board-frame');
  const brand = page.getByRole('link', { name: 'Weiqi Trainer home', exact: true });
  const sidebar = page.locator('.sidebar-content');
  const assertBoardFits = async () => {
    await expect(board).toBeVisible();
    const bounds = await board.boundingBox();
    const header = await page.locator('.app-header').boundingBox();
    const footer = await page.locator('.engine-bar').boundingBox();
    expect(bounds!.width).toBeGreaterThan(180);
    const controls = await page.locator('.history-controls').boundingBox();
    expect(Math.abs(controls!.x + controls!.width / 2 - bounds!.x - bounds!.width / 2)).toBeLessThan(1);
    expect(bounds!.y).toBeGreaterThanOrEqual(header!.y + header!.height);
    expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(footer!.y + 1);
    const dimensions = await page.evaluate(() => ({ height: document.documentElement.scrollHeight, width: document.documentElement.scrollWidth, viewportHeight: innerHeight, viewportWidth: innerWidth, scroll: scrollY }));
    expect(dimensions.height).toBeLessThanOrEqual(dimensions.viewportHeight);
    expect(dimensions.width).toBeLessThanOrEqual(dimensions.viewportWidth);
    expect(dimensions.scroll).toBe(0);
    if (dimensions.viewportWidth > 760) {
      if (await page.locator('.game-metadata').count()) {
      const navigation = await page.locator('.board-navigation').boundingBox();
      const metadata = await page.locator('.game-metadata').boundingBox();
      expect(Math.abs(navigation!.y + navigation!.height - metadata!.y - metadata!.height)).toBeLessThan(1);
      const move = await page.locator('.history-controls > span').boundingBox();
      const captures = await page.locator('.game-metadata > span').boundingBox();
      expect(Math.abs(move!.y + move!.height / 2 - captures!.y - captures!.height / 2)).toBeLessThan(2);
      }
    }

  };
  const selectedIsInHistoryViewport = () => history.evaluate(element => {
    const selected = element.querySelector('[aria-current="step"]')!.getBoundingClientRect();
    const bounds = element.getBoundingClientRect();
    return selected.top >= bounds.top - 1 && selected.bottom <= bounds.bottom + 1;
  });
  await expect(page.locator('.app-header').getByRole('heading', { name: 'Human SL 5k Black stone Your turn' })).toBeVisible();
  await expect(page.locator('.app-header').getByRole('button', { name: 'Review', exact: true })).toBeVisible();
  await expect(page.getByText('AUTO OPPONENT', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Arrow keys on the board to move · Enter to place a stone', { exact: true })).toHaveCount(0);
  await assertBoardFits();
  expect(await history.evaluate(element => element.clientHeight)).toBeGreaterThan(400);
  await expect.poll(selectedIsInHistoryViewport).toBe(true);

  await expect(page.locator('.history-controls > span')).toHaveText('80/80');
  await expect(page.locator('.board-topline')).toHaveCount(0);
  await expect(page.getByText('Take your time.', { exact: true })).toHaveCount(0);
  await expect(page.getByText('No clock. Find a move that feels right.', { exact: true })).toHaveCount(0);
  await expect(page.locator('.board-hint')).toHaveCount(0);
  const geometry = await page.locator('svg.board').evaluate(svg => {
    const labels = Array.from(svg.querySelectorAll<SVGTextElement>('.coords')).map(label => label.getBBox());
    const stones = Array.from(svg.querySelectorAll<SVGCircleElement>('circle[fill="url(#black)"], circle[fill="url(#white)"]')).map(stone => stone.getBBox());
    const clear = labels.every(label => stones.every(stone => Math.max(
      stone.x - label.x - label.width, label.x - stone.x - stone.width,
      stone.y - label.y - label.height, label.y - stone.y - stone.height,
    ) >= 6));
    const stone = svg.querySelector<SVGCircleElement>('circle[fill="url(#white)"]')!.getBoundingClientRect();
    const target = document.getElementById('intersection-1')!.getBoundingClientRect();
    return { clear, targetOffset: Math.hypot(stone.x + stone.width / 2 - target.x - target.width / 2, stone.y + stone.height / 2 - target.y - target.height / 2) };
  });
  expect(geometry.clear).toBe(true);
  expect(geometry.targetOffset).toBeLessThan(0.5);
  await page.screenshot({ path: 'test-results/focused-play-desktop.png', fullPage: true });

  const playFromActive = async (point: number, expectedMoves: number) => {
    const current = await (await request.get(`/api/games/${created.id}`)).json();
    const response = await request.post(`/api/games/${created.id}/moves`, { data: { parent: current.game.active_head, point, review: false, expected_head: current.game.active_head } });
    expect(response.ok()).toBeTruthy();
    await expect.poll(async () => Object.keys((await (await request.get(`/api/games/${created.id}`)).json()).game.nodes).length).toBe(expectedMoves + 1);
    // Setup queues analysis for 80 positions; allow the debounced SSE refresh to settle on a busy host.
    await expect(page.locator('.variation-panel')).toContainText(`${expectedMoves} moves`, { timeout: 15000 });
  };
  await playFromActive(80, 82);
  await expect(history.locator('[aria-current="step"]')).toContainText('active');
  await expect.poll(selectedIsInHistoryViewport).toBe(true);
  await expect.poll(() => history.evaluate(element => element.scrollHeight - element.scrollTop - element.clientHeight)).toBeLessThan(5);

  for (const size of [{ width: 1360, height: 650 }, { width: 390, height: 844 }, { width: 844, height: 390 }]) {
    await page.setViewportSize(size);
    await assertBoardFits();
    await expect(page.locator('.app-header').getByRole('button', { name: 'Review', exact: true })).toBeVisible();
    if (size.width === 390) await page.screenshot({ path: 'test-results/focused-play-mobile.png', fullPage: true });
  }
  await page.setViewportSize({ width: 1360, height: 1000 });
  await page.getByRole('button', { name: 'Review', exact: true }).click();
  await page.locator('.variation-panel > summary').click();
  await page.getByRole('button', { name: 'First move', exact: true }).click();
  await expect(history.locator('[aria-current="step"]')).toContainText('Starting position');
  await expect(page.locator('.history-controls > span')).toHaveText('0/82');
  await expect(page.locator('.board-hint')).toHaveCount(0);
  await expect.poll(() => history.evaluate(element => element.scrollTop)).toBeLessThan(4);
  await playFromActive(81, 84);
  await expect(page.locator('.history-controls > span')).toHaveText('0/84');
  await expect(history.locator('[aria-current="step"]')).toContainText('Starting position');
  expect(await history.evaluate(element => element.scrollTop)).toBeLessThan(4);
  await assertBoardFits();
  await page.setViewportSize({ width: 1360, height: 650 });
  const before = await board.boundingBox();
  const overviewBefore = await page.locator('.review-overview').boundingBox();
  await sidebar.hover();
  await page.mouse.wheel(0, 1200);
  await expect.poll(() => sidebar.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
  expect(await board.boundingBox()).toEqual(before);
  const overviewAfter = await page.locator('.review-overview').boundingBox();
  expect(overviewAfter!.y).toBe(overviewBefore!.y);
  await assertBoardFits();
  await page.screenshot({ path: 'test-results/focused-review-desktop.png', fullPage: true });
  // Opening diagnostics overlays the workspace and leaves the board in place.
  await page.locator('.engine-bar summary').click();
  expect(await board.boundingBox()).toEqual(before);
  await page.locator('.engine-bar summary').click();
  await brand.hover();
  await expect(brand).toHaveCSS('cursor', 'pointer');
  await expect(brand).toHaveCSS('background-color', 'rgb(232, 237, 223)');
  expect(await brand.evaluate(element => {
    const box = element.getBoundingClientRect();
    return document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2)?.closest('.brand') === element;
  })).toBe(true);
  await page.setViewportSize({ width: 390, height: 844 });
  await assertBoardFits();
  await page.screenshot({ path: 'test-results/focused-review-mobile.png', fullPage: true });
  await brand.click();
  await expect(page.getByRole('heading', { name: 'Find your next move.' })).toBeVisible();
  await page.goBack();
  await expect(board).toBeVisible();
  expect(errors).toEqual([]);
});

test('Home board tiles load cached SVGs and follow the active played position', async ({ page, request }) => {
  const created = await (await request.post('/api/games', { data: { mode: 'manual', rank: -5, color: 'B' } })).json();
  await page.goto('/');
  const tile = page.locator(`[data-game-id="${created.id}"]`);
  const image = tile.locator('img.game-preview');
  await tile.scrollIntoViewIfNeeded();
  await expect.poll(() => image.evaluate((element: HTMLImageElement) => element.complete && element.naturalWidth > 0)).toBe(true);
  const firstUrl = await image.getAttribute('src');
  await tile.click();
  await page.getByRole('gridcell', { name: /D16, (suggestion.*empty|empty)/ }).click();
  await expect(page.getByRole('gridcell', { name: 'Q4, white stone', exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Weiqi Trainer home', exact: true }).click();
  await expect(image).not.toHaveAttribute('src', firstUrl!);
  await tile.scrollIntoViewIfNeeded();
  await expect.poll(() => image.evaluate((element: HTMLImageElement) => element.complete && element.naturalWidth > 0)).toBe(true);
  const url = (await image.getAttribute('src'))!;
  const response = await request.get(url);
  expect(response.headers()['content-type']).toBe('image/svg+xml');
  expect(response.headers()['cache-control']).toContain('immutable');
  const svg = await response.text();
  expect(svg).toContain('<use href="#b" x="80" y="80"');
  expect(svg).toContain('<use href="#w" x="320" y="320"');
  const conditional = await request.get(url, { headers: { 'If-None-Match': response.headers().etag } });
  expect(conditional.status()).toBe(304);
  expect(await conditional.body()).toHaveLength(0);
  const current = await (await request.get(`/api/games/${created.id}`)).json();
  await request.post(`/api/games/${created.id}/moves`, { data: { parent: current.game.root, point: 61, review: true } });
  await page.reload();
  await tile.scrollIntoViewIfNeeded();
  await expect(image).toHaveAttribute('src', url);
  await expect.poll(() => image.evaluate((element: HTMLImageElement) => element.complete && element.naturalWidth > 0)).toBe(true);
  // Revisiting Home reuses the immutable image from the browser cache.
  await tile.click();
  await page.getByRole('link', { name: 'Weiqi Trainer home', exact: true }).click();
  await tile.scrollIntoViewIfNeeded();
  await expect.poll(() => image.evaluate((element: HTMLImageElement) => element.complete && element.naturalWidth > 0)).toBe(true);
  const transferred = await page.evaluate(path => {
    const entries = performance.getEntriesByName(new URL(path, location.href).href) as PerformanceResourceTiming[];
    return entries.at(-1)?.transferSize;
  }, url);
  expect(transferred).toBe(0);

  for (const size of [{ width: 1360, height: 1000 }, { width: 1024, height: 768 }]) {
    await page.setViewportSize(size);
    for (const title of ['Ongoing games', 'Review games']) {
      const tiles = page.getByRole('region', { name: title }).locator('.game-row');
      const images = tiles.locator('.game-preview');
      const positions = await images.evaluateAll(elements => elements.slice(0, 3).map(element => {
        const box = element.getBoundingClientRect();
        return { x: box.x, y: box.y, width: box.width };
      }));
      expect(positions).toHaveLength(3);
      expect(new Set(positions.map(position => position.y)).size).toBe(1);
      expect(new Set(positions.map(position => position.x)).size).toBe(3);
      for (let index = 0; index < 3; index++) {
        await tiles.nth(index).hover();
        await expect(tiles.nth(index)).toHaveCSS('background-color', 'rgb(232, 237, 223)');
        await expect(images.nth(index)).toHaveCSS('box-shadow', 'rgb(63, 101, 73) 0px 0px 0px 2px');
      }
    }
  }
  await page.setViewportSize({ width: 1360, height: 1000 });
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({ path: 'test-results/board-gallery-desktop.png', fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(390);
  await page.screenshot({ path: 'test-results/board-gallery-mobile.png', fullPage: true });
});

test('reviews human mistakes before each move and explores corrections without changing the result', async ({ page, request }) => {
  let moveWrites = 0;
  let preferPlayed = false;
  page.on('request', request => {
    if (request.method() === 'POST' && /\/api\/games\/[^/]+\/(moves|variations)$/.test(request.url())) moveWrites++;
  });
  await expect.poll(async () => (await (await request.get('/api/state')).json()).engine.human.supported_ranks.length).toBe(29);
  const created = await (await request.post('/api/games', { data: { mode: 'manual', rank: -5, color: 'B' } })).json();
  const initial = await (await request.get(`/api/games/${created.id}`)).json();
  const ids = [initial.game.root];
  for (const point of [60, 300, 72, 288, 62, 342]) {
    const response = await request.post(`/api/games/${created.id}/moves`, { data: { parent: ids.at(-1), point, review: true } });
    expect(response.ok()).toBe(true);
    ids.push((await response.json()).node);
  }
  await request.post(`/api/games/${created.id}/continue`, { data: { node: ids[6], expected_head: ids[0] } });
  await request.post(`/api/games/${created.id}/resign`, { data: { head: ids[6] } });
  const audit = await (await request.get(`/api/games/${created.id}/audit`)).json();
  // Supply deterministic strong evaluations while retaining the real server's boards, rules,
  // node persistence, analysis queue, final result and rating transaction.
  await page.route(new RegExp('/api/games/' + created.id + '(?:\\?.*)?$'), async route => {
    const response = await route.fetch();
    const data = await response.json();
    const template = Object.values(data.evaluations)[0] as Record<string, unknown> | undefined;
    const evaluation = (score: number, winrate: number) => ({ ...template, score_black: score, winrate_black: winrate, visits: 256, candidates: [], ownership: [], engine: 'KataGo fixture', model: 'strong fixture', config: 'fixture', created_at: '2026-10-29T08:00:00Z' });
    for (const [index, id] of ids.entries()) data.evaluations[id] = evaluation([6, -2, 1, -3, 2, 1, 0][index], [.8, .4, .6, .35, .6, .55, .5][index]);
    data.evaluations[ids[0]].candidates = [
      { point: 61, visits: 256, score_black: 6, winrate_black: .8, order: 0, pv: [61, 300, 62] },
      { point: 63, visits: 256, score_black: 5, winrate_black: .78, order: 1, pv: [63, 300] },
      { point: 60, visits: 256, score_black: -2, winrate_black: .4, order: 2, pv: [60, 300] },
    ];
    data.evaluations[ids[2]].candidates = [
      { point: 73, visits: 256, score_black: 1, winrate_black: .6, order: 0, pv: [73, 288, 62] },
      { point: 74, visits: 256, score_black: .5, winrate_black: .55, order: 1, pv: [74, 288] },
    ];
    data.evaluations[ids[4]].candidates = [{ point: 63, visits: 256, score_black: 2, winrate_black: .6, order: 0, pv: [63, 342] }];
    for (const [index, label, loss] of [[1, 'Blunder', 8], [2, 'Blunder', 20], [3, 'Mistake', 4], [5, 'Inaccuracy', 1]] as const) {
      const row = data.reviews.find((row: { node: string }) => row.node === ids[index]);
      Object.assign(row, { label, score_loss: loss, confidence: '256 visits' });
    }
    const branchLabels: Record<number, string> = { 61: 'AI move', 300: 'Inaccuracy', 62: 'Good move', 18: 'Mistake' };
    data.candidate_reviews ??= {};
    data.candidate_reviews[ids[0]] = [
      { point: 61, category: 'Good move', provisional: false, confidence: 256 },
      { point: 63, category: 'Good move', provisional: false, confidence: 256 },
      { point: 60, category: 'Blunder', provisional: false, confidence: 256 },
    ];
    if (preferPlayed) {
      data.evaluations[ids[0]].candidates[0].order = 1;
      data.evaluations[ids[0]].candidates[0].visits = 8;
      data.evaluations[ids[0]].candidates[0].score_black = -10;
      data.evaluations[ids[0]].candidates[0].winrate_black = .1;
      data.evaluations[ids[0]].candidates[1].order = 2;
      data.evaluations[ids[0]].candidates[1].visits = 3;
      data.evaluations[ids[0]].candidates[1].score_black = -7.4;
      data.evaluations[ids[0]].candidates[1].winrate_black = .35;
      data.evaluations[ids[0]].candidates[2].order = 0;
      data.evaluations[ids[0]].score_black = -2;
      data.evaluations[ids[0]].winrate_black = .4;
      data.reviews.find((row: { node: string }) => row.node === ids[1]).label = 'Good move';
      branchLabels[61] = 'Blunder';
      data.candidate_reviews[ids[0]] = [
        { point: 60, category: 'Good move', provisional: false, confidence: 256 },
        { point: 61, category: 'Blunder', provisional: true, confidence: 8 },
        { point: 63, category: 'Mistake', provisional: true, confidence: 3 },
      ];
    }
    for (const row of data.reviews) {
      const node = data.game.nodes[row.node];
      if (!ids.includes(node.id) && node.point !== null && branchLabels[node.point])
        Object.assign(row, { label: branchLabels[node.point], confidence: '256 visits' });
      if (preferPlayed && node.parent === ids[0] && node.point === 61 && data.evaluations[node.id]) {
        Object.assign(data.evaluations[node.id], { score_black: -10, winrate_black: .1 });
        Object.assign(row, { score_loss: 8, winrate_loss: .3 });
      }
    }
    await route.fulfill({ json: data });
  });
  await page.goto(`/#game/${created.id}`);
  const mistakes = page.getByRole('region', { name: 'Your mistakes' });
  await expect(mistakes.locator('.review-list button')).toHaveCount(2);
  await expect(mistakes.locator('.review-list button').first()).toContainText('1. D16');
  await expect(mistakes.locator('.review-list button').last()).toContainText('3. Q16');
  await expect(page.locator('.chart-navigation')).toHaveCount(0);
  await expect(page.getByRole('link', { name: 'Export SGF' })).toHaveCount(0);
  await expect(page.locator('.game-metadata')).toHaveCount(0);
  await mistakes.getByRole('button', { name: /1\. D16/ }).click();
  const selected = page.getByRole('region', { name: 'Selected move' });
  await expect(selected).toContainText('Played D16');
  await expect(selected).toContainText('80.0% → 40.0%');
  await expect(page.locator('circle.actual-move')).toHaveAttribute('fill', '#b8363b');
  await expect(page.getByRole('gridcell', { name: 'D16, played move 1, Blunder' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Return to played game', exact: true })).toBeVisible();
  await expect(page.locator('svg.board circle[fill="url(#black)"]')).toHaveCount(1);
  await expect(page.getByText('Cancel analysis', { exact: true })).not.toBeVisible();
  await expect(page.getByText('G16', { exact: true })).not.toBeVisible();
  await page.screenshot({ path: 'test-results/review-selected-desktop.png', fullPage: true });
  await page.getByRole('gridcell', { name: 'D16, played move 1, Blunder' }).click();
  await expect(page.locator('.history-controls > span')).toHaveText('2/6');
  await expect(selected).toContainText('Played Q4');
  await expect(page.getByRole('region', { name: 'Explored position' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Next move', exact: true }).click();
  await expect(page.locator('.history-controls > span')).toHaveText('3/6');
  expect(moveWrites).toBe(0);
  expect(Object.keys((await (await request.get(`/api/games/${created.id}`)).json()).game.nodes)).toHaveLength(7);
  await mistakes.getByRole('button', { name: /1\. D16/ }).click();
  await page.getByRole('gridcell', { name: 'E16, suggestion A, AI recommendation, empty' }).click();
  await expect(page.getByRole('region', { name: 'Explored position' })).toContainText('Original');
  await expect(page.getByRole('gridcell', { name: 'E16, played move 1, AI move' })).toBeVisible();
  await expect(page.locator('circle.actual-move')).toHaveAttribute('fill', '#3069ad');
  await expect(page.locator('circle.actual-move + text')).toHaveText('1');
  await expect(page.getByRole('button', { name: 'Return to played game', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Last move', exact: true }).click();
  await expect(page.locator('.history-controls > span')).toHaveText('3/6');
  await expect(page.getByRole('gridcell', { name: 'F16, played move 3, Good move' })).toBeVisible();
  await expect(page.locator('circle.actual-move')).toHaveAttribute('fill', '#287647');
  await expect(page.locator('circle.actual-move + text')).toHaveText('3');
  await expect(page.getByRole('button', { name: 'Return to played game', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Previous move', exact: true }).click();
  await page.getByRole('button', { name: 'Previous move', exact: true }).click();
  await page.getByRole('button', { name: 'Previous move', exact: true }).click();
  await expect(page.locator('.history-controls > span')).toHaveText('0/6');
  await expect(page.getByRole('button', { name: 'Return to played game', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Next move', exact: true }).click();
  await expect(page.getByRole('gridcell', { name: 'E16, played move 1, AI move' })).toBeVisible();
  await page.getByRole('button', { name: 'Next move', exact: true }).click();
  await expect(page.getByRole('gridcell', { name: 'Q4, played move 2, Inaccuracy' })).toBeVisible();
  await expect(page.locator('circle.actual-move')).toHaveAttribute('fill', '#8b6500');
  await page.getByRole('gridcell', { name: 'T19, empty' }).click();
  await expect(page.getByRole('gridcell', { name: 'T19, played move 3, Mistake' })).toBeVisible();
  await expect(page.locator('circle.actual-move')).toHaveAttribute('fill', '#a64c10');
  await page.getByRole('button', { name: 'Previous move', exact: true }).click();
  await page.getByRole('button', { name: 'Next move', exact: true }).click();
  await expect(page.getByRole('gridcell', { name: 'T19, played move 3, Mistake' })).toBeVisible();
  await page.getByRole('button', { name: 'Return to played game', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Return to played game', exact: true })).toBeVisible();
  await expect(page.getByRole('gridcell', { name: 'D16, played move 1, Blunder' })).toBeVisible();
  await page.getByRole('button', { name: 'Last move', exact: true }).click();
  await expect(page.locator('.history-controls > span')).toHaveText('6/6');
  await expect(page.getByRole('button', { name: 'Return to played game', exact: true })).toHaveCount(0);
  await mistakes.getByRole('button', { name: /1\. D16/ }).click();
  const writesBeforeReuse = moveWrites;
  await page.getByRole('button', { name: /Try E16/ }).click();
  await expect(page.getByRole('region', { name: 'Explored position' })).toBeVisible();
  await page.getByRole('button', { name: 'Last move', exact: true }).click();
  await expect(page.getByRole('gridcell', { name: 'F16, played move 3, Good move' })).toBeVisible();
  expect(moveWrites).toBe(writesBeforeReuse);
  await page.getByRole('button', { name: 'Return to played game', exact: true }).click();
  const persisted = await (await request.get(`/api/games/${created.id}`)).json();
  expect(persisted.game.active_head).toBe(ids[6]);
  expect(persisted.game.result).toBe('W+R');
  expect(Object.keys(persisted.game.nodes)).toHaveLength(11);
  expect(await (await request.get(`/api/games/${created.id}/audit`)).json()).toEqual(audit);
  await mistakes.getByLabel('Include inaccuracies').check();
  await expect(mistakes.locator('.review-list button')).toHaveCount(3);
  await page.getByLabel('Sort mistakes').selectOption('severity');
  await expect(mistakes.locator('.review-list button').first()).toContainText('Blunder');
  const chart = page.getByRole('slider', { name: 'Review game timeline' });
  const bounds = (await chart.boundingBox())!;
  await page.mouse.move(bounds.x + bounds.width * 34 / 300, bounds.y + bounds.height / 2);
  await page.mouse.down();
  await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await expect(chart).toHaveAttribute('aria-valuenow', '3');
  await expect(page.locator('.history-controls > span')).toHaveText('1/6');
  await page.mouse.up();
  await expect(page.locator('.history-controls > span')).toHaveText('3/6');
  await page.setViewportSize({ width: 1360, height: 650 });
  const side = page.locator('.sidebar-content');
  const overview = page.getByRole('region', { name: 'Game overview' });
  const top = (await overview.boundingBox())!.y;
  await page.locator('.analysis-details summary').click();
  await expect(page.getByText('Cancel analysis', { exact: true })).toBeVisible();
  await side.hover(); await page.mouse.wheel(0, 1000);
  await expect.poll(() => side.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
  expect((await overview.boundingBox())!.y).toBe(top);
  await page.screenshot({ path: 'test-results/review-learning-desktop.png', fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await side.evaluate(element => element.scrollTop = 0);
  await mistakes.getByRole('button', { name: /1\. D16/ }).click();
  await expect(page.getByRole('gridcell', { name: 'D16, played move 1, Blunder' })).toBeVisible();
  await side.evaluate(element => element.scrollTop = 0);
  await page.screenshot({ path: 'test-results/review-learning-mobile.png', fullPage: true });
  // The best move is already played. Its exclusion must not turn the next candidate into AI.
  preferPlayed = true;
  await page.setViewportSize({ width: 1360, height: 1000 });
  await page.getByRole('button', { name: 'First move', exact: true }).click();
  await expect(page.locator('.history-controls > span')).toHaveText('0/6');
  await page.getByRole('button', { name: 'Next move', exact: true }).click();
  await expect(page.locator('.history-controls > span')).toHaveText('1/6');
  await expect(page.getByText('Best move played', { exact: true })).toBeVisible();
  const knownBad = page.getByRole('gridcell', { name: 'E16, suggestion A, Blunder, empty' });
  await expect(knownBad).toHaveAttribute('title', 'Engine #2 · Blunder · 8 candidate visits');
  const point = await knownBad.evaluate(button => Number(button.id.replace('intersection-', '')));
  const marker = page.locator(`svg.board circle[cx="${48 + point % 19 * 32}"][cy="${48 + Math.floor(point / 19) * 32}"][r="12"]`);
  await expect(marker).toHaveAttribute('stroke', '#b8363b');
  const estimated = page.getByRole('gridcell', { name: 'G16, suggestion B, Estimated Mistake, empty' });
  await expect(estimated).toHaveAttribute('title', 'Engine #3 · Estimated Mistake · 3 candidate visits');
  await expect(page.locator('svg.board circle[cx="240"][cy="144"][r="12"]')).toHaveAttribute('stroke', '#a64c10');
  await page.locator('.other-suggestions summary').click();
  await expect(page.locator('.other-suggestions .candidate-letter').first()).toHaveCSS('color', 'rgb(184, 54, 59)');
  const writesBeforeBad = moveWrites;
  await knownBad.click();
  await expect(page.locator('circle.actual-move')).toHaveAttribute('fill', '#b8363b');
  expect(moveWrites).toBe(writesBeforeBad);
  await page.screenshot({ path: 'test-results/review-best-already-played.png', fullPage: true });
  await page.unrouteAll({ behavior: 'wait' });
});
