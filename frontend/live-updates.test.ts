import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { watchLiveUpdates } from './live-updates';

class Stream extends EventTarget {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSED = 2;
  static instances: Stream[] = [];
  readyState = Stream.CONNECTING;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  close = vi.fn(() => (this.readyState = Stream.CLOSED));
  constructor(public url: string) {
    super();
    Stream.instances.push(this);
  }
  open() {
    this.readyState = Stream.OPEN;
    this.onopen?.();
  }
}
let stop: (() => void) | undefined;
beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-10-28T08:00:00Z'));
  Stream.instances = [];
  vi.stubGlobal('EventSource', Stream);
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('visible');
});
afterEach(() => {
  stop?.();
  stop = undefined;
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
function watch() {
  const onSync = vi.fn();
  const onConnection = vi.fn();
  stop = watchLiveUpdates({ onSync, onConnection });
  return { onSync, onConnection };
}
it('retries a failed stream with bounded backoff and resynchronizes on open', () => {
  const { onSync, onConnection } = watch();
  const first = Stream.instances[0];
  first.open();
  first.dispatchEvent(new Event('sync'));
  vi.advanceTimersByTime(90);
  expect(onSync).toHaveBeenCalledTimes(1);
  first.onerror?.();
  expect(first.close).toHaveBeenCalled();
  expect(onConnection).toHaveBeenLastCalledWith(false);
  for (const delay of [1_000, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000]) {
    const count = Stream.instances.length;
    vi.advanceTimersByTime(delay - 1);
    expect(Stream.instances).toHaveLength(count);
    vi.advanceTimersByTime(1);
    expect(Stream.instances).toHaveLength(count + 1);
    Stream.instances.at(-1)!.onerror?.();
  }
  vi.advanceTimersByTime(30_000);
  Stream.instances.at(-1)!.open();
  vi.advanceTimersByTime(90);
  expect(onConnection).toHaveBeenLastCalledWith(true);
  expect(onSync).toHaveBeenCalledTimes(2);
});
it('reopens and catches up after sleep even if the browser still reports an open stream', () => {
  const { onSync } = watch();
  const old = Stream.instances[0];
  old.open();
  vi.advanceTimersByTime(90);
  vi.setSystemTime(Date.now() + 8 * 60 * 60 * 1_000);
  vi.advanceTimersByTime(15_000);
  expect(old.close).toHaveBeenCalled();
  expect(Stream.instances).toHaveLength(2);
  vi.advanceTimersByTime(90);
  expect(onSync).toHaveBeenCalledTimes(2);
});
it('coalesces wake signals, ignores hidden tabs and stale stream callbacks', () => {
  const { onSync, onConnection } = watch();
  const old = Stream.instances[0];
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
  window.dispatchEvent(new Event('focus'));
  vi.advanceTimersByTime(100);
  expect(Stream.instances).toHaveLength(1);
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('visible');
  document.dispatchEvent(new Event('visibilitychange'));
  window.dispatchEvent(new Event('focus'));
  window.dispatchEvent(new Event('online'));
  vi.advanceTimersByTime(100);
  expect(Stream.instances).toHaveLength(2);
  expect(onSync).toHaveBeenCalledTimes(1);
  old.open();
  old.onerror?.();
  old.dispatchEvent(new Event('sync'));
  vi.advanceTimersByTime(100);
  expect(onConnection).toHaveBeenLastCalledWith(false);
  expect(onSync).toHaveBeenCalledTimes(1);
});
it('replaces a stream stuck connecting and removes all listeners and timers on unmount', () => {
  const { onSync } = watch();
  vi.advanceTimersByTime(30_001);
  expect(Stream.instances).toHaveLength(2);
  const current = Stream.instances[1];
  current.onerror?.();
  stop!();
  expect(vi.getTimerCount()).toBe(0);
  window.dispatchEvent(new Event('focus'));
  window.dispatchEvent(new Event('pageshow'));
  document.dispatchEvent(new Event('visibilitychange'));
  current.open();
  vi.advanceTimersByTime(120_000);
  expect(Stream.instances).toHaveLength(2);
  expect(onSync).not.toHaveBeenCalled();
});
