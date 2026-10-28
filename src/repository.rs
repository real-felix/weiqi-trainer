use crate::{domain::*, engine::Evaluation, settings::Settings};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

pub struct Repository {
    conn: Connection,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub rating: f64,
    pub auto_rank: i32,
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
        if version > 1 {
            return Err("Database was created by a newer version".into());
        }
        if version == 0 {
            conn.execute_batch(&format!(
                "BEGIN IMMEDIATE; {} COMMIT;",
                include_str!("../migrations/001_initial.sql")
            ))
            .map_err(|e| e.to_string())?;
        }
        Ok(Self { conn })
    }
    pub fn profile(&self) -> Result<Profile> {
        self.conn
            .query_row(
                "SELECT rating, auto_rank, initialized FROM profile WHERE id=1",
                [],
                |r| {
                    Ok(Profile {
                        rating: r.get(0)?,
                        auto_rank: r.get(1)?,
                        initialized: r.get(2)?,
                    })
                },
            )
            .map_err(|e| e.to_string())
    }
    pub fn initialize(&mut self, rank: i32) -> Result<()> {
        if !(-20..=8).contains(&rank) {
            return Err("Unsupported starting rank".into());
        }
        self.conn.execute("UPDATE profile SET rating=?1,auto_rank=?2,initialized=1 WHERE id=1 AND initialized=0",params![rank_rating(rank),rank]).map_err(|e| e.to_string())?;
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
        s.validate()?;
        self.conn.execute("INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET data=excluded.data",[serde_json::to_string(s).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
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
            return Err("Imported records cannot update the training rating".into());
        }
        let profile = self.profile()?;
        let won = result.starts_with(&g.human.to_string());
        let after = update_rating(profile.rating, g.rank, won);
        g.result = Some(result.into());
        g.status = "finished".into();
        g.updated_at = now();
        let mut meta = g.clone();
        meta.nodes.clear();
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO rating_events VALUES(?1,?2,?3,?4,?5,'elo-k40-v1',?6)",
            params![g.id, profile.rating, after, g.rank, result, now()],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE profile SET rating=?1,auto_rank=?2,initialized=1 WHERE id=1",
            params![after, auto_rank(after, profile.auto_rank)],
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
        self.conn.query_row("SELECT before_rating,after_rating,opponent_rank,result,algorithm,created_at FROM rating_events WHERE game_id=?1",[id],|r|Ok(serde_json::json!({"before":r.get::<_,f64>(0)?,"after":r.get::<_,f64>(1)?,"opponent_rank":r.get::<_,i32>(2)?,"result":r.get::<_,String>(3)?,"algorithm":r.get::<_,String>(4)?,"created_at":r.get::<_,String>(5)?}))).optional().map(|x|x.unwrap_or(serde_json::Value::Null)).map_err(|e|e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_restart_immutable_nodes_and_exactly_one_rating() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("db");
        let mut r = Repository::open(&db).unwrap();
        let mut g = Game::new(Color::B, -5, "manual".into());
        let root = g.root.clone();
        let a = g.add(&root, Some(0), false).unwrap();
        g.add_opponent(&a, Some(1)).unwrap();
        g.add(&root, Some(19), true).unwrap();
        r.save(&g).unwrap();
        let before = r.profile().unwrap().rating;
        let done = r.finalize(&g.id, &g.active_head, "B+R").unwrap();
        r.finalize(&g.id, &root, "W+R").unwrap();
        assert_eq!(r.profile().unwrap().rating, before + 20.);
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
