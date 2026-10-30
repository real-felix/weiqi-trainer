<script lang="ts">
  import { path, type Game, type Evaluation } from './lib';
  import { lead, percent, playerScore, playerWinrate } from './review';
  let {
    game,
    selected,
    evaluations,
    onselect,
  }: {
    game: Game;
    selected: string;
    evaluations: Record<string, Evaluation>;
    onselect: (id: string) => void;
  } = $props();
  const line = $derived(path(game.nodes, game.active_head));
  const selectedMove = $derived(path(game.nodes, selected).length - 1);
  const onPlayed = $derived(line.some((node) => node.id === selected));
  let scrub = $state<number | null>(null);
  let dragging = $state(false);
  const index = $derived(scrub ?? Math.min(selectedMove, line.length - 1));
  const current = $derived(evaluations[scrub === null ? selected : line[index]?.id]);
  const scale = $derived(
    Math.max(
      10,
      Math.ceil(
        Math.max(0, ...line.map((node) => Math.abs(evaluations[node.id]?.score_black ?? 0))) / 5,
      ) * 5,
    ),
  );
  const x = (move: number) => 34 + (move / Math.max(1, line.length - 1)) * 232;
  const scoreY = (evaluation: Evaluation) =>
    48 - (playerScore(evaluation, game.human) / scale) * 34;
  const winY = (evaluation: Evaluation) => 82 - playerWinrate(evaluation, game.human)! * 68;
  const series = (kind: 'score' | 'win') => {
    let joined = false;
    return line
      .map((node, move) => {
        const evaluation = evaluations[node.id];
        if (!evaluation) {
          joined = false;
          return '';
        }
        const command = `${joined ? 'L' : 'M'}${x(move)},${kind === 'score' ? scoreY(evaluation) : winY(evaluation)}`;
        joined = true;
        return command;
      })
      .join(' ');
  };
  function point(event: PointerEvent) {
    const bounds = (event.currentTarget as SVGSVGElement).getBoundingClientRect();
    return Math.max(
      0,
      Math.min(
        line.length - 1,
        Math.round(
          ((((event.clientX - bounds.left) / bounds.width) * 300 - 34) / 232) * (line.length - 1),
        ),
      ),
    );
  }
  function key(event: KeyboardEvent) {
    let next = index;
    if (event.key === 'ArrowLeft') next--;
    else if (event.key === 'ArrowRight') next++;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = line.length - 1;
    else return;
    event.preventDefault();
    event.stopPropagation();
    onselect(line[Math.max(0, Math.min(line.length - 1, next))].id);
  }
</script>

<section class="review-overview" aria-label="Game overview">
  <div class="overview-values">
    <div class="score-value"><strong>{lead(current)}</strong><span>Estimated lead</span></div>
    <div class="win-value">
      <strong>{percent(playerWinrate(current, game.human))}</strong><span
        >{game.human === 'B' ? 'Black' : 'White'} win probability</span
      >
    </div>
  </div>
  <svg
    viewBox="0 0 300 102"
    role="slider"
    tabindex="0"
    aria-label="Review game timeline"
    aria-orientation="horizontal"
    aria-valuemin="0"
    aria-valuemax={line.length - 1}
    aria-valuenow={index}
    aria-valuetext={`Move ${index} of ${line.length - 1}`}
    onkeydown={key}
    onpointerdown={(event) => {
      event.preventDefault();
      (event.currentTarget as SVGSVGElement).setPointerCapture(event.pointerId);
      dragging = true;
      scrub = point(event);
    }}
    onpointermove={(event) => {
      if (dragging) scrub = point(event);
    }}
    onpointerup={(event) => {
      if (!dragging) return;
      const chosen = point(event);
      dragging = false;
      scrub = null;
      onselect(line[chosen].id);
    }}
    onpointercancel={() => {
      dragging = false;
      scrub = null;
    }}
  >
    <title
      >Green: {game.human === 'B' ? 'Black' : 'White'} score lead; blue: {game.human === 'B'
        ? 'Black'
        : 'White'} win probability</title
    >
    <path d="M34 14H266M34 48H266M34 82H266" stroke="var(--line)" stroke-dasharray="3 4" />
    <g class="score-scale"
      ><text x="0" y="18">+{scale}</text><text x="0" y="52">0</text><text x="0" y="86"
        >−{scale}</text
      ></g
    >
    <g class="win-scale" text-anchor="end"
      ><text x="300" y="18">100%</text><text x="300" y="52">50%</text><text x="300" y="86">0%</text
      ></g
    >
    <path d={series('score')} fill="none" stroke="var(--review-score)" stroke-width="1.2" />
    <path
      d={series('win')}
      fill="none"
      stroke="var(--review-win)"
      stroke-width="1.2"
    />
    <line x1={x(index)} x2={x(index)} y1="10" y2="86" stroke="var(--ink)" stroke-width="1" />
    {#if current}<circle
        cx={x(index)}
        cy={scoreY(current)}
        r="3"
        fill="var(--review-score)"
      /><circle cx={x(index)} cy={winY(current)} r="3" fill="var(--review-win)" />{/if}
    <text x="34" y="100">0</text><text x="266" y="100" text-anchor="end">{line.length - 1}</text>
    <text x={Math.max(60, Math.min(240, x(index)))} y="9" text-anchor="middle" class="scrub-label"
      >{!onPlayed && scrub === null ? 'Variation ' : 'Move '}{scrub ?? selectedMove}</text
    >
  </svg>
</section>

<style>
  .review-overview {
    position: sticky;
    top: 0;
    z-index: 3;
    background: var(--paper);
    padding: 0 0 12px;
    margin-bottom: 18px;
    border-bottom: 1px solid var(--line);
  }
  .overview-values {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .overview-values > div {
    display: grid;
    gap: 5px;
  }
  .overview-values > div:last-child {
    text-align: right;
  }
  strong {
    font-size: 1.375rem;
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  span {
    font-size: 0.75rem;
  }
  .score-value {
    color: var(--review-score);
  }
  .win-value {
    color: var(--review-win);
  }
  svg {
    display: block;
    width: 100%;
    height: auto;
    touch-action: none;
    cursor: crosshair;
    border-radius: 4px;
  }
  svg:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  text {
    font-size: 9px;
    fill: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .score-scale text {
    fill: var(--review-score);
  }
  .win-scale text {
    fill: var(--review-win);
  }
  .scrub-label {
    fill: var(--ink);
  }
</style>
