use crate::{domain::*, engine::Evaluation, settings::Settings};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

pub struct Repository {
    conn: Connection,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub level: i32,
    pub initialized: bool,
}
impl Repository {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > 2 {
            return Err("Database was created by a newer version".into());
        }
        if version == 0 {
            conn.execute_batch(&format!(
                "BEGIN IMMEDIATE; {} COMMIT;",
                include_str!("../migrations/001_initial.sql")
            ))
            .map_err(|e| e.to_string())?;
        }
        if version < 2 {
            conn.execute_batch(&format!(
                "BEGIN IMMEDIATE; {} COMMIT;",
                include_str!("../migrations/002_rank_ladder.sql")
            ))
            .map_err(|e| e.to_string())?;
        }
        Ok(Self { conn })
    }
    pub fn profile(&self) -> Result<Profile> {
        self.conn
            .query_row(
                "SELECT level, initialized FROM profile WHERE id=1",
                [],
                |r| {
                    Ok(Profile {
                        level: r.get(0)?,
                        initialized: r.get(1)?,
                    })
                },
            )
            .map_err(|e| e.to_string())
    }
    pub fn initialize(&mut self, rank: i32) -> Result<()> {
        if !(-20..=8).contains(&rank) {
            return Err("Unsupported starting rank".into());
        }
        self.conn
            .execute(
                "UPDATE profile SET level=?1,initialized=1 WHERE id=1 AND initialized=0",
                [rank * 2],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn settings(&self) -> Result<Settings> {
        let s: Option<String> = self
            .conn
            .query_row("SELECT data FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        s.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
            .unwrap_or_else(|| Ok(Settings::default()))
    }
    pub fn save_settings(&mut self, s: &Settings) -> Result<()> {
        let mut s = s.clone();
        s.player_name = s.player_name.trim().to_string();
        s.validate()?;
        self.conn.execute("INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET data=excluded.data",[serde_json::to_string(&s).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<Game>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM games")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut games = vec![];
        for row in rows {
            games.push(
                serde_json::from_str::<Game>(&row.map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            );
        }
        games.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(games)
    }
    pub fn count(&self) -> Result<usize> {
        self.conn
            .query_row(
                "SELECT count(*) FROM games WHERE json_extract(data,'$.imported')=0",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }
    pub fn game(&self, id: &str) -> Result<Game> {
        let data: String = self
            .conn
            .query_row("SELECT data FROM games WHERE id=?1", [id], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Game not found")?;
        let mut g: Game = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM nodes WHERE game_id=?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let n: Node = serde_json::from_str(&row.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            g.nodes.insert(n.id.clone(), n);
        }
        Ok(g)
    }
    pub fn save(&mut self, g: &Game) -> Result<()> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        // Persistence enforces final-result/active-head immutability as well as domain code.
        let old: Option<String> = tx
            .query_row("SELECT data FROM games WHERE id=?1", [&g.id], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(old) = old {
            let old: Game = serde_json::from_str(&old).map_err(|e| e.to_string())?;
            if old.rank != g.rank
                || old.half_rank != g.half_rank
                || old.human != g.human
                || old.komi != g.komi
                || old.initial_stones != g.initial_stones
            {
                return Err("Game starting conditions are immutable".into());
            }
            if old.result.is_some()
                && (old.result != g.result
                    || old.active_head != g.active_head
                    || old.status != g.status
                    || old.resume_heads != g.resume_heads)
            {
                return Err("Finalized result is immutable".into());
            }
        }
        let mut meta = g.clone();
        meta.nodes.clear();
        tx.execute(
            "INSERT INTO games VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
            params![
                g.id,
                serde_json::to_string(&meta).map_err(|e| e.to_string())?
            ],
        )
        .map_err(|e| e.to_string())?;
        // Parents precede children regardless of UUID order.
        let mut nodes: Vec<_> = g.nodes.values().collect();
        nodes.sort_by_key(|n| g.path(&n.id).map(|p| p.len()).unwrap_or(usize::MAX));
        for n in nodes {
            let data = serde_json::to_string(n).map_err(|e| e.to_string())?;
            let old: Option<String> = tx
                .query_row("SELECT data FROM nodes WHERE id=?1", [&n.id], |r| r.get(0))
                .optional()
                .map_err(|e| e.to_string())?;
            if old.as_ref().is_some_and(|old| old != &data) {
                return Err("Move nodes are immutable".into());
            }
            tx.execute(
                "INSERT OR IGNORE INTO nodes VALUES(?1,?2,?3,?4)",
                params![n.id, g.id, n.parent, data],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn finalize(&mut self, id: &str, head: &str, result: &str) -> Result<Game> {
        let mut g = self.game(id)?;
        // Retries are idempotent and cannot create a second event or change an existing result.
        if g.result.is_some() {
            return Ok(g);
        }
        if g.active_head != head {
            return Err("Position changed; review the current active position".into());
        }
        if g.imported {
            return Err("Imported records cannot update the training level".into());
        }
        let profile = self.profile()?;
        let won = result.starts_with(&g.human.to_string());
        let after = next_level(g.level(), won);
        g.result = Some(result.into());
        g.status = "finished".into();
        g.updated_at = now();
        let mut meta = g.clone();
        meta.nodes.clear();
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO rank_events VALUES(?1,?2,?3,?4,?5,'rank-half-step-v1',?6)",
            params![g.id, profile.level, after, g.level(), result, now()],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE profile SET level=?1,initialized=1 WHERE id=1",
            [after],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE games SET data=?1 WHERE id=?2",
            params![
                serde_json::to_string(&meta).map_err(|e| e.to_string())?,
                g.id
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(g)
    }
    pub fn evaluations(&self, id: &str) -> Result<BTreeMap<String, Evaluation>> {
        let mut stmt = self
            .conn
            .prepare("SELECT node_id,data FROM analysis WHERE game_id=?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut result = BTreeMap::new();
        for row in rows {
            let (id, data) = row.map_err(|e| e.to_string())?;
            result.insert(id, serde_json::from_str(&data).map_err(|e| e.to_string())?);
        }
        Ok(result)
    }
    pub fn save_evaluation(&mut self, game: &str, node: &str, e: &Evaluation) -> Result<()> {
        self.conn.execute("INSERT INTO analysis VALUES(?1,?2,?3,?4) ON CONFLICT(game_id,node_id) DO UPDATE SET visits=excluded.visits,data=excluded.data WHERE excluded.visits>=analysis.visits",params![game,node,e.visits,serde_json::to_string(e).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn audit(&self, id: &str) -> Result<serde_json::Value> {
        let rank = self.conn.query_row("SELECT before_level,after_level,opponent_level,result,algorithm,created_at FROM rank_events WHERE game_id=?1",[id],|r|Ok(serde_json::json!({"before_level":r.get::<_,i32>(0)?,"after_level":r.get::<_,i32>(1)?,"opponent_level":r.get::<_,i32>(2)?,"result":r.get::<_,String>(3)?,"algorithm":r.get::<_,String>(4)?,"created_at":r.get::<_,String>(5)?}))).optional().map_err(|e|e.to_string())?;
        if let Some(rank) = rank {
            return Ok(rank);
        }
        self.conn.query_row("SELECT before_rating,after_rating,opponent_rank,result,algorithm,created_at FROM rating_events WHERE game_id=?1",[id],|r|Ok(serde_json::json!({"before":r.get::<_,f64>(0)?,"after":r.get::<_,f64>(1)?,"opponent_rank":r.get::<_,i32>(2)?,"result":r.get::<_,String>(3)?,"algorithm":r.get::<_,String>(4)?,"created_at":r.get::<_,String>(5)?}))).optional().map(|x|x.unwrap_or(serde_json::Value::Null)).map_err(|e|e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn player_name_defaults_for_old_settings_and_persists_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.sqlite3");
        let mut repo = Repository::open(&path).unwrap();
        repo.conn
            .execute(
                "INSERT INTO settings VALUES(1,?1)",
                [r#"{"executable":"/local/katago"}"#],
            )
            .unwrap();
        let mut settings = repo.settings().unwrap();
        assert!(settings.player_name.is_empty());
        settings.player_name = "  Zoë [Fox]  ".into();
        repo.save_settings(&settings).unwrap();
        settings.player_name = "a".repeat(81);
        assert!(repo.save_settings(&settings).is_err());
        settings.player_name = "one\ntwo".into();
        assert!(repo.save_settings(&settings).is_err());
        drop(repo);
        let settings = Repository::open(&path).unwrap().settings().unwrap();
        assert_eq!(settings.player_name, "Zoë [Fox]");
        assert_eq!(settings.executable, "/local/katago");
    }
    #[test]
    fn elo_migration_preserves_the_existing_rank_games_and_historical_audits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite3");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        conn.execute(
            "UPDATE profile SET rating=1823.4,auto_rank=-2,initialized=1",
            [],
        )
        .unwrap();
        let mut game = Game::new(Color::B, -5, "auto".into());
        game.result = Some("W+R".into());
        game.status = "finished".into();
        let mut meta = serde_json::to_value(&game).unwrap();
        for field in ["half_rank", "komi", "initial_stones"] {
            meta.as_object_mut().unwrap().remove(field);
        }
        meta["nodes"] = serde_json::json!({});
        conn.execute(
            "INSERT INTO games VALUES(?1,?2)",
            params![game.id, meta.to_string()],
        )
        .unwrap();
        let root = &game.nodes[&game.root];
        conn.execute(
            "INSERT INTO nodes VALUES(?1,?2,NULL,?3)",
            params![root.id, game.id, serde_json::to_string(root).unwrap()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO rating_events VALUES(?1,1500,1480,-5,'W+R','elo-k40-v1','legacy')",
            [&game.id],
        )
        .unwrap();
        drop(conn);
        let repo = Repository::open(&path).unwrap();
        assert_eq!(repo.profile().unwrap().level, -4);
        assert!(repo.profile().unwrap().initialized);
        let saved = repo.game(&game.id).unwrap();
        assert_eq!(saved.komi, 7.5);
        assert!(saved.initial_stones.is_empty());
        assert_eq!(saved.result, game.result);
        assert_eq!(repo.audit(&game.id).unwrap()["algorithm"], "elo-k40-v1");
        assert_eq!(repo.audit(&game.id).unwrap()["after"], 1480.);
        drop(repo);
        assert_eq!(
            Repository::open(&path).unwrap().profile().unwrap().level,
            -4
        );
    }
    #[test]
    fn wins_and_losses_change_one_half_step_once_and_manual_results_use_the_played_level() {
        let mut repo = Repository::open(Path::new(":memory:")).unwrap();
        for (level, human, result, expected) in [
            (-10, Color::B, "B+R", -9),
            (-9, Color::W, "W+0.5", -8),
            (-8, Color::B, "W+R", -9),
            (-9, Color::B, "W+R", -10),
            (-2, Color::B, "B+R", -1),
        ] {
            let game = Game::at_level(human, level, "manual".into()).unwrap();
            repo.save(&game).unwrap();
            repo.finalize(&game.id, &game.active_head, result).unwrap();
            let audit = repo.audit(&game.id).unwrap();
            repo.finalize(&game.id, &game.root, "B+100.5").unwrap();
            assert_eq!(repo.profile().unwrap().level, expected);
            assert_eq!(repo.audit(&game.id).unwrap(), audit);
            assert_eq!(audit["opponent_level"], level);
            assert_eq!(audit["algorithm"], "rank-half-step-v1");
        }
    }
    #[test]
    fn game_starting_conditions_are_immutable_even_before_finalization() {
        let mut repo = Repository::open(Path::new(":memory:")).unwrap();
        let mut game = Game::at_level(Color::B, -9, "auto".into()).unwrap();
        repo.save(&game).unwrap();
        game.komi = 7.5;
        assert!(repo.save(&game).is_err());
        game.komi = 0.5;
        game.initial_stones.clear();
        assert!(repo.save(&game).is_err());
    }
    #[test]
    fn migration_restart_immutable_nodes_and_exactly_one_rank_update() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("db");
        let mut r = Repository::open(&db).unwrap();
        let mut g = Game::new(Color::B, -5, "manual".into());
        let root = g.root.clone();
        let a = g.add(&root, Some(0), false).unwrap();
        g.add_opponent(&a, Some(1)).unwrap();
        g.add(&root, Some(19), true).unwrap();
        r.save(&g).unwrap();
        let before = r.profile().unwrap().level;
        let done = r.finalize(&g.id, &g.active_head, "B+R").unwrap();
        r.finalize(&g.id, &root, "W+R").unwrap();
        assert_eq!(r.profile().unwrap().level, before + 1);
        drop(r);
        let mut r = Repository::open(&db).unwrap();
        let mut loaded = r.game(&g.id).unwrap();
        assert_eq!(loaded.nodes.len(), 4);
        assert_eq!(loaded.result, done.result);
        loaded.add(&root, Some(20), true).unwrap();
        r.save(&loaded).unwrap();
        loaded.result = Some("W+R".into());
        assert!(r.save(&loaded).is_err());
        loaded = r.game(&g.id).unwrap();
        loaded.nodes.get_mut(&root).unwrap().point = Some(18);
        assert!(r.save(&loaded).is_err());
        assert_eq!(r.game(&g.id).unwrap().nodes.len(), 5);
    }
}
