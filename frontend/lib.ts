export type Color = 'B' | 'W';
export interface Node {
  id: string;
  parent: string | null;
  color: Color | null;
  point: number | null;
  created_at: string;
}
export interface Game {
  id: string;
  root: string;
  active_head: string;
  human: Color;
  rank: number;
  mode: string;
  status: string;
  result: string | null;
  created_at: string;
  updated_at: string;
  nodes: Record<string, Node>;
  resume_heads: string[];
  imported: boolean;
}
export interface Candidate {
  point: number | null;
  visits: number;
  score_black: number;
  winrate_black: number;
  pv: (number | null)[];
  order: number;
}
export interface Evaluation {
  score_black: number;
  winrate_black: number;
  visits: number;
  candidates: Candidate[];
  ownership: number[];
  engine: string;
  model: string;
  config: string;
  parameters: unknown;
  created_at: string;
}
export interface Review {
  node: string;
  label: string | null;
  score_loss: number | null;
  winrate_loss: number | null;
  version: string;
  confidence: string;
}
export interface Position {
  board: number[];
  turn: Color;
  passes: number;
  captures: [number, number];
}
export interface GameView {
  game: Game;
  position: Position;
  selected: string;
  evaluations: Record<string, Evaluation>;
  reviews: Review[];
  thinking: boolean;
}
export interface WorkerStatus {
  state: string;
  detail: string;
  error: string | null;
  visits: number;
  pid: number | null;
  version: string;
  recent_log: string[];
  supported_ranks: number[];
}
export interface Settings {
  executable: string;
  human_model: string;
  analysis_model: string;
  config: string;
  quick_visits: number;
  deep_visits: number;
  timeout_seconds: number;
  resignation_enabled: boolean;
  resignation_winrate: number;
  resignation_score: number;
  resignation_min_moves: number;
  resignation_confirmations: number;
  classification: {
    version: string;
    minimum_visits: number;
    score_thresholds: number[];
    winrate_thresholds: number[];
  };
}
export interface State {
  schema_version: number;
  games: Game[];
  opponent_rank: number;
  initialized: boolean;
  settings: Settings;
  engine: { human: WorkerStatus; analysis: WorkerStatus };
  data_dir: string;
}
export interface Scoring {
  score: { black: number; white: number; result: string };
  suggested_dead: number[];
  agreed: boolean;
  uncertain: boolean;
  message: string;
}
export const labels = ['AI move', 'Good move', 'Inaccuracy', 'Mistake', 'Blunder'] as const;
export const colors: Record<string, string> = {
  'AI move': '#3069ad',
  'Good move': '#287647',
  Inaccuracy: '#8b6500',
  Mistake: '#a64c10',
  Blunder: '#b8363b',
};
export const rankLabel = (rank: number) => (rank < 0 ? `${-rank}k` : `${rank + 1}d`);
export const coordinate = (point: number | null) =>
  point === null ? 'Pass' : `${'ABCDEFGHJKLMNOPQRST'[point % 19]}${19 - Math.floor(point / 19)}`;
export const ranks = Array.from({ length: 29 }, (_, i) => i - 20);
export function path(nodes: Record<string, Node>, head: string): Node[] {
  const result: Node[] = [];
  let current: string | null = head;
  while (current && nodes[current]) {
    const n: Node = nodes[current];
    result.push(n);
    current = n.parent;
    if (result.length > Object.keys(nodes).length) break;
  }
  return result.reverse();
}
export function treeRows(
  game: Game,
): { node: Node; depth: number; move: number; active: boolean; fork: boolean }[] {
  const active = new Set(path(game.nodes, game.active_head).map((n) => n.id));
  const children = new Map<string, Node[]>();
  for (const n of Object.values(game.nodes)) {
    if (n.parent) children.set(n.parent, [...(children.get(n.parent) ?? []), n]);
  }
  const rows: ReturnType<typeof treeRows> = [];
  function walk(id: string, depth: number, move: number, fork: boolean) {
    const node = game.nodes[id];
    if (!node) return;
    rows.push({ node, depth, move, active: active.has(id), fork });
    const next = (children.get(id) ?? []).sort(
      (a, b) =>
        Number(active.has(b.id)) - Number(active.has(a.id)) ||
        a.created_at.localeCompare(b.created_at) ||
        a.id.localeCompare(b.id),
    );
    for (const [i, child] of next.entries())
      walk(child.id, depth + (i > 0 ? 1 : 0), move + 1, next.length > 1);
  }
  walk(game.root, 0, 0, false);
  return rows;
}
export function groupAt(board: number[], point: number): number[] {
  if (!board[point]) return [];
  const seen = new Set([point]);
  const stack = [point];
  while (stack.length) {
    const p = stack.pop()!;
    const x = p % 19;
    const y = Math.floor(p / 19);
    for (const n of [
      x > 0 ? p - 1 : -1,
      x < 18 ? p + 1 : -1,
      y > 0 ? p - 19 : -1,
      y < 18 ? p + 19 : -1,
    ]) {
      if (n >= 0 && board[n] === board[point] && !seen.has(n)) {
        seen.add(n);
        stack.push(n);
      }
    }
  }
  return [...seen];
}
export async function api<T>(url: string, body?: unknown, method = 'POST'): Promise<T> {
  const response = await fetch(
    `/api${url}`,
    body === undefined
      ? undefined
      : { method, headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) },
  );
  const result = await response.json();
  if (!response.ok) throw new Error(result.error ?? `Request failed (${response.status})`);
  return result as T;
}
export function shortDate(value: string) {
  return new Date(value).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}
export function lastPlayed(value: string) {
  const minutes = Math.max(0, Math.round((Date.now() - Date.parse(value)) / 60000));
  return minutes < 1
    ? 'Just played'
    : minutes < 60
      ? `${minutes}m ago`
      : minutes < 1440
        ? `${Math.floor(minutes / 60)}h ago`
        : shortDate(value);
}
