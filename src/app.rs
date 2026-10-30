use crate::settings::Settings;
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

fn review_visit_limit(settings: &Settings) -> u32 {
    settings.deep_visits.max(4096)
}

fn review_budget(evaluation: Option<&Evaluation>, settings: &Settings) -> Option<u32> {
    let minimum = settings.classification.minimum_visits;
    if evaluation.is_some_and(|evaluation| {
        evaluation.visits >= minimum
            && evaluation
                .candidates
                .first()
                .is_some_and(|best| best.visits >= minimum)
    }) {
        return None;
    }
    let visits = evaluation.map_or(0, |evaluation| evaluation.visits);
    let limit = review_visit_limit(settings);
    (visits < limit).then(|| {
        settings
            .deep_visits
            .max(visits.saturating_mul(2))
            .min(limit)
    })
}

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
    pub previous_position: Option<Position>,
    pub selected: String,
    pub evaluations: BTreeMap<String, Evaluation>,
    pub reviews: Vec<MoveReview>,
    pub thinking: bool,
    pub analysis: AnalysisProgress,
}
#[derive(Serialize)]
pub struct AnalysisProgress {
    pub completed: usize,
    pub total: usize,
    pub queued: usize,
    pub running: bool,
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
    pub fn enqueue(&self, game: &str, node: &str, mut visits: u32, priority: bool) {
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
            visits = existing.visits.max(visits);
            existing.visits = visits;
            if !priority {
                return;
            }
        }
        if priority {
            q.retain(|j| !(j.game == game && j.node == node));
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
    pub fn enqueue_review(&self, game: &str, node: &str, parent: Option<&str>, visits: u32) {
        let running = self.running.lock().unwrap().clone();
        let targets: Vec<_> = [Some(node), parent].into_iter().flatten().collect();
        let mut queue = self.queue.lock().unwrap();
        // Keep unfinished work when selection changes, but move the comparison pair ahead of it.
        for job in queue.iter_mut().filter(|job| job.game == game) {
            job.priority = false;
        }
        for target in &targets {
            let mut budget = visits;
            queue.retain(|job| {
                if job.game == game && job.node == *target {
                    budget = budget.max(job.visits);
                    false
                } else {
                    true
                }
            });
            if running
                .as_ref()
                .is_some_and(|job| job.game == game && job.node == *target && job.visits >= budget)
            {
                continue;
            }
            if queue.len() >= 512 {
                queue.pop_back();
            }
            queue.push_front(Job {
                game: game.into(),
                node: (*target).into(),
                visits: budget,
                priority: true,
            });
        }
        drop(queue);
        if running.is_some_and(|job| job.game != game || !targets.contains(&job.node.as_str())) {
            self.engine.cancel_analysis();
        }
        self.wake.notify_one();
        self.changed();
    }
    pub fn prepare_review(&self, id: &str) -> Result<()> {
        let jobs = {
            let repo = self.repo.lock().unwrap();
            let game = repo.game(id)?;
            let settings = repo.settings()?;
            let evaluations = repo.evaluations(id)?;
            let mut jobs = vec![];
            for node in game.nodes.values() {
                if evaluations
                    .get(&node.id)
                    .is_none_or(|evaluation| evaluation.visits < settings.quick_visits)
                {
                    jobs.push((node.id.clone(), settings.quick_visits));
                }
                if node.color.is_some()
                    && let Some(parent) = &node.parent
                {
                    let before = evaluations.get(parent);
                    // Total visits do not guarantee enough samples for the best move.
                    // Refine weak comparisons in bounded steps until they meet confidence.
                    if let Some(visits) = review_budget(before, &settings) {
                        jobs.push((parent.clone(), visits));
                    }
                    let minimum = settings.classification.minimum_visits;
                    let actual_ready = before.is_some_and(|evaluation| {
                        evaluation.candidates.iter().any(|candidate| {
                            candidate.point == node.point && candidate.visits >= minimum
                        })
                    }) || evaluations
                        .get(&node.id)
                        .is_some_and(|evaluation| evaluation.visits >= minimum);
                    if !actual_ready {
                        jobs.push((
                            node.id.clone(),
                            settings
                                .quick_visits
                                .max(minimum)
                                .min(review_visit_limit(&settings)),
                        ));
                    }
                }
            }
            jobs
        };
        for (node, visits) in jobs {
            self.enqueue(id, &node, visits, false);
        }
        Ok(())
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
                            if epoch == app.epoch.load(Ordering::SeqCst) {
                                let _ = app.prepare_review(&g.id);
                                if job.priority
                                    && let Some(visits) = review_budget(Some(&e), &s)
                                {
                                    app.enqueue(&g.id, &job.node, visits, true);
                                }
                            }
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
                        app.enqueue(&g.id, &g.active_head, s.deep_visits, false);
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
                    let Ok(g) = app.repo.lock().unwrap().game(&g.id) else {
                        continue;
                    };
                    app.kick(&g.id);
                    let evals = app.repo.lock().unwrap().evaluations(&g.id);
                    if let Ok(evals) = evals {
                        for node in g.nodes.keys() {
                            if !evals.contains_key(node) {
                                app.enqueue(&g.id, node, s.quick_visits, false);
                            }
                        }
                    }
                    let _ = app.prepare_review(&g.id);
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
        let previous_position = g.nodes[&selected]
            .parent
            .as_deref()
            .map(|parent| g.position(parent))
            .transpose()?;
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
                            format!(
                                "{confidence} visits · low confidence{}",
                                if before.visits >= review_visit_limit(&s) {
                                    " · search limit reached"
                                } else {
                                    ""
                                }
                            )
                        };
                    }
                    if review.label.is_none()
                        && before.visits >= review_visit_limit(&s)
                        && before.candidates.is_empty()
                    {
                        review.confidence = "No suggested move · search limit reached".into();
                    }
                }
                review
            })
            .collect();
        let thinking = self.thinking.lock().unwrap().contains(id);
        let analysis = AnalysisProgress {
            completed: evaluations.len(),
            total: g.nodes.len(),
            queued: self
                .queue
                .lock()
                .unwrap()
                .iter()
                .filter(|job| job.game == id)
                .count(),
            running: self
                .running
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|job| job.game == id),
        };
        Ok(GameView {
            game: g,
            position,
            previous_position,
            selected,
            evaluations,
            reviews,
            thinking,
            analysis,
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
        a.enqueue("g", "new", 512, true);
        a.enqueue("g", "n", 64, true);
        assert_eq!(a.queue.lock().unwrap().len(), 3);
        assert_eq!(a.queue.lock().unwrap().front().unwrap().visits, 256);
        assert!(a.queue.lock().unwrap().iter().any(|job| job.node == "new"));
        a.cancel_queued("g");
        assert!(a.queue.lock().unwrap().is_empty());
    }
    #[test]
    fn review_context_is_authoritative_and_both_positions_are_prioritized() {
        let mut repo = Repository::open(Path::new(":memory:")).unwrap();
        let mut game = Game::new(Color::B, -5, "auto".into());
        let root = game.root.clone();
        let first = game.add(&root, Some(60), false).unwrap();
        let second = game.add_opponent(&first, Some(300)).unwrap();
        repo.save(&game).unwrap();
        let app = App::new(repo, "test".into());
        let view = app.view(&game.id, Some(&first)).unwrap();
        assert_eq!(view.position.board[60], 1);
        assert!(
            view.previous_position
                .unwrap()
                .board
                .iter()
                .all(|stone| *stone == 0)
        );
        assert_eq!(view.analysis.total, 3);
        app.enqueue_review(&game.id, &first, Some(&root), 256);
        app.enqueue_review(&game.id, &first, Some(&root), 64);
        assert_eq!(app.queue.lock().unwrap().len(), 2);
        assert!(
            app.queue
                .lock()
                .unwrap()
                .iter()
                .all(|job| job.visits == 256)
        );
        app.enqueue_review(&game.id, &second, Some(&first), 256);
        let queue = app.queue.lock().unwrap();
        assert_eq!(queue.front().unwrap().node, first);
        assert_eq!(queue[1].node, second);
        assert!(
            queue
                .iter()
                .find(|job| job.node == root)
                .is_some_and(|job| !job.priority)
        );
        drop(queue);
        assert_eq!(app.view(&game.id, None).unwrap().analysis.queued, 3);
        assert!(
            app.view(&game.id, Some(&root))
                .unwrap()
                .previous_position
                .is_none()
        );
    }
    #[test]
    fn old_quick_reviews_are_upgraded_without_weakening_classification_confidence() {
        let mut repo = Repository::open(Path::new(":memory:")).unwrap();
        let mut game = Game::new(Color::B, -5, "auto".into());
        let root = game.root.clone();
        let node = game.add(&root, Some(60), false).unwrap();
        repo.save(&game).unwrap();
        let mut before = evaluation();
        before.visits = 65;
        before.candidates = vec![Candidate {
            point: Some(61),
            visits: 63,
            score_black: 6.,
            winrate_black: 0.8,
            pv: vec![Some(61)],
            order: 0,
        }];
        let mut after = before.clone();
        after.score_black = -2.;
        after.winrate_black = 0.4;
        repo.save_evaluation(&game.id, &root, &before).unwrap();
        repo.save_evaluation(&game.id, &node, &after).unwrap();
        let app = App::new(repo, "test".into());
        assert!(app.view(&game.id, None).unwrap().reviews[0].label.is_none());
        app.prepare_review(&game.id).unwrap();
        app.prepare_review(&game.id).unwrap();
        assert_eq!(app.queue.lock().unwrap().len(), 1);
        assert_eq!(app.queue.lock().unwrap()[0].node, root);
        assert_eq!(app.queue.lock().unwrap()[0].visits, 256);
        before.visits = 257;
        before.candidates[0].visits = 200;
        app.repo
            .lock()
            .unwrap()
            .save_evaluation(&game.id, &root, &before)
            .unwrap();
        assert_eq!(
            app.view(&game.id, None).unwrap().reviews[0]
                .label
                .as_deref(),
            Some("Blunder")
        );
        assert_eq!(
            app.repo
                .lock()
                .unwrap()
                .settings()
                .unwrap()
                .classification
                .minimum_visits,
            64
        );
    }
    #[tokio::test]
    async fn background_review_refines_completed_searches_until_the_move_can_be_classified() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("spread-visits.json");
        std::fs::write(&model, r#"{"candidate_visit_divisor":16}"#).unwrap();
        let settings = Settings {
            executable: format!("{}/tests/fake_katago.py", env!("CARGO_MANIFEST_DIR")),
            analysis_model: model.to_string_lossy().into(),
            config: format!("{}/config/analysis.cfg", env!("CARGO_MANIFEST_DIR")),
            ..Default::default()
        };
        let mut repo = Repository::open(Path::new(":memory:")).unwrap();
        repo.save_settings(&settings).unwrap();
        let mut game = Game::new(Color::B, -5, "auto".into());
        let root = game.root.clone();
        let node = game.add(&root, Some(60), false).unwrap();
        repo.save(&game).unwrap();
        let mut before = evaluation();
        before.visits = 257;
        before.candidates[0].visits = 32;
        let mut after = before.clone();
        after.visits = 65;
        repo.save_evaluation(&game.id, &root, &before).unwrap();
        repo.save_evaluation(&game.id, &node, &after).unwrap();
        let app = App::new(repo, "test".into());
        assert!(app.view(&game.id, None).unwrap().reviews[0].label.is_none());
        app.background();
        // One preparation call must also recover the already completed 256-visit search.
        // The first refinement still has only 32 child visits and must schedule another.
        app.prepare_review(&game.id).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let view = app.view(&game.id, None).unwrap();
                if view.reviews[0].label.is_some() {
                    assert!(view.evaluations[&root].visits >= 1030);
                    assert!(view.evaluations[&root].candidates[0].visits >= 64);
                    assert_eq!(view.game.nodes.len(), 2);
                    assert_eq!(view.game.active_head, node);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("background review did not finish without another UI request");
        app.engine.shutdown().await;
    }
    #[test]
    fn review_recovers_missing_analysis_for_both_players_and_existing_branches() {
        let mut repo = Repository::open(Path::new(":memory:")).unwrap();
        let mut game = Game::new(Color::B, -5, "auto".into());
        let root = game.root.clone();
        let human = game.add(&root, Some(60), false).unwrap();
        let bot = game.add_opponent(&human, Some(300)).unwrap();
        let branch_human = game.add(&root, Some(61), true).unwrap();
        let branch_bot = game.add(&branch_human, Some(301), true).unwrap();
        repo.save(&game).unwrap();
        let app = App::new(repo, "test".into());
        app.prepare_review(&game.id).unwrap();
        app.prepare_review(&game.id).unwrap();
        let queue = app.queue.lock().unwrap();
        assert_eq!(queue.len(), 5);
        for id in [&root, &human, &branch_human] {
            assert_eq!(
                queue.iter().find(|job| &job.node == id).unwrap().visits,
                256
            );
        }
        for id in [&bot, &branch_bot] {
            assert_eq!(queue.iter().find(|job| &job.node == id).unwrap().visits, 64);
        }
        drop(queue);
        let saved = app.repo.lock().unwrap().game(&game.id).unwrap();
        assert_eq!(saved.active_head, bot);
        assert_eq!(saved.nodes.len(), 5);
    }
    #[tokio::test]
    async fn bot_reply_starts_cached_analysis_for_both_players_without_review() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("spread-visits.json");
        std::fs::write(&model, r#"{"candidate_visit_divisor":16}"#).unwrap();
        let settings = Settings {
            executable: format!("{}/tests/fake_katago.py", env!("CARGO_MANIFEST_DIR")),
            human_model: model.to_string_lossy().into(),
            analysis_model: model.to_string_lossy().into(),
            config: format!("{}/config/analysis.cfg", env!("CARGO_MANIFEST_DIR")),
            ..Default::default()
        };
        let db = dir.path().join("trainer.sqlite3");
        let mut repo = Repository::open(&db).unwrap();
        repo.save_settings(&settings).unwrap();
        let mut game = Game::new(Color::B, -5, "auto".into());
        let root = game.root.clone();
        let human = game.add(&root, Some(60), false).unwrap();
        repo.save(&game).unwrap();
        let app = App::new(repo, "test".into());
        app.background();
        app.kick(&game.id);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let view = app.view(&game.id, None).unwrap();
                if view.game.nodes.len() == 3
                    && view.evaluations.len() == 3
                    && view.reviews.iter().all(|review| review.label.is_some())
                    && !view.analysis.running
                    && view.analysis.queued == 0
                {
                    assert!(view.evaluations[&root].candidates[0].visits >= 64);
                    assert!(view.evaluations[&human].candidates[0].visits >= 64);
                    assert!(view.evaluations[&view.game.active_head].visits >= 64);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("both players must be analyzed without any Review request");
        app.engine.shutdown().await;
        let restarted = App::new(Repository::open(&db).unwrap(), "test".into());
        restarted.prepare_review(&game.id).unwrap();
        let cached = restarted.view(&game.id, None).unwrap();
        assert_eq!(cached.evaluations.len(), 3);
        assert!(cached.reviews.iter().all(|review| review.label.is_some()));
        assert_eq!(
            cached.analysis.queued, 0,
            "cached analyses must be reused after restart"
        );
    }
    #[test]
    fn review_refinement_is_bounded_and_does_not_requeue_low_confidence_forever() {
        let settings = Settings::default();
        let mut before = evaluation();
        before.visits = review_visit_limit(&settings);
        before.candidates[0].visits = 32;
        assert_eq!(review_budget(Some(&before), &settings), None);
        let mut repo = Repository::open(Path::new(":memory:")).unwrap();
        let mut game = Game::new(Color::B, -5, "auto".into());
        let root = game.root.clone();
        let node = game.add(&root, Some(60), false).unwrap();
        repo.save(&game).unwrap();
        repo.save_evaluation(&game.id, &root, &before).unwrap();
        repo.save_evaluation(&game.id, &node, &before).unwrap();
        let app = App::new(repo, "test".into());
        app.prepare_review(&game.id).unwrap();
        assert!(app.queue.lock().unwrap().is_empty());
        let view = app.view(&game.id, None).unwrap();
        assert!(view.reviews[0].label.is_none());
        assert!(view.reviews[0].confidence.contains("search limit reached"));
    }
    use std::path::Path;
}
