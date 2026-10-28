//! Derived Home images. Move nodes remain the source of truth; this cache is disposable.
use crate::domain::{Game, Result};
use sha2::{Digest, Sha256};
use std::{collections::VecDeque, fmt::Write, sync::Arc};

const CACHE_CAPACITY: usize = 128;

pub struct Preview {
    pub svg: String,
    pub etag: String,
}
impl Preview {
    pub fn render(game: &Game, head: &str) -> Result<Self> {
        let position = game.position(head)?;
        let mut svg = String::with_capacity(20_000);
        svg.push_str(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 400"><defs><linearGradient id="wood" x2=".8" y2="1"><stop stop-color="#e6c991"/><stop offset="1" stop-color="#d9b579"/></linearGradient><radialGradient id="black" cx="32%" cy="25%" r="75%"><stop stop-color="#50514c"/><stop offset="1" stop-color="#171914"/></radialGradient><radialGradient id="white" cx="32%" cy="25%" r="75%"><stop stop-color="#fffef9"/><stop offset="1" stop-color="#d8d6cb"/></radialGradient><circle id="b" r="9.1" fill="url(#black)" stroke="#20211c" stroke-width=".5"/><circle id="w" r="9.1" fill="url(#white)" stroke="#c8c4b8" stroke-width=".5"/></defs><rect width="400" height="400" rx="5" fill="url(#wood)"/><path d=""##);
        for i in 0..19 {
            let p = 20 + i * 20;
            write!(svg, "M20 {p}H380M{p} 20V380").unwrap();
        }
        svg.push_str(r##"" fill="none" stroke="#6e593a" stroke-width=".65" opacity=".75"/>"##);
        for y in [3, 9, 15] {
            for x in [3, 9, 15] {
                write!(
                    svg,
                    r##"<circle cx="{}" cy="{}" r="2.1" fill="#59492f"/>"##,
                    20 + x * 20,
                    20 + y * 20
                )
                .unwrap();
            }
        }
        for (point, stone) in position.board.iter().enumerate() {
            let id = match stone {
                1 => 'b',
                2 => 'w',
                _ => continue,
            };
            write!(
                svg,
                r##"<use href="#{id}" x="{}" y="{}"/>"##,
                20 + point % 19 * 20,
                20 + point / 19 * 20
            )
            .unwrap();
        }
        if let Some(point) = game.nodes.get(head).and_then(|node| node.point) {
            let stroke = if position.board[point] == 1 {
                "#faf8ee"
            } else {
                "#50574b"
            };
            write!(svg, r##"<circle cx="{}" cy="{}" r="2.6" fill="none" stroke="{stroke}" stroke-width="1.1"/>"##, 20 + point % 19 * 20, 20 + point / 19 * 20).unwrap();
        }
        svg.push_str("</svg>");
        let etag = format!("\"{:x}\"", Sha256::digest(svg.as_bytes()));
        Ok(Self { svg, etag })
    }
}

#[derive(Default)]
pub struct PreviewCache {
    entries: VecDeque<(String, String, Arc<Preview>)>,
}
impl PreviewCache {
    pub fn get(&mut self, game: &str, head: &str) -> Option<Arc<Preview>> {
        let index = self
            .entries
            .iter()
            .position(|(g, h, _)| g == game && h == head)?;
        let entry = self.entries.remove(index)?;
        let preview = entry.2.clone();
        self.entries.push_back(entry);
        Some(preview)
    }
    pub fn insert(&mut self, game: &str, head: &str, preview: Arc<Preview>) {
        self.entries.retain(|(g, h, _)| g != game || h != head);
        if self.entries.len() == CACHE_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back((game.into(), head.into(), preview));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Color;

    #[test]
    fn previews_apply_captures_and_use_the_requested_played_head() {
        let mut game = Game::new(Color::B, -5, "auto".into());
        let root = game.root.clone();
        let first = game.add(&root, Some(1), true).unwrap();
        let second = game.add(&first, Some(0), true).unwrap();
        let head = game.add(&second, Some(19), true).unwrap();
        let preview = Preview::render(&game, &head).unwrap();
        assert_eq!(preview.svg.matches("<use href=\"#b\"").count(), 2);
        assert!(!preview.svg.contains("<use href=\"#w\""));
        let old = Preview::render(&game, &second).unwrap();
        assert!(old.svg.contains("<use href=\"#w\" x=\"20\" y=\"20\""));
        game.add(&root, Some(360), true).unwrap();
        assert_eq!(Preview::render(&game, &head).unwrap().etag, preview.etag);
        assert!(Preview::render(&game, "missing").is_err());
        assert!(!Preview::render(&game, &root).unwrap().svg.contains("<use "));
    }

    #[test]
    fn cache_reuses_images_and_bounds_memory_with_recent_use_eviction() {
        let game = Game::new(Color::B, -5, "auto".into());
        let preview = Arc::new(Preview::render(&game, &game.root).unwrap());
        let mut cache = PreviewCache::default();
        for i in 0..CACHE_CAPACITY {
            cache.insert(&i.to_string(), "head", preview.clone());
        }
        assert!(Arc::ptr_eq(&preview, &cache.get("0", "head").unwrap()));
        assert!(cache.get("0", "different-head").is_none());
        cache.insert("new", "head", preview.clone());
        assert!(cache.get("1", "head").is_none());
        assert!(cache.get("0", "head").is_some());
        assert_eq!(cache.entries.len(), CACHE_CAPACITY);
    }

    #[test]
    #[ignore = "requires WEIQI_PREVIEW_GAME_JSON pointing to a read-only game-view snapshot"]
    fn benchmark_saved_game_preview() {
        let path = std::env::var("WEIQI_PREVIEW_GAME_JSON").unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let game: Game = serde_json::from_value(value["game"].clone()).unwrap();
        let mut samples = vec![];
        for _ in 0..50 {
            let start = std::time::Instant::now();
            std::hint::black_box(Preview::render(&game, &game.active_head).unwrap());
            samples.push(start.elapsed());
        }
        samples.sort();
        let preview = Arc::new(Preview::render(&game, &game.active_head).unwrap());
        let bytes = preview.svg.len();
        let mut cache = PreviewCache::default();
        cache.insert(&game.id, &game.active_head, preview);
        let start = std::time::Instant::now();
        for _ in 0..10_000 {
            std::hint::black_box(cache.get(&game.id, &game.active_head).unwrap());
        }
        println!(
            "{} nodes: cold median {:?}, SVG {} bytes, warm lookup mean {:?}",
            game.nodes.len(),
            samples[25],
            bytes,
            start.elapsed() / 10_000
        );
    }
}
