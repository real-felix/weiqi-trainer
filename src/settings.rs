use crate::domain::{ClassificationConfig, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub executable: String,
    pub human_model: String,
    pub analysis_model: String,
    pub config: String,
    pub quick_visits: u32,
    pub deep_visits: u32,
    pub timeout_seconds: u64,
    pub resignation_enabled: bool,
    pub resignation_winrate: f64,
    pub resignation_score: f64,
    pub resignation_min_moves: usize,
    pub resignation_confirmations: usize,
    pub classification: ClassificationConfig,
}
impl Default for Settings {
    fn default() -> Self {
        let human = std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join("Library/Application Support/KataGo/model/b18c384nbt-humanv0.bin.gz"))
            .filter(|p| p.is_file())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            executable: discover()
                .map(|p| p.to_string_lossy().into())
                .unwrap_or_default(),
            human_model: human,
            analysis_model: [
                "/opt/homebrew/share/katago/kata1-b18c384nbt-s9996604416-d4316597426.bin.gz",
                "/usr/local/share/katago/kata1-b18c384nbt-s9996604416-d4316597426.bin.gz",
            ]
            .into_iter()
            .find(|p| Path::new(p).is_file())
            .unwrap_or("")
            .into(),
            config: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("config/analysis.cfg")
                .to_string_lossy()
                .into(),
            quick_visits: 64,
            deep_visits: 256,
            timeout_seconds: 120,
            resignation_enabled: true,
            resignation_winrate: 0.005,
            resignation_score: 25.,
            resignation_min_moves: 80,
            resignation_confirmations: 3,
            classification: ClassificationConfig::default(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        for (label, p) in [
            ("Executable", &self.executable),
            ("Human model", &self.human_model),
            ("Analysis model", &self.analysis_model),
            ("Configuration", &self.config),
        ] {
            if !p.is_empty() && !Path::new(p).is_absolute() {
                return Err(format!("{label} must be an absolute path"));
            }
        }
        if !(1..=4096).contains(&self.quick_visits)
            || !(64..=20000).contains(&self.deep_visits)
            || self.deep_visits < self.quick_visits
        {
            return Err(
                "Use 1–4096 quick visits and 64–20000 deep visits, with deep ≥ quick".into(),
            );
        }
        if !(10..=600).contains(&self.timeout_seconds) {
            return Err("Timeout must be 10–600 seconds".into());
        }
        if !(0.0..=0.01).contains(&self.resignation_winrate)
            || self.resignation_score < 20.
            || self.resignation_min_moves < 60
            || self.resignation_confirmations < 3
            || self.resignation_confirmations > 10
        {
            return Err("Resignation requires ≤1% winrate, ≥20 points behind, ≥60 moves and 3–10 confirmations".into());
        }
        let c = &self.classification;
        if c.minimum_visits < 32
            || c.version.is_empty()
            || !c
                .score_thresholds
                .windows(2)
                .all(|w| w[0] > 0. && w[1] > w[0])
            || !c
                .winrate_thresholds
                .windows(2)
                .all(|w| w[0] > 0. && w[1] > w[0] && w[1] <= 1.)
        {
            return Err("Classification thresholds must be positive and increasing".into());
        }
        Ok(())
    }
}
pub fn discover() -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
        .map(|p| p.join("katago"))
        .chain([
            PathBuf::from("/opt/homebrew/bin/katago"),
            PathBuf::from("/usr/local/bin/katago"),
        ])
        .find(|p| {
            use std::os::unix::fs::PermissionsExt;
            p.is_file()
                && p.metadata()
                    .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        })
}
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("WEIQI_DATA_DIR") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Weiqi Trainer")
    } else {
        home.join(".local/share/weiqi-trainer")
    }
}
