use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const SIZE: usize = 19;
pub const POINTS: usize = SIZE * SIZE;
pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    B,
    W,
}
impl Color {
    pub fn other(self) -> Self {
        match self {
            Self::B => Self::W,
            Self::W => Self::B,
        }
    }
    pub fn stone(self) -> u8 {
        if self == Self::B { 1 } else { 2 }
    }
}
impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", if *self == Self::B { "B" } else { "W" })
    }
}

// A null point is a pass. The root has no color, so is distinct from a pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub parent: Option<String>,
    pub color: Option<Color>,
    pub point: Option<usize>,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub root: String,
    pub active_head: String,
    pub human: Color,
    pub rank: i32,
    pub mode: String,
    pub status: String,
    pub result: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub nodes: BTreeMap<String, Node>,
    pub resume_heads: Vec<String>,
    pub imported: bool,
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
impl Game {
    pub fn new(human: Color, rank: i32, mode: String) -> Self {
        let root = id();
        let stamp = now();
        let node = Node {
            id: root.clone(),
            parent: None,
            color: None,
            point: None,
            created_at: stamp.clone(),
        };
        Self {
            id: id(),
            root: root.clone(),
            active_head: root.clone(),
            human,
            rank,
            mode,
            status: "playing".into(),
            result: None,
            created_at: stamp.clone(),
            updated_at: stamp,
            nodes: BTreeMap::from([(root, node)]),
            resume_heads: vec![],
            imported: false,
        }
    }
    pub fn path(&self, head: &str) -> Result<Vec<&Node>> {
        let mut path = vec![];
        let mut current = Some(head);
        while let Some(key) = current {
            let n = self.nodes.get(key).ok_or("Unknown node")?;
            path.push(n);
            current = n.parent.as_deref();
            if path.len() > self.nodes.len() {
                return Err("Invalid cyclic tree".into());
            }
        }
        path.reverse();
        Ok(path)
    }
    pub fn position(&self, head: &str) -> Result<Position> {
        let mut p = Position::default();
        for n in self.path(head)? {
            if let Some(color) = n.color {
                if p.turn != color {
                    return Err("Invalid move order".into());
                }
                p.play(n.point)?;
            }
            if self.resume_heads.contains(&n.id) {
                p.passes = 0;
            }
        }
        Ok(p)
    }
    pub fn add(&mut self, parent: &str, point: Option<usize>, review: bool) -> Result<String> {
        if !review && (self.result.is_some() || self.imported) {
            return Err("A finalized game is immutable".into());
        }
        let mut p = self.position(parent)?;
        if !review && p.turn != self.human {
            return Err("It is the opponent's turn; continue from this position first".into());
        }
        if !review && p.passes >= 2 {
            return Err("Confirm scoring or resume play".into());
        }
        p.play(point)?;
        let color = p.turn.other();
        let existing = self
            .nodes
            .values()
            .find(|n| {
                n.parent.as_deref() == Some(parent) && n.point == point && n.color == Some(color)
            })
            .map(|n| n.id.clone());
        let key = existing.unwrap_or_else(|| {
            let key = id();
            self.nodes.insert(
                key.clone(),
                Node {
                    id: key.clone(),
                    parent: Some(parent.into()),
                    color: Some(color),
                    point,
                    created_at: now(),
                },
            );
            key
        });
        if !review {
            self.active_head = key.clone();
            self.status = if p.passes >= 2 { "scoring" } else { "playing" }.into();
        }
        self.updated_at = now();
        Ok(key)
    }
    pub fn add_opponent(&mut self, expected: &str, point: Option<usize>) -> Result<String> {
        if self.active_head != expected || self.result.is_some() || self.status != "playing" {
            return Err("Stale opponent response".into());
        }
        let mut p = self.position(expected)?;
        if p.turn == self.human {
            return Err("Opponent cannot move for the human".into());
        }
        let color = p.turn;
        p.play(point)?;
        let existing = self
            .nodes
            .values()
            .find(|n| {
                n.parent.as_deref() == Some(expected) && n.point == point && n.color == Some(color)
            })
            .map(|n| n.id.clone());
        let key = existing.unwrap_or_else(|| {
            let key = id();
            self.nodes.insert(
                key.clone(),
                Node {
                    id: key.clone(),
                    parent: Some(expected.into()),
                    color: Some(color),
                    point,
                    created_at: now(),
                },
            );
            key
        });
        self.active_head = key.clone();
        self.updated_at = now();
        self.status = if p.passes >= 2 { "scoring" } else { "playing" }.into();
        Ok(key)
    }
    pub fn activate(&mut self, node: &str, resume: bool) -> Result<()> {
        if self.result.is_some() || self.imported {
            return Err("A finalized game cannot resume".into());
        }
        let p = self.position(node)?;
        if resume && !self.resume_heads.iter().any(|n| n == node) {
            self.resume_heads.push(node.into());
        }
        self.active_head = node.into();
        self.status = if !resume && p.passes >= 2 {
            "scoring"
        } else {
            "playing"
        }
        .into();
        self.updated_at = now();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub board: Vec<u8>,
    pub turn: Color,
    pub passes: u32,
    pub captures: [u32; 2],
    #[serde(skip)]
    history: HashSet<(Vec<u8>, Color)>,
}
impl Default for Position {
    fn default() -> Self {
        let board = vec![0; POINTS];
        Self {
            history: HashSet::from([(board.clone(), Color::B)]),
            board,
            turn: Color::B,
            passes: 0,
            captures: [0, 0],
        }
    }
}
pub fn neighbors(p: usize) -> impl Iterator<Item = usize> {
    let mut n = Vec::with_capacity(4);
    if p >= SIZE {
        n.push(p - SIZE);
    }
    if p + SIZE < POINTS {
        n.push(p + SIZE);
    }
    if !p.is_multiple_of(SIZE) {
        n.push(p - 1);
    }
    if p % SIZE < SIZE - 1 {
        n.push(p + 1);
    }
    n.into_iter()
}
pub fn group(board: &[u8], start: usize) -> (Vec<usize>, bool) {
    let mut seen = HashSet::from([start]);
    let mut stack = vec![start];
    let mut liberty = false;
    while let Some(p) = stack.pop() {
        for n in neighbors(p) {
            if board[n] == 0 {
                liberty = true;
            }
            if board[n] == board[start] && seen.insert(n) {
                stack.push(n);
            }
        }
    }
    (seen.into_iter().collect(), liberty)
}
impl Position {
    pub fn play(&mut self, point: Option<usize>) -> Result<()> {
        let Some(p) = point else {
            self.passes += 1;
            self.turn = self.turn.other();
            self.history.insert((self.board.clone(), self.turn));
            return Ok(());
        };
        if p >= POINTS {
            return Err("Move is outside the board".into());
        }
        if self.board[p] != 0 {
            return Err("That intersection is occupied".into());
        }
        let mut board = self.board.clone();
        board[p] = self.turn.stone();
        let mut captured = 0;
        for n in neighbors(p) {
            if board[n] == self.turn.other().stone() {
                let (stones, liberty) = group(&board, n);
                if !liberty {
                    captured += stones.len() as u32;
                    for s in stones {
                        board[s] = 0;
                    }
                }
            }
        }
        if !group(&board, p).1 {
            return Err("Suicide is not allowed under AGA rules".into());
        }
        let state = (board.clone(), self.turn.other());
        if self.history.contains(&state) {
            return Err("This move repeats a previous situation (superko)".into());
        }
        self.history.insert(state);
        self.board = board;
        self.captures[if self.turn == Color::B { 0 } else { 1 }] += captured;
        self.turn = self.turn.other();
        self.passes = 0;
        Ok(())
    }
    pub fn score(&self, dead: &[usize]) -> Result<Score> {
        let mut board = self.board.clone();
        let marked: HashSet<usize> = dead.iter().copied().collect();
        for &p in dead {
            if p >= POINTS || self.board[p] == 0 {
                return Err("Dead-stone selection must contain occupied intersections".into());
            }
            if group(&self.board, p).0.iter().any(|s| !marked.contains(s)) {
                return Err("Mark the entire dead group".into());
            }
            board[p] = 0;
        }
        let mut black = board.iter().filter(|&&s| s == 1).count() as f64;
        let mut white = board.iter().filter(|&&s| s == 2).count() as f64 + 7.5;
        let mut seen = HashSet::new();
        for p in 0..POINTS {
            if board[p] != 0 || seen.contains(&p) {
                continue;
            }
            let mut stack = vec![p];
            seen.insert(p);
            let mut count = 0;
            let mut border = HashSet::new();
            while let Some(s) = stack.pop() {
                count += 1;
                for n in neighbors(s) {
                    if board[n] == 0 {
                        if seen.insert(n) {
                            stack.push(n);
                        }
                    } else {
                        border.insert(board[n]);
                    }
                }
            }
            if border.len() == 1 {
                if border.contains(&1) {
                    black += count as f64;
                } else {
                    white += count as f64;
                }
            }
        }
        Ok(Score {
            black,
            white,
            result: format!(
                "{}+{:.1}",
                if black > white { "B" } else { "W" },
                (black - white).abs()
            ),
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub black: f64,
    pub white: f64,
    pub result: String,
}

pub fn rank_label(rank: i32) -> String {
    if rank < 0 {
        format!("{}k", -rank)
    } else {
        format!("{}d", rank + 1)
    }
}
pub fn rank_rating(rank: i32) -> f64 {
    2000. + rank as f64 * 100.
}
pub fn auto_rank(rating: f64, previous: i32) -> i32 {
    let target = ((rating - 2000.) / 100.).round() as i32;
    if (rating - rank_rating(previous)).abs() < 65. {
        previous
    } else {
        target.clamp(-20, 8)
    }
}
pub fn update_rating(rating: f64, rank: i32, won: bool) -> f64 {
    let expected = 1. / (1. + 10_f64.powf((rank_rating(rank) - rating) / 400.));
    rating + 40. * (if won { 1. } else { 0. } - expected)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationConfig {
    pub version: String,
    pub minimum_visits: u32,
    pub score_thresholds: [f64; 3],
    pub winrate_thresholds: [f64; 3],
}
impl Default for ClassificationConfig {
    fn default() -> Self {
        Self {
            version: "heuristic-v1".into(),
            minimum_visits: 64,
            score_thresholds: [1.5, 4., 8.],
            winrate_thresholds: [0.04, 0.10, 0.20],
        }
    }
}
pub fn classify(
    score_loss: f64,
    win_loss: f64,
    rank: i32,
    preferred: bool,
    visits: u32,
    c: &ClassificationConfig,
) -> Option<&'static str> {
    if visits < c.minimum_visits {
        return None;
    }
    if preferred
        && visits >= c.minimum_visits.saturating_mul(4)
        && score_loss < 0.5
        && win_loss < 0.01
    {
        return Some("AI move");
    }
    let factor = 1. + (-rank).max(0) as f64 * 0.025;
    for i in (0..3).rev() {
        if score_loss >= c.score_thresholds[i] * factor
            || win_loss >= c.winrate_thresholds[i] * factor
        {
            return Some(["Inaccuracy", "Mistake", "Blunder"][i]);
        }
    }
    Some("Good move")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_suicide_and_bounds() {
        let mut p = Position::default();
        for m in [Some(1), Some(0), Some(19)] {
            p.play(m).unwrap();
        }
        assert_eq!(p.board[0], 0);
        assert_eq!(p.captures, [1, 0]);
        assert!(p.play(Some(0)).unwrap_err().contains("Suicide"));
        assert!(p.play(Some(POINTS)).is_err());
        assert!(p.play(Some(1)).is_err());
    }
    #[test]
    fn ko_and_situational_history() {
        let mut p = Position::default();
        // White at B2 with one liberty C2; black capture C2 has just B2 as liberty.
        for i in [1, 19, 39] {
            p.board[i] = 1;
        }
        for i in [20, 3, 23, 41] {
            p.board[i] = 2;
        }
        // Use a simple symmetric ko in the interior.
        p.board = vec![0; POINTS];
        for i in [20, 38, 58] {
            p.board[i] = 1;
        }
        for i in [39, 41, 40 + 19, 40 - 19] {
            p.board[i] = 2;
        }
        p.history = HashSet::from([(p.board.clone(), Color::B)]);
        p.play(Some(40)).unwrap();
        assert_eq!(p.board[39], 0);
        assert!(p.play(Some(39)).unwrap_err().contains("superko"));
        // The same stones with a different player to move is a distinct AGA situation.
        let mut q = Position::default();
        let mut next = q.clone();
        next.play(Some(0)).unwrap();
        q.history.insert((next.board.clone(), Color::B));
        assert!(q.play(Some(0)).is_ok());
    }
    #[test]
    fn passes_scoring_and_dead_group_validation() {
        let mut p = Position::default();
        p.play(None).unwrap();
        p.play(None).unwrap();
        assert_eq!(p.passes, 2);
        assert_eq!(p.score(&[]).unwrap().result, "W+7.5");
        p.passes = 0;
        p.play(Some(0)).unwrap();
        assert_eq!(p.turn, Color::W);
        assert_eq!(p.score(&[]).unwrap().black, 361.);
        p.board[1] = 1;
        assert!(p.score(&[0]).is_err());
        assert_eq!(p.score(&[0, 1]).unwrap().result, "W+7.5");
    }
    #[test]
    fn branches_do_not_change_active_result() {
        let mut g = Game::new(Color::B, -5, "auto".into());
        let root = g.root.clone();
        let a = g.add(&root, Some(0), false).unwrap();
        let b = g.add_opponent(&a, Some(1)).unwrap();
        let branch = g.add(&root, Some(19), true).unwrap();
        assert_eq!(g.active_head, b);
        g.activate(&branch, false).unwrap();
        assert_eq!(g.nodes.len(), 4);
        g.result = Some("B+R".into());
        assert!(g.activate(&root, false).is_err());
        assert!(g.add(&root, Some(20), false).is_err());
        assert!(g.add(&root, Some(20), true).is_ok());
        assert_eq!(g.result.as_deref(), Some("B+R"));
    }
    #[test]
    fn rating_hysteresis_and_labels() {
        assert_eq!(rank_label(-1), "1k");
        assert_eq!(rank_label(0), "1d");
        assert_eq!(auto_rank(1555., -5), -5);
        assert_eq!(auto_rank(1566., -5), -4);
        assert_eq!(update_rating(1500., -5, true), 1520.);
        let c = ClassificationConfig::default();
        assert_eq!(classify(9., 0.3, -5, false, 64, &c), Some("Blunder"));
        assert_eq!(classify(0., 0., -5, true, 64, &c), Some("Good move"));
        assert_eq!(classify(0., 0., -5, true, 256, &c), Some("AI move"));
        assert_eq!(classify(0., 0., -5, true, 1, &c), None);
    }
    #[test]
    fn captures_multiple_adjacent_groups_without_mutating_rejected_positions() {
        let mut p = Position::default();
        // White stones A2 and B1 are independently captured by black A1.
        for i in [2, 20, 38] {
            p.board[i] = 1;
        }
        for i in [1, 19] {
            p.board[i] = 2;
        }
        p.play(Some(0)).unwrap();
        assert_eq!(p.captures[0], 2);
        assert_eq!(p.board[1], 0);
        assert_eq!(p.board[19], 0);
        let before = p.board.clone();
        let turn = p.turn;
        assert!(p.play(Some(0)).is_err());
        assert_eq!(p.board, before);
        assert_eq!(p.turn, turn);
    }
    #[test]
    fn area_score_counts_neutral_regions_and_dead_groups() {
        let mut p = Position::default();
        p.board[0] = 1;
        p.board[360] = 2;
        // Shared open regions have both colors at their boundary and belong to neither.
        let s = p.score(&[]).unwrap();
        assert_eq!(s.black, 1.);
        assert_eq!(s.white, 8.5);
        let s = p.score(&[360]).unwrap();
        assert_eq!(s.black, 361.);
        assert_eq!(s.white, 7.5);
    }
    #[test]
    fn scoring_resumption_preserves_passes_and_original_branches() {
        let mut g = Game::new(Color::B, -5, "auto".into());
        assert!(g.position(&g.root).unwrap().board.iter().all(|&s| s == 0));
        let root = g.root.clone();
        let first = g.add(&root, None, false).unwrap();
        let second = g.add_opponent(&first, None).unwrap();
        assert_eq!(g.status, "scoring");
        assert!(g.add(&second, Some(0), false).is_err());
        g.activate(&second, true).unwrap();
        assert_eq!(g.position(&second).unwrap().passes, 0);
        let m = g.add(&second, Some(0), false).unwrap();
        assert_eq!(g.path(&m).unwrap().len(), 4);
        assert!(g.nodes.contains_key(&first));
        assert_eq!(g.resume_heads, vec![second]);
    }
}
