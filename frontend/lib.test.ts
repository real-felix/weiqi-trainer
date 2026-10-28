import { describe, it, expect } from 'vitest';
import { coordinate, rankLabel, path, treeRows, groupAt, colors } from './lib';
import { fixtureGame } from './fixtures';
describe('presentation transforms', () => {
  it('keeps active line, selected node, forks and move depth distinct', () => {
    expect(path(fixtureGame.nodes, 'b').map((n) => n.id)).toEqual(['root', 'a', 'b']);
    expect(treeRows(fixtureGame).map((r) => [r.node.id, r.active, r.depth, r.move])).toEqual([
      ['root', true, 0, 0],
      ['a', true, 0, 1],
      ['b', true, 0, 2],
      ['fork', false, 1, 1],
    ]);
  });
  it('uses standard Go coordinates and rank labels', () => {
    expect(coordinate(60)).toBe('D16');
    expect(coordinate(null)).toBe('Pass');
    expect(rankLabel(-1)).toBe('1k');
    expect(rankLabel(0)).toBe('1d');
  });
  it('walks long paths without enumerating the whole tree and stops at cycles', () => {
    const nodes = Object.fromEntries(
      Array.from({ length: 1000 }, (_, i) => [
        String(i),
        { ...fixtureGame.nodes.a, id: String(i), parent: i ? String(i - 1) : null },
      ]),
    );
    const indexed = new Proxy(nodes, {
      ownKeys() {
        throw new Error('A parent traversal must not enumerate unrelated nodes');
      },
    });
    expect(path(indexed, '999')).toHaveLength(1000);
    expect(path(indexed, 'missing')).toEqual([]);
    nodes['0'].parent = '999';
    expect(new Set(path(indexed, '999').map((node) => node.id)).size).toBe(1000);
    expect(path(indexed, '999')).toHaveLength(1000);
  });
  it('selects complete connected stone groups without wrapping rows', () => {
    const board = Array(361).fill(0);
    for (const p of [0, 1, 19, 18]) board[p] = 1;
    expect(groupAt(board, 0).sort((a, b) => a - b)).toEqual([0, 1, 19]);
  });
  it('keeps all five exact classification colors', () => {
    expect(Object.values(colors)).toEqual(['#3069ad', '#287647', '#8b6500', '#a64c10', '#b8363b']);
  });
});
