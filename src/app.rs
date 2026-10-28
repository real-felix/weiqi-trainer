use crate::{
    domain::*,
    engine::{Engine, Evaluation},
    preview::{Preview, PreviewCache},
    repository::Repository,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::{Notify, broadcast};

#[derive(Clone)]
pub struct Job {
    pub game: String,
    pub node: String,
    pub visits: u32,
    pub priority: bool,
}
pub struct App {
    pub repo: Mutex<Repository>,
    pub engine: Engine,
    previews: Mutex<PreviewCache>,
    queue: Mutex<VecDeque<Job>>,
    running: Mutex<Option<Job>>,
    epoch: AtomicU64,
    wake: Notify,
    pub thinking: Mutex<HashSet<String>>,
    pub data_dir: String,
}
#[derive(Serialize)]
pub struct MoveReview {
    pub node: String,
    pub label: Option<String>,
    pub score_loss: Option<f64>,
    pub winrate_loss: Option<f64>,
    pub version: String,
    pub confidence: String,
}
#[derive(Serialize)]
pub struct GameView {
    pub game: Game,
    pub position: Position,
    pub selected: String,
    pub evaluations: BTreeMap<String, Evaluation>,
    pub reviews: Vec<MoveReview>,
    pub thinking: bool,
}
impl App {
    pub fn new(repo: Repository, data_dir: String) -> Arc<Self> {
        let (events, _) = broadcast::channel(128);
        Arc::new(Self {
            repo: Mutex::new(repo),
            engine: Engine::new(events),
            previews: Mutex::new(PreviewCache::default()),
            queue: Mutex::new(VecDeque::new()),
            running: Mutex::new(None),
            epoch: AtomicU64::new(0),
            wake: Notify::new(),
            thinking: Mutex::new(HashSet::new()),
            data_dir,
        })
    }
    pub fn changed(&self) {
        let _ = self.engine.events.send("refresh".into());
    }
    pub fn preview(&self, id: &str, head: &str) -> Result<Arc<Preview>> {
        // Serialize cold builds so concurrent image requests do not duplicate work.
        // Cache hits need neither SQLite nor engine analysis.
        let mut cache = self.previews.lock().unwrap();
        if let Some(preview) = cache.get(id, head) {
            return Ok(preview);
        }
        let game = self.repo.lock().unwrap().game(id)?;
        let preview = Arc::new(Preview::render(&game, head)?);
        cache.insert(id, head, preview.clone());
        Ok(preview)
    }
    pub fn enqueue(&self, game: &str, node: &str, visits: u32, priority: bool) {
        {
            let running = self.running.lock().unwrap();
            if running
                .as_ref()
                .is_some_and(|j| j.game == game && j.node == node && j.visits >= visits)
            {
                return;
            }
        }
        let mut q = self.queue.lock().unwrap();
        if let Some(existing) = q.iter_mut().find(|j| j.game == game && j.node == node) {
            existing.visits = existing.visits.max(visits);
            if !priority {
                return;
            }
        }
        if priority {
            q.retain(|j| !(j.game == game && (j.node == node || j.priority)));
        } else if q.iter().any(|j| j.game == game && j.node == node) {
            return;
        }
        if q.len() >= 512 {
            q.pop_back();
        }
        let job = Job {
            game: game.into(),
            node: node.into(),
            visits,
            priority,
        };
        if priority {
            q.push_front(job);
        } else {
            q.push_back(job);
        }
        drop(q);
        if priority
            && self
                .running
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|j| j.game != game || j.node != node)
        {
            self.engine.cancel_analysis();
        }
        self.wake.notify_one();
    }
    pub fn cancel_queued(&self, game: &str) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.queue.lock().unwrap().retain(|j| j.game != game);
        self.engine.cancel_analysis();
        self.changed();
    }
    pub fn background(self: &Arc<Self>) {
        let app = self.clone();
        tokio::spawn(async move {
            loop {
                app.wake.notified().await;
                loop {
                    if !app.thinking.lock().unwrap().is_empty() {
                        break;
                    }
                    let Some(job) = app.queue.lock().unwrap().pop_front() else {
                        break;
                    };
                    let data = {
                        let r = app.repo.lock().unwrap();
                        r.game(&job.game).and_then(|g| {
                            Ok((
                                g,
                                r.settings()?,
                                r.evaluations(&job.game)?
                                    .get(&job.node)
                                    .map(|e| e.visits)
                                    .unwrap_or(0),
                            ))
                        })
                    };
                    let Ok((g, s, existing)) = data else {
                        continue;
                    };
                    if existing >= job.visits {
                        continue;
                    }
                    let epoch = app.epoch.load(Ordering::SeqCst);
                    *app.running.lock().unwrap() = Some(job.clone());
                    let result = app.engine.analyze(&s, &g, &job.node, job.visits).await;
                    *app.running.lock().unwrap() = None;
                    match result {
                        Ok(e) => {
                            let _ = app
                                .repo
                                .lock()
                                .unwrap()
                                .save_evaluation(&g.id, &job.node, &e);
                            if s.resignation_enabled
                                && g.path(&job.node)
                                    .is_ok_and(|p| p.len() > s.resignation_min_moves)
                                && g.nodes[&job.node].color == Some(g.human)
                                && e.visits < s.deep_visits.max(256)
                            {
                                let win = if g.human == Color::B {
                                    1. - e.winrate_black
                                } else {
                                    e.winrate_black
                                };
                                let score = if g.human == Color::B {
                                    -e.score_black
                                } else {
                                    e.score_black
                                };
                                if win < s.resignation_winrate && score < -s.resignation_score {
                                    app.enqueue(&g.id, &job.node, s.deep_visits.max(256), false);
                                }
                            }
                            let current = app.repo.lock().unwrap().game(&g.id);
                            if let Ok(current) = current
                                && app.resignation(&current).unwrap_or(false)
                            {
                                let _ = app.repo.lock().unwrap().finalize(
                                    &g.id,
                                    &current.active_head,
                                    &format!("{}+R", current.human),
                                );
                            }
                            app.changed();
                        }
                        Err(e) if e == "Analysis cancelled" => {
                            if epoch == app.epoch.load(Ordering::SeqCst) {
                                app.enqueue(&job.game, &job.node, job.visits, false);
                            }
                            break;
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
        });
    }
    pub fn kick(self: &Arc<Self>, id: &str) {
        let Ok(g) = self.repo.lock().unwrap().game(id) else {
            return;
        };
        if g.status != "playing"
            || g.result.is_some()
            || g.imported
            || g.position(&g.active_head)
                .map_or(true, |p| p.turn == g.human)
        {
            return;
        }
        if !self.thinking.lock().unwrap().insert(id.into()) {
            return;
        }
        self.engine.cancel_analysis();
        self.changed();
        let app = self.clone();
        tokio::spawn(async move {
            let settings = app.repo.lock().unwrap().settings();
            if let Ok(s) = settings {
                let generated = app.engine.generate(&s, &g).await;
                if let Ok(point) = generated {
                    let mut repo = app.repo.lock().unwrap();
                    if let Ok(mut current) = repo.game(&g.id)
                        && let Ok(node) = current.add_opponent(&g.active_head, point)
                    {
                        let _ = repo.save(&current);
                        app.enqueue(&g.id, &g.active_head, s.quick_visits, false);
                        app.enqueue(&g.id, &node, s.quick_visits, false);
                    }
                }
            }
            app.thinking.lock().unwrap().remove(&g.id);
            let current = app.repo.lock().unwrap().game(&g.id);
            if current.is_ok_and(|current| current.active_head != g.active_head) {
                app.kick(&g.id);
            }
            app.wake.notify_one();
            app.changed();
        });
    }
    pub fn setup(self: &Arc<Self>) {
        let app = self.clone();
        tokio::spawn(async move {
            app.engine.cancel_analysis();
            app.engine.restart().await;
            let settings = app.repo.lock().unwrap().settings();
            if let Ok(s) = settings {
                let human = app.engine.probe_human(&s);
                let sample = Game::new(Color::B, -5, "auto".into());
                let strong = app
                    .engine
                    .analyze(&s, &sample, &sample.root, s.quick_visits);
                let _ = tokio::join!(human, strong);
                let games = app.repo.lock().unwrap().list().unwrap_or_default();
                for g in games {
                    app.kick(&g.id);
                    let evals = app.repo.lock().unwrap().evaluations(&g.id);
                    if let Ok(evals) = evals {
                        for node in g.nodes.keys() {
                            if !evals.contains_key(node) {
                                app.enqueue(&g.id, node, s.quick_visits, false);
                            }
                        }
                    }
                }
                app.wake.notify_one();
                app.changed();
            }
        });
    }
    pub fn view(&self, id: &str, node: Option<&str>) -> Result<GameView> {
        let r = self.repo.lock().unwrap();
        let g = r.game(id)?;
        let selected = node.unwrap_or(&g.active_head).to_string();
        let position = g.position(&selected)?;
        let evaluations = r.evaluations(id)?;
        let s = r.settings()?;
        let reviews = g
            .nodes
            .values()
            .filter(|n| n.color.is_some())
            .map(|n| {
                let before = n.parent.as_ref().and_then(|id| evaluations.get(id));
                let after = evaluations.get(&n.id);
                let mut review = MoveReview {
                    node: n.id.clone(),
                    label: None,
                    score_loss: None,
                    winrate_loss: None,
                    version: s.classification.version.clone(),
                    confidence: "Awaiting analysis".into(),
                };
                if let Some(before) = before {
                    let candidate = before.candidates.iter().find(|c| {
                        c.point == n.point && c.visits >= s.classification.minimum_visits
                    });
                    let actual = candidate
                        .map(|c| (c.score_black, c.winrate_black, c.visits))
                        .or_else(|| after.map(|e| (e.score_black, e.winrate_black, e.visits)));
                    if let (Some(best), Some((score, winrate, visits))) =
                        (before.candidates.first(), actual)
                    {
                        let sign = if n.color == Some(Color::B) { 1. } else { -1. };
                        let score_loss = ((best.score_black - score) * sign).max(0.);
                        let winrate_loss = ((best.winrate_black - winrate) * sign).max(0.);
                        let confidence = visits.min(before.visits).min(best.visits);
                        review.score_loss = Some(score_loss);
                        review.winrate_loss = Some(winrate_loss);
                        review.label = classify(
                            score_loss,
                            winrate_loss,
                            g.rank,
                            best.point == n.point,
                            confidence,
                            &s.classification,
                        )
                        .map(String::from);
                        review.confidence = if review.label.is_some() {
                            format!("{confidence} visits · heuristic")
                        } else {
                            format!("{confidence} visits · low confidence")
                        };
                    }
                }
                review
            })
            .collect();
        let thinking = self.thinking.lock().unwrap().contains(id);
        Ok(GameView {
            game: g,
            position,
            selected,
            evaluations,
            reviews,
            thinking,
        })
    }
    pub fn resignation(&self, g: &Game) -> Result<bool> {
        let r = self.repo.lock().unwrap();
        let s = r.settings()?;
        if !s.resignation_enabled || g.result.is_some() || g.status != "playing" {
            return Ok(false);
        }
        let path = g.path(&g.active_head)?;
        if path.len() - 1 < s.resignation_min_moves {
            return Ok(false);
        }
        let ev = r.evaluations(&g.id)?;
        let recent: Vec<_> = path
            .iter()
            .rev()
            .filter(|n| n.color == Some(g.human))
            .take(s.resignation_confirmations)
            .collect();
        if recent.len() != s.resignation_confirmations {
            return Ok(false);
        }
        Ok(recent.iter().all(|n| {
            ev.get(&n.id).is_some_and(|e| {
                let win = if g.human == Color::B {
                    1. - e.winrate_black
                } else {
                    e.winrate_black
                };
                let score = if g.human == Color::B {
                    -e.score_black
                } else {
                    e.score_black
                };
                e.visits >= s.deep_visits.max(256)
                    && win < s.resignation_winrate
                    && score < -s.resignation_score
            })
        }))
    }
}

#[derive(Serialize)]
pub struct Scoring {
    pub score: Score,
    pub suggested_dead: Vec<usize>,
    pub agreed: bool,
    pub uncertain: bool,
    pub message: String,
}
pub fn scoring(position: &Position, e: &Evaluation, dead: &[usize]) -> Result<Scoring> {
    let score = position.score(dead)?;
    if e.ownership.len() != POINTS {
        return Err("Scoring requires a strong analysis with ownership".into());
    }
    let mut seen: HashSet<usize> = HashSet::new();
    let mut suggested = vec![];
    let mut uncertain = false;
    for p in 0..POINTS {
        if position.board[p] == 0 || seen.contains(&p) {
            continue;
        }
        let stones = group(&position.board, p).0;
        seen.extend(stones.iter());
        let sign = if position.board[p] == 1 { 1. } else { -1. };
        if stones.iter().all(|&s| e.ownership[s] * sign < -0.8) {
            suggested.extend(stones);
        } else if !stones.iter().all(|&s| e.ownership[s] * sign > 0.8) {
            uncertain = true;
        }
    }
    suggested.sort_unstable();
    let mut marked = dead.to_vec();
    marked.sort_unstable();
    marked.dedup();
    let agreed = !uncertain && suggested == marked;
    Ok(Scoring {score,suggested_dead:suggested,agreed,uncertain,message:if uncertain {"Stone status is uncertain. Resume play to settle the position."} else if agreed {"Both sides agree on stone status. Confirm this result to finish permanently."} else {"The opponent disagrees with the marked dead groups. Compare its suggestion or resume play."}.into()})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::{Candidate, decode_evaluation},
        settings::Settings,
    };
    use serde_json::json;
    fn evaluation() -> Evaluation {
        decode_evaluation(
            &serde_json::from_str(include_str!("../tests/fixtures/strong.json")).unwrap(),
            "fixture",
            &Settings::default(),
            json!({}),
        )
        .unwrap()
    }
    #[test]
    fn scoring_never_confirms_uncertain_or_disputed_dead_stones() {
        let mut p = Position::default();
        p.play(Some(0)).unwrap();
        p.play(Some(360)).unwrap();
        let mut e = evaluation();
        e.ownership = vec![0.; POINTS];
        assert!(scoring(&p, &e, &[]).unwrap().uncertain);
        assert!(!scoring(&p, &e, &[]).unwrap().agreed);
        e.ownership[0] = 1.;
        e.ownership[360] = 1.;
        let disputed = scoring(&p, &e, &[]).unwrap();
        assert_eq!(disputed.suggested_dead, vec![360]);
        assert!(!disputed.agreed);
        let agreed = scoring(&p, &e, &[360]).unwrap();
        assert!(agreed.agreed);
        assert_eq!(agreed.score.result, "B+353.5");
    }
    #[test]
    fn white_loss_compares_normalized_best_alternative_with_actual_continuation() {
        let mut r = Repository::open(Path::new(":memory:")).unwrap();
        let mut g = Game::new(Color::B, -5, "auto".into());
        let root = g.root.clone();
        let a = g.add(&root, Some(0), false).unwrap();
        let b = g.add_opponent(&a, Some(360)).unwrap();
        r.save(&g).unwrap();
        let mut before = evaluation();
        before.visits = 512;
        before.candidates = vec![Candidate {
            point: Some(359),
            visits: 300,
            score_black: -2.,
            winrate_black: 0.4,
            pv: vec![Some(359)],
            order: 0,
        }];
        let mut after = before.clone();
        after.score_black = 5.;
        after.winrate_black = 0.8;
        r.save_evaluation(&g.id, &a, &before).unwrap();
        r.save_evaluation(&g.id, &b, &after).unwrap();
        let app = App::new(r, "test".into());
        let view = app.view(&g.id, None).unwrap();
        let review = view.reviews.iter().find(|r| r.node == b).unwrap();
        assert_eq!(review.score_loss, Some(7.));
        assert_eq!(review.label.as_deref(), Some("Blunder"));
    }
    #[test]
    fn analysis_queue_deduplicates_promotes_and_cancels() {
        let r = Repository::open(Path::new(":memory:")).unwrap();
        let a = App::new(r, "test".into());
        a.enqueue("g", "n", 64, false);
        a.enqueue("g", "n", 256, true);
        a.enqueue("g", "other", 64, false);
        assert_eq!(a.queue.lock().unwrap().len(), 2);
        assert_eq!(a.queue.lock().unwrap().front().unwrap().visits, 256);
        a.cancel_queued("g");
        assert!(a.queue.lock().unwrap().is_empty());
    }
    use std::path::Path;
}
