import { test,expect } from '@playwright/test';
test('play, fork, review, finalize, reload, and interchange SGF',async ({page,request}) => {
  const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto('/');await expect(page.getByRole('heading',{name:'Find your next move.'})).toBeVisible();
  await expect.poll(async()=> (await (await request.get('/api/state')).json()).engine.human.supported_ranks.length).toBe(29);
  await page.getByRole('button',{name:'New game'}).click();await page.getByLabel('Your color').selectOption('B');await page.getByRole('button',{name:'Start game',exact:true}).click();
  await expect(page.getByRole('heading',{name:'Take your time.'})).toBeVisible();
  await page.getByRole('gridcell',{name:'D16, empty',exact:true}).click();
  await expect(page.getByRole('gridcell',{name:'Q4, white stone',exact:true})).toBeVisible();
  const gameId=page.url().split('/').pop()!;
  await page.getByRole('button',{name:'First move',exact:true}).click();
  await page.getByRole('gridcell',{name:'E16, empty',exact:true}).click();
  await expect(page.getByRole('gridcell',{name:'Q4, white stone',exact:true})).toBeVisible();
  let view=await (await request.get(`/api/games/${gameId}`)).json();expect(Object.keys(view.game.nodes)).toHaveLength(5);const active=view.game.active_head;
  expect(await page.getByText('Position analysis',{exact:true}).count()).toBe(0);
  await page.getByRole('button',{name:'Review',exact:true}).click();await expect(page.getByRole('heading',{name:'Position analysis'})).toBeVisible();
  await page.getByRole('button',{name:'First move',exact:true}).click();await page.getByRole('gridcell',{name:'F16, empty',exact:true}).click();
  view=await (await request.get(`/api/games/${gameId}`)).json();expect(view.game.active_head).toBe(active);expect(Object.keys(view.game.nodes)).toHaveLength(6);
  await expect(page.getByText('Black win probability',{exact:true})).toBeVisible();
  await page.screenshot({path:'test-results/review-desktop.png',fullPage:true});
  await page.getByRole('button',{name:'Play',exact:true}).click();await page.getByRole('button',{name:'Return to active continuation'}).click();await page.getByRole('button',{name:'Resign',exact:true}).click();
  await expect(page.getByRole('button',{name:'Play',exact:true})).toHaveCount(0);
  const audit=await (await request.get(`/api/games/${gameId}/audit`)).json();expect(audit.after).toBeLessThan(audit.before);
  await request.post(`/api/games/${gameId}/resign`,{data:{head:active}});expect(await (await request.get(`/api/games/${gameId}/audit`)).json()).toEqual(audit);
  await page.reload();await expect(page.getByRole('heading',{name:'Position analysis'})).toBeVisible();
  const sgf=await (await request.get(`/api/games/${gameId}/sgf`)).text();expect(sgf).toContain('(;B[');
  const imported=await (await request.post('/api/games/import',{data:{sgf}})).json();const importedGame=await (await request.get(`/api/games/${imported.id}`)).json();expect(Object.keys(importedGame.game.nodes)).toHaveLength(6);
  await page.getByRole('link',{name:'Weiqi home',exact:true}).click();await expect(page.getByRole('heading',{name:/Review games/})).toBeVisible();
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
  await page.getByRole('button',{name:'Disagree / resume play'}).click();await expect(page.getByRole('heading',{name:'Take your time.'})).toBeVisible();
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
    await expect(page.getByRole('gridcell', { name: 'D16, empty', exact: true })).toBeVisible();
    await expect(page.getByRole('alert')).toHaveCount(0);
    expect(navigations).toBe(1);
    // A successful catch-up clears transport errors, but must preserve domain errors.
    await page.route(`**/api/games/${created.id}/moves`, route => route.fulfill({ status: 409, contentType: 'application/json', body: JSON.stringify({ error: 'Illegal move fixture' }) }));
    await page.getByRole('gridcell', { name: 'D16, empty', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('Illegal move fixture');
    await page.evaluate(() => window.dispatchEvent(new Event('focus')));
    await expect(page.getByText('Offline by design', { exact: true })).toBeVisible();
    await expect(page.getByRole('alert')).toContainText('Illegal move fixture');
  } finally {
    await context.setOffline(false);
  }
});

test('keeps game libraries on Home and gives Settings a refreshable page with browser navigation', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Home', exact: true })).toHaveCount(0);
  await expect(page.getByRole('link', { name: 'Weiqi home', exact: true })).toContainText('LOCAL GO TRAINER');
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
  await page.getByRole('link', { name: 'Weiqi home', exact: true }).click();
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
  const brand = page.getByRole('link', { name: 'Weiqi home', exact: true });
  const sidebar = page.locator('.game-sidebar');
  const assertBoardFits = async () => {
    await expect(board).toBeVisible();
    const bounds = await board.boundingBox();
    const header = await page.locator('.app-header').boundingBox();
    const footer = await page.locator('.engine-bar').boundingBox();
    expect(bounds!.width).toBeGreaterThan(180);
    expect(bounds!.y).toBeGreaterThanOrEqual(header!.y + header!.height);
    expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(footer!.y + 1);
    const dimensions = await page.evaluate(() => ({ height: document.documentElement.scrollHeight, width: document.documentElement.scrollWidth, viewportHeight: innerHeight, viewportWidth: innerWidth, scroll: scrollY }));
    expect(dimensions.height).toBeLessThanOrEqual(dimensions.viewportHeight);
    expect(dimensions.width).toBeLessThanOrEqual(dimensions.viewportWidth);
    expect(dimensions.scroll).toBe(0);
  };
  const selectedIsInHistoryViewport = () => history.evaluate(element => {
    const selected = element.querySelector('[aria-current="step"]')!.getBoundingClientRect();
    const bounds = element.getBoundingClientRect();
    return selected.top >= bounds.top - 1 && selected.bottom <= bounds.bottom + 1;
  });
  await expect(page.locator('.app-header').getByRole('heading', { name: 'Human SL 5k · You play Black' })).toBeVisible();
  await expect(page.locator('.app-header').getByRole('button', { name: 'Review', exact: true })).toBeVisible();
  await expect(page.getByText('AUTO OPPONENT', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Arrow keys on the board to move · Enter to place a stone', { exact: true })).toHaveCount(0);
  await assertBoardFits();
  expect(await history.evaluate(element => element.clientHeight)).toBeGreaterThan(400);
  await expect.poll(selectedIsInHistoryViewport).toBe(true);
  await page.screenshot({ path: 'test-results/focused-play-desktop.png', fullPage: true });

  const playFromActive = async (point: number, expectedMoves: number) => {
    const current = await (await request.get(`/api/games/${created.id}`)).json();
    const response = await request.post(`/api/games/${created.id}/moves`, { data: { parent: current.game.active_head, point, review: false, expected_head: current.game.active_head } });
    expect(response.ok()).toBeTruthy();
    await expect.poll(async () => Object.keys((await (await request.get(`/api/games/${created.id}`)).json()).game.nodes).length).toBe(expectedMoves + 1);
    await expect(page.locator('.variation-panel .section-heading')).toContainText(`${expectedMoves} moves`);
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
  await page.getByRole('button', { name: 'First move', exact: true }).click();
  await expect(history.locator('[aria-current="step"]')).toContainText('Starting position');
  await expect.poll(() => history.evaluate(element => element.scrollTop)).toBeLessThan(4);
  await playFromActive(81, 84);
  await expect(history.locator('[aria-current="step"]')).toContainText('Starting position');
  expect(await history.evaluate(element => element.scrollTop)).toBeLessThan(4);
  await assertBoardFits();
  const before = await board.boundingBox();
  await sidebar.hover();
  await page.mouse.wheel(0, 1200);
  await expect.poll(() => sidebar.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
  expect(await board.boundingBox()).toEqual(before);
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
