// Opt-in hardware integration: real binary, real models, actual browser, server restart.
import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';

const data=await mkdtemp(join(tmpdir(),'weiqi-real-'));
const url='http://127.0.0.1:43128';
let child;
let browser;
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function request(path,body) {
  const r=await fetch(url+'/api'+path,body ? {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)} : undefined);
  const value=await r.json();if(!r.ok) throw new Error(JSON.stringify(value));return value;
}
async function start() {
  child=spawn('target/debug/weiqi-trainer',[],{env:{...process.env,WEIQI_PORT:'43128',WEIQI_DATA_DIR:data},stdio:'inherit'});
  const deadline=Date.now()+90000;
  while(Date.now()<deadline) {
    try {const s=await request('/state');if(s.engine.human.supported_ranks.length===29 && s.engine.analysis.state==='ready') return;}catch{/* Wait for listener. */}
    await pause(200);
  } throw new Error('Real engine startup timed out');
}
async function stop() {
  if(!child || child.exitCode!==null) return;
  const exited=new Promise(resolve=>child.once('exit',resolve));child.kill('SIGINT');
  await Promise.race([exited,pause(10000).then(()=>{if(child.exitCode===null) throw new Error('Graceful shutdown did not complete');})]);
}
try {
  await start();
  browser=await chromium.launch();
  const page=await browser.newPage({viewport:{width:1360,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(url);await page.getByRole('button',{name:'New game'}).click();await page.getByLabel('Your color').selectOption('B');await page.getByRole('button',{name:'Start game',exact:true}).click();
  await page.getByRole('gridcell',{name:'D4, empty',exact:true}).click();
  const id=page.url().split('/').pop();let view;const deadline=Date.now()+30000;
  do {await pause(100);view=await request(`/games/${id}`);} while((Object.keys(view.game.nodes).length<3 || view.thinking) && Date.now()<deadline);
  assert.equal(Object.keys(view.game.nodes).length,3);assert.equal(view.position.turn,'B');
  await expect(page.getByRole('heading',{name:'Take your time.'})).toBeVisible();
  await expect(page.locator('svg.board circle[fill="url(#black)"]')).toHaveCount(1);
  await expect(page.locator('svg.board circle[fill="url(#white)"]')).toHaveCount(1);
  await page.getByRole('gridcell',{name:'E16, empty',exact:true}).hover();
  await expect(page.getByRole('gridcell',{name:'E16, empty',exact:true})).toHaveCSS('background-color','rgba(0, 0, 0, 0)');
  const active=view.game.active_head;
  await mkdir('.local',{recursive:true});await page.screenshot({path:'.local/real-play.png',fullPage:true});
  await page.getByRole('button',{name:'Review',exact:true}).click();await page.getByRole('button',{name:'First move',exact:true}).click();await page.getByRole('gridcell',{name:'D16, empty',exact:true}).click();
  await expect(page.getByRole('gridcell',{name:'D16, black stone',exact:true})).toBeVisible();
  await page.getByRole('heading',{name:'Position analysis'}).waitFor();
  await page.getByText('Black win probability',{exact:true}).waitFor({timeout:30000});
  view=await request(`/games/${id}`);assert.equal(view.game.active_head,active);assert.equal(Object.keys(view.game.nodes).length,4);
  await page.screenshot({path:'.local/real-review.png',fullPage:true});
  await page.close();await browser.close();browser=null;
  await stop();await start();view=await request(`/games/${id}`);
  assert.equal(view.game.active_head,active);assert.equal(Object.keys(view.game.nodes).length,4);assert.equal(view.game.result,null);assert.ok(Object.keys(view.evaluations).length>0);assert.deepEqual(errors,[]);
  console.log('PASS: real human SL game, review branch, analysis, and persistence across backend restart');
} finally {if(browser) await browser.close();await stop();await rm(data,{recursive:true,force:true});}
