use crate::{domain::*, settings::Settings};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{Mutex, Notify, broadcast},
    time::timeout,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub point: Option<usize>,
    pub visits: u32,
    pub score_black: f64,
    pub winrate_black: f64,
    pub pv: Vec<Option<usize>>,
    pub order: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub score_black: f64,
    pub winrate_black: f64,
    pub visits: u32,
    pub candidates: Vec<Candidate>,
    pub ownership: Vec<f64>,
    pub engine: String,
    pub model: String,
    pub config: String,
    pub parameters: Value,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct WorkerStatus {
    pub state: String,
    pub detail: String,
    pub error: Option<String>,
    pub visits: u32,
    pub pid: Option<u32>,
    pub version: String,
    pub recent_log: VecDeque<String>,
    pub supported_ranks: Vec<i32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub human: WorkerStatus,
    pub analysis: WorkerStatus,
}
pub struct Engine {
    human: Mutex<Worker>,
    strong: Mutex<Worker>,
    pub status: Arc<StdMutex<Status>>,
    pub events: broadcast::Sender<String>,
    cancel: Arc<Notify>,
    cancel_all: Arc<Notify>,
}
struct Worker {
    human: bool,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<Lines<BufReader<ChildStdout>>>,
    status: Arc<StdMutex<Status>>,
    events: broadcast::Sender<String>,
    signature: String,
    version: String,
    models: Value,
    cancel: Arc<Notify>,
    cancel_all: Arc<Notify>,
}
impl Worker {
    fn update(&self, state: &str, detail: &str, error: Option<String>, visits: u32) {
        let mut s = self.status.lock().unwrap();
        let s = if self.human {
            &mut s.human
        } else {
            &mut s.analysis
        };
        s.state = state.into();
        s.detail = detail.into();
        s.error = error;
        s.visits = visits;
        let _ = self.events.send("status".into());
    }
    async fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill().await;
            let _ = c.wait().await;
        }
        self.stdin = None;
        self.stdout = None;
        self.signature.clear();
        let mut status = self.status.lock().unwrap();
        let s = if self.human {
            &mut status.human
        } else {
            &mut status.analysis
        };
        s.pid = None;
        s.supported_ranks.clear();
    }
    async fn start(&mut self, s: &Settings) -> Result<()> {
        let signature = serde_json::to_string(s).map_err(|e| e.to_string())?;
        if self.child.is_some() && self.signature == signature {
            return Ok(());
        }
        self.stop().await;
        self.update("locating executable", "Checking local paths", None, 0);
        for (label, p) in [
            ("KataGo executable", s.executable.as_str()),
            ("Analysis model", s.analysis_model.as_str()),
            ("Analysis configuration", s.config.as_str()),
        ]
        .into_iter()
        .chain(
            self.human
                .then_some(("Human SL model", s.human_model.as_str())),
        ) {
            if p.is_empty() || !Path::new(p).is_file() {
                return Err(format!(
                    "{label} not found. Set its absolute path in Settings."
                ));
            }
        }
        self.update("starting", "Validating KataGo version and backend", None, 0);
        let output = timeout(
            Duration::from_secs(10),
            Command::new(&s.executable)
                .arg("version")
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| "KataGo version check timed out")?
        .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
        self.version = String::from_utf8_lossy(&output.stdout).trim().into();
        let mut command = Command::new(&s.executable);
        command.args(["analysis","-config",&s.config,"-model",&s.analysis_model,"-override-config","reportAnalysisWinratesAs=BLACK,numAnalysisThreads=1,numSearchThreadsPerAnalysisThread=2,nnMaxBatchSize=4,nnCacheSizePowerOfTwo=18,nnMutexPoolSizePowerOfTwo=14,logToStderr=true", "-quit-without-waiting"]);
        if self.human {
            command.args(["-human-model", &s.human_model]);
        }
        let mut child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| e.to_string())?;
        self.stdin = child.stdin.take();
        self.stdout = child.stdout.take().map(|o| BufReader::new(o).lines());
        let stderr = child.stderr.take().unwrap();
        {
            let mut status = self.status.lock().unwrap();
            let st = if self.human {
                &mut status.human
            } else {
                &mut status.analysis
            };
            st.pid = child.id();
            st.version = self.version.clone();
            st.recent_log.clear();
        }
        let status = self.status.clone();
        let human = self.human;
        let pid = child.id();
        let events = self.events.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut status = status.lock().unwrap();
                let st = if human {
                    &mut status.human
                } else {
                    &mut status.analysis
                };
                if st.recent_log.len() >= 24 {
                    st.recent_log.pop_front();
                }
                st.recent_log.push_back(line.chars().take(1500).collect());
            }
            let mut status = status.lock().unwrap();
            let st = if human {
                &mut status.human
            } else {
                &mut status.analysis
            };
            if st.pid == pid && st.state != "error" {
                st.state = "error".into();
                st.detail = "Process exited".into();
                st.error =
                    Some("KataGo exited. Expand the recent log, then retry the worker.".into());
                let _ = events.send("status".into());
            }
        });
        self.child = Some(child);
        self.signature = signature;
        self.update(
            "loading model",
            "Loading local model; first startup may take a moment",
            None,
            0,
        );
        let probe = json!({"id":id(),"action":"query_models"});
        self.stdin
            .as_mut()
            .unwrap()
            .write_all(format!("{probe}\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        self.stdin
            .as_mut()
            .unwrap()
            .flush()
            .await
            .map_err(|e| e.to_string())?;
        let line = timeout(
            Duration::from_secs(s.timeout_seconds),
            self.stdout.as_mut().unwrap().next_line(),
        )
        .await
        .map_err(|_| "KataGo model validation timed out")?
        .map_err(|e| e.to_string())?
        .ok_or("KataGo exited during model validation")?;
        let info: Value =
            serde_json::from_str(&line).map_err(|e| format!("Invalid KataGo JSON: {e}"))?;
        if let Some(error) = info.get("error") {
            return Err(format!("KataGo: {error}"));
        }
        let models = info["models"]
            .as_array()
            .ok_or("Engine does not support query_models metadata validation")?;
        if info["id"] != probe["id"]
            || models
                .first()
                .and_then(|m| m["usesHumanSLProfile"].as_bool())
                != Some(false)
        {
            return Err(
                "Analysis model must be a regular strong model, not a human SL model".into(),
            );
        }
        if self.human && !models.iter().any(|m| m["usesHumanSLProfile"] == true) {
            return Err(
                "Human model is not rank-conditioned: usesHumanSLProfile must be true".into(),
            );
        }
        self.models = info["models"].clone();
        Ok(())
    }
    async fn raw(&mut self, s: &Settings, query: &Value) -> Result<Value> {
        // Register before spawning/loading so cancellation during startup is not lost.
        let cancel = self.cancel.clone();
        let cancel_all = self.cancel_all.clone();
        let cancelled = cancel.notified();
        let stopped = cancel_all.notified();
        tokio::pin!(cancelled, stopped);
        cancelled.as_mut().enable();
        stopped.as_mut().enable();
        let human = self.human;
        tokio::select! {
            result=self.start(s)=>result?,
            _=&mut cancelled, if !human=>return Err("Analysis cancelled".into()),
            _=&mut stopped=>return Err("Worker restart requested".into()),
        }
        let bytes = format!("{query}\n");
        self.stdin
            .as_mut()
            .ok_or("Missing engine stdin")?
            .write_all(bytes.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        self.stdin
            .as_mut()
            .unwrap()
            .flush()
            .await
            .map_err(|e| e.to_string())?;
        let id = query["id"].as_str().unwrap_or("");
        let task = async {
            loop {
                let line = self
                    .stdout
                    .as_mut()
                    .ok_or("Missing engine stdout")?
                    .next_line()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or("KataGo exited unexpectedly; expand diagnostics for stderr")?;
                let v: Value =
                    serde_json::from_str(&line).map_err(|e| format!("Invalid KataGo JSON: {e}"))?;
                if let Some(e) = v.get("error") {
                    return Err(format!("KataGo: {e}"));
                }
                if v["id"].as_str() != Some(id) {
                    continue;
                }
                if v.get("warning").is_some() {
                    return Err(format!(
                        "KataGo rejected a protocol setting: {}",
                        v["warning"]
                    ));
                }
                if v["isDuringSearch"].as_bool() == Some(true) {
                    self.update(
                        if self.human {
                            "generating move"
                        } else {
                            "analyzing"
                        },
                        "Searching",
                        None,
                        v["rootInfo"]["visits"].as_u64().unwrap_or(0) as u32,
                    );
                    continue;
                }
                return Ok(v);
            }
        };
        tokio::select! {
            result=timeout(Duration::from_secs(s.timeout_seconds),task) => result.map_err(|_|"KataGo timed out; retry to restart the worker".to_string())?,
            _=&mut cancelled, if !human => Err("Analysis cancelled".into()),
            _=&mut stopped => Err("Worker restart requested".into()),
        }
    }
    async fn request(&mut self, s: &Settings, q: &Value) -> Result<Value> {
        let result = self.raw(s, q).await;
        if let Err(e) = &result {
            self.stop().await;
            if e == "Analysis cancelled" {
                self.update(
                    "ready",
                    "Analysis paused for opponent or cancelled",
                    None,
                    0,
                );
            } else {
                self.update("error", "Retry or check Settings", Some(e.clone()), 0);
            }
        }
        result
    }
}
impl Engine {
    pub fn new(events: broadcast::Sender<String>) -> Self {
        let status = Arc::new(StdMutex::new(Status {
            human: WorkerStatus {
                state: "locating executable".into(),
                detail: "Waiting for setup validation".into(),
                ..Default::default()
            },
            analysis: WorkerStatus {
                state: "locating executable".into(),
                detail: "Waiting for setup validation".into(),
                ..Default::default()
            },
        }));
        let cancel = Arc::new(Notify::new());
        let cancel_all = Arc::new(Notify::new());
        let worker = |human| Worker {
            human,
            child: None,
            stdin: None,
            stdout: None,
            status: status.clone(),
            events: events.clone(),
            signature: String::new(),
            version: String::new(),
            models: Value::Null,
            cancel: cancel.clone(),
            cancel_all: cancel_all.clone(),
        };
        Self {
            human: Mutex::new(worker(true)),
            strong: Mutex::new(worker(false)),
            status,
            events,
            cancel,
            cancel_all,
        }
    }
    pub fn cancel_analysis(&self) {
        self.cancel.notify_waiters();
    }
    pub async fn restart(&self) {
        self.cancel_all.notify_waiters();
        self.human.lock().await.stop().await;
        self.strong.lock().await.stop().await;
        {
            let mut s = self.status.lock().unwrap();
            {
                let w = &mut s.human;
                w.state = "starting".into();
                w.error = None;
            }
            s.analysis.state = "starting".into();
            s.analysis.error = None;
        }
        let _ = self.events.send("status".into());
    }
    pub async fn shutdown(&self) {
        self.cancel_all.notify_waiters();
        self.human.lock().await.stop().await;
        self.strong.lock().await.stop().await;
    }
    pub async fn probe_human(&self, s: &Settings) -> Result<()> {
        let mut w = self.human.lock().await;
        for rank in -20..=8 {
            let mut q = query(&Game::new(Color::B, rank, "auto".into()), None, 1, true)?;
            q["overrideSettings"] = json!({"humanSLProfile":format!("rank_{}",rank_label(rank)),"ignorePreRootHistory":false});
            let v = w.request(s, &q).await?;
            if decode_policy(&v).is_err() {
                w.stop().await;
                w.update("error","Incompatible human model",Some("Expected rank-conditioned humanPolicy with 362 entries. Use b18c384nbt-humanv0.bin.gz as the human model.".into()),0);
                return Err("Incompatible human SL model".into());
            }
            self.status.lock().unwrap().human.supported_ranks.push(rank);
        }
        w.update("ready", "Human SL profiles 20k–9d validated", None, 0);
        Ok(())
    }
    pub async fn generate(&self, s: &Settings, g: &Game) -> Result<Option<usize>> {
        let mut w = self.human.lock().await;
        let position = g.position(&g.active_head)?;
        let after_pass = position.passes == 1;
        let visits = if after_pass {
            s.deep_visits.max(256)
        } else {
            s.quick_visits.max(64)
        };
        let mut q = query(g, None, visits, true)?;
        q["includeOwnership"] = json!(after_pass);
        for setting in [
            "humanSLRootExploreProbWeightless",
            "humanSLRootExploreProbWeightful",
            "humanSLPlaExploreProbWeightless",
            "humanSLPlaExploreProbWeightful",
            "humanSLOppExploreProbWeightless",
            "humanSLOppExploreProbWeightful",
        ] {
            q["overrideSettings"][setting] = json!(0.);
        }
        w.update(
            "generating move",
            &format!(
                "Checking passing and sampling {} human policy",
                rank_label(g.rank)
            ),
            None,
            0,
        );
        let v = w.request(s, &q).await?;
        let generated: Result<Option<usize>> = async {
            let chosen = select_human_move(&v, &position, rand::rng().random::<f64>())?;
            if chosen.is_none() {
                return Ok(None);
            }
            // A settled board may have many equivalent cleanup moves ahead of pass.
            // After the human passes, explicitly compare passing against continuing.
            if after_pass && settled_position(&v, &position)? {
                w.update(
                    "generating move",
                    "Checking whether cleanup changes the score",
                    None,
                    0,
                );
                let mut pass_query = q.clone();
                pass_query["id"] = json!(id());
                pass_query["includePolicy"] = json!(false);
                pass_query["allowMoves"] =
                    json!([{ "player": position.turn, "moves": ["pass"], "untilDepth": 1 }]);
                let pass = w.request(s, &pass_query).await?;
                if equivalent_pass(&v, &pass, &position)? {
                    return Ok(None);
                }
            }
            Ok(chosen)
        }
        .await;
        let point = match generated {
            Ok(point) => point,
            Err(e) => {
                w.stop().await;
                w.update(
                    "error",
                    "Opponent move generation failed",
                    Some(e.clone()),
                    0,
                );
                return Err(e);
            }
        };
        w.update("ready", "Human opponent ready", None, visits);
        Ok(point)
    }
    pub async fn analyze(
        &self,
        s: &Settings,
        g: &Game,
        node: &str,
        visits: u32,
    ) -> Result<Evaluation> {
        let mut w = self.strong.lock().await;
        w.update(
            "analyzing",
            &format!("Analyzing position · up to {visits} visits"),
            None,
            0,
        );
        let q = query(g, Some(node), visits, false)?;
        let v = w.request(s, &q).await?;
        let result = match decode_evaluation(
            &v,
            &w.version,
            s,
            json!({"request":q,"loaded_models":w.models,"classification_version":s.classification.version,"worker_overrides":{"reportAnalysisWinratesAs":"BLACK","numAnalysisThreads":1,"numSearchThreadsPerAnalysisThread":2,"nnMaxBatchSize":4,"nnCacheSizePowerOfTwo":18}}),
        ) {
            Ok(result) => result,
            Err(e) => {
                w.stop().await;
                w.update(
                    "error",
                    "Invalid strong-analysis response",
                    Some(e.clone()),
                    0,
                );
                return Err(e);
            }
        };
        w.update("ready", "Strong analyzer ready", None, result.visits);
        Ok(result)
    }
}
pub fn gtp(point: Option<usize>) -> String {
    point
        .map(|p| {
            format!(
                "{}{}",
                b"ABCDEFGHJKLMNOPQRST"[p % SIZE] as char,
                SIZE - p / SIZE
            )
        })
        .unwrap_or_else(|| "pass".into())
}
pub fn parse_gtp(s: &str) -> Result<Option<usize>> {
    if s.eq_ignore_ascii_case("pass") {
        return Ok(None);
    }
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return Err("Invalid engine coordinate".into());
    }
    let x = b"ABCDEFGHJKLMNOPQRST"
        .iter()
        .position(|b| *b == bytes[0].to_ascii_uppercase())
        .ok_or("Invalid engine column")?;
    let y = s[1..].parse::<usize>().map_err(|_| "Invalid engine row")?;
    if !(1..=SIZE).contains(&y) {
        return Err("Invalid engine row".into());
    }
    Ok(Some((SIZE - y) * SIZE + x))
}
fn query(g: &Game, node: Option<&str>, visits: u32, human: bool) -> Result<Value> {
    let moves: Vec<_> = g
        .path(node.unwrap_or(&g.active_head))?
        .iter()
        .filter_map(|n| n.color.map(|c| json!([c, gtp(n.point)])))
        .collect();
    let mut q = json!({"id":id(),"moves":moves,"rules":"aga","komi":7.5,"boardXSize":19,"boardYSize":19,"maxVisits":visits,"includePolicy":human,"includeOwnership":!human,"reportDuringSearchEvery":0.5,"overrideSettings":{"ignorePreRootHistory":false}});
    if human {
        q["overrideSettings"]["humanSLProfile"] = json!(format!("rank_{}", rank_label(g.rank)));
    }
    Ok(q)
}
fn decode_policy(v: &Value) -> Result<Vec<f64>> {
    let values = v["humanPolicy"]
        .as_array()
        .ok_or("Human policy missing; the human model is incompatible")?;
    if values.len() != POINTS + 1 {
        return Err("Invalid human policy length".into());
    }
    values
        .iter()
        .map(|n| {
            n.as_f64()
                .filter(|n| n.is_finite())
                .ok_or_else(|| "Invalid human policy probability".into())
        })
        .collect()
}
fn preferred_candidate(v: &Value) -> Result<&Value> {
    v["moveInfos"]
        .as_array()
        .and_then(|moves| moves.iter().find(|m| m["order"].as_u64() == Some(0)))
        .ok_or_else(|| "Strong passing judgment is missing its preferred move".into())
}
fn preferred_move(v: &Value) -> Result<Option<usize>> {
    if v["rootInfo"]["visits"].as_u64().unwrap_or(0) < 64 {
        return Err("Passing judgment needs at least 64 strong-search visits; check the engine's time limits".into());
    }
    parse_gtp(
        preferred_candidate(v)?["move"]
            .as_str()
            .ok_or("Preferred move is missing its coordinate")?,
    )
}
fn select_human_move(v: &Value, position: &Position, random: f64) -> Result<Option<usize>> {
    let mut policy = decode_policy(v)?;
    if preferred_move(v)?.is_none() {
        return Ok(None);
    }
    // Never replace a human board move with the strong engine's preferred board move.
    policy[POINTS] = 0.;
    sample_policy(&policy, position, random)
}
fn settled_position(v: &Value, position: &Position) -> Result<bool> {
    if v["rootInfo"]["visits"].as_u64().unwrap_or(0) < 256
        || number(&v["rootInfo"], "scoreStdev")? > 2.
    {
        return Ok(false);
    }
    let ownership = v["ownership"]
        .as_array()
        .filter(|o| o.len() == POINTS)
        .ok_or("Passing judgment requires ownership for the full board")?;
    Ok(position.board.iter().any(|&stone| stone != 0)
        && position.board.iter().enumerate().all(|(p, &stone)| {
            stone == 0
                || ownership[p]
                    .as_f64()
                    .is_some_and(|value| value.is_finite() && (0.8..=1.).contains(&value.abs()))
        }))
}
fn equivalent_pass(search: &Value, pass: &Value, position: &Position) -> Result<bool> {
    if position.passes != 1 || !settled_position(search, position)? {
        return Ok(false);
    }
    let candidate = preferred_candidate(pass)?;
    if preferred_move(pass)?.is_some()
        || candidate["visits"].as_u64().unwrap_or(0) < 128
        || !settled_position(pass, position)?
    {
        return Ok(false);
    }
    let best = preferred_candidate(search)?;
    let sign = if position.turn == Color::B { 1. } else { -1. };
    let score_loss = (number(best, "scoreLead")? - number(candidate, "scoreLead")?) * sign;
    let winrate_loss = (number(best, "winrate")? - number(candidate, "winrate")?) * sign;
    // Conservative endgame-only tolerances. Final scoring still requires explicit agreement.
    Ok(score_loss <= 0.5 && winrate_loss <= 0.01)
}
pub fn sample_policy(policy: &[f64], position: &Position, random: f64) -> Result<Option<usize>> {
    if policy.len() != POINTS + 1 {
        return Err("Invalid human policy length".into());
    }
    let legal: Vec<_> = policy
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| {
            let point = if i == POINTS { None } else { Some(i) };
            let mut test = position.clone();
            (p.is_finite() && p > 0. && test.play(point).is_ok()).then_some((point, p))
        })
        .collect();
    let total: f64 = legal.iter().map(|(_, p)| p).sum();
    if total <= 0. {
        return Err("Human model returned no legal probability mass".into());
    }
    let mut target = random.clamp(0., 1. - f64::EPSILON) * total;
    for (point, p) in &legal {
        target -= p;
        if target < 0. {
            return Ok(*point);
        }
    }
    Ok(legal.last().unwrap().0)
}
fn number(v: &Value, key: &str) -> Result<f64> {
    v[key]
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("Engine response missing {key}"))
}
fn identity(path: &str) -> String {
    use sha2::{Digest, Sha256};
    if let Ok(meta) = std::fs::metadata(path) {
        // Path + file size + modification timestamp identifies model revisions without rereading gigabytes per position.
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        if path.ends_with(".cfg")
            && let Ok(bytes) = std::fs::read(path)
        {
            return format!("{path} sha256:{:x}", Sha256::digest(bytes));
        }
        return format!("{path} size:{} modified:{modified}", meta.len());
    }
    path.into()
}
pub fn decode_evaluation(
    v: &Value,
    version: &str,
    s: &Settings,
    parameters: Value,
) -> Result<Evaluation> {
    let root = &v["rootInfo"];
    let mut candidates = vec![];
    for m in v["moveInfos"]
        .as_array()
        .ok_or("Engine response missing moveInfos")?
    {
        let pv = m["pv"]
            .as_array()
            .ok_or("Candidate missing PV")?
            .iter()
            .map(|p| parse_gtp(p.as_str().unwrap_or("")))
            .collect::<Result<Vec<_>>>()?;
        candidates.push(Candidate {
            point: parse_gtp(m["move"].as_str().ok_or("Candidate missing move")?)?,
            visits: m["visits"].as_u64().ok_or("Candidate missing visits")? as u32,
            score_black: number(m, "scoreLead")?,
            winrate_black: number(m, "winrate")?,
            pv,
            order: m["order"].as_u64().unwrap_or(999) as usize,
        });
    }
    candidates.sort_by_key(|c| c.order);
    let ownership = v["ownership"]
        .as_array()
        .map(|a| a.iter().map(|n| n.as_f64().unwrap_or(0.)).collect())
        .unwrap_or_default();
    let winrate = number(root, "winrate")?;
    if !(0.0..=1.).contains(&winrate) {
        return Err("Invalid engine winrate".into());
    }
    Ok(Evaluation {
        score_black: number(root, "scoreLead")?,
        winrate_black: winrate,
        visits: root["visits"].as_u64().ok_or("Root missing visits")? as u32,
        candidates,
        ownership,
        engine: version.into(),
        model: identity(&s.analysis_model),
        config: identity(&s.config),
        parameters,
        created_at: now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_is_sampled_proportionally_and_legally() {
        let mut policy = vec![-1.; 362];
        policy[0] = 0.25;
        policy[1] = 0.75;
        assert_eq!(
            sample_policy(&policy, &Position::default(), 0.1).unwrap(),
            Some(0)
        );
        assert_eq!(
            sample_policy(&policy, &Position::default(), 0.3).unwrap(),
            Some(1)
        );
        let mut p = Position::default();
        p.play(Some(0)).unwrap();
        assert_eq!(sample_policy(&policy, &p, 0.).unwrap(), Some(1));
        assert!(decode_policy(&json!({"policy":policy})).is_err());
        assert!(parse_gtp("I2").is_err());
        for i in 0..POINTS {
            assert_eq!(parse_gtp(&gtp(Some(i))).unwrap(), Some(i));
        }
    }
    #[test]
    fn real_strong_fixture_and_malformed_response() {
        let value: Value =
            serde_json::from_str(include_str!("../tests/fixtures/strong.json")).unwrap();
        let e = decode_evaluation(&value, "test", &Settings::default(), json!({})).unwrap();
        assert!(e.score_black < 0.);
        assert_eq!(e.ownership.len(), 361);
        assert!(!e.candidates.is_empty());
        assert!(decode_evaluation(&json!({}), "test", &Settings::default(), json!({})).is_err());
    }
    #[tokio::test]
    async fn missing_executable_reports_error_and_retry_is_possible() {
        let (events, _) = broadcast::channel(10);
        let engine = Engine::new(events);
        let s = Settings {
            executable: "/nonexistent/katago".into(),
            ..Default::default()
        };
        let g = Game::new(Color::B, -5, "auto".into());
        assert!(engine.analyze(&s, &g, &g.root, 1).await.is_err());
        assert_eq!(engine.status.lock().unwrap().analysis.state, "error");
        engine.restart().await;
        assert_eq!(engine.status.lock().unwrap().analysis.state, "starting");
    }
    #[tokio::test]
    #[ignore = "requires WEIQI_HUMAN_MODEL and a Metal-capable host; run outside restricted GPU sandbox"]
    async fn real_engine_smoke_restart_and_human_profiles() {
        let (events, _) = broadcast::channel(10);
        let engine = Engine::new(events);
        let s = Settings {
            human_model: std::env::var("WEIQI_HUMAN_MODEL").expect("Set WEIQI_HUMAN_MODEL"),
            ..Default::default()
        };
        engine.probe_human(&s).await.unwrap();
        let g = Game::new(Color::B, -5, "auto".into());
        let m = engine.generate(&s, &g).await.unwrap();
        assert!(g.position(&g.root).unwrap().play(m).is_ok());
        let a = engine.analyze(&s, &g, &g.root, 64).await.unwrap();
        assert!(a.visits >= 64);
        engine.restart().await;
        assert!(engine.analyze(&s, &g, &g.root, 16).await.is_ok());
        engine.shutdown().await;
    }

    fn fixture(body: &str) -> (tempfile::TempDir, Settings) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("katago");
        std::fs::write(&executable,format!("#!/usr/bin/env python3\nimport sys,json,time\nif 'version' in sys.argv:\n print('KataGo test fixture');sys.exit(0)\n{body}\n")).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let model = dir.path().join("model");
        std::fs::write(&model, "fixture").unwrap();
        let config = dir.path().join("config.cfg");
        std::fs::write(&config, "fixture").unwrap();
        let settings = Settings {
            executable: executable.to_string_lossy().into(),
            analysis_model: model.to_string_lossy().into(),
            human_model: model.to_string_lossy().into(),
            config: config.to_string_lossy().into(),
            timeout_seconds: 1,
            ..Default::default()
        };
        (dir, settings)
    }
    #[tokio::test]
    async fn protocol_errors_crashes_malformed_and_timeouts_are_recoverable() {
        for (body, expected) in [
            (
                "for line in sys.stdin:\n print(json.dumps({'error':'bad profile'}),flush=True)",
                "bad profile",
            ),
            (
                "for line in sys.stdin:\n print('not JSON',flush=True)",
                "Invalid KataGo JSON",
            ),
            (
                "for line in sys.stdin:\n print('crash fixture',file=sys.stderr,flush=True);sys.exit(1)",
                "exited",
            ),
            ("for line in sys.stdin:\n time.sleep(30)", "timed out"),
        ] {
            let (_dir, s) = fixture(body);
            let (events, _) = broadcast::channel(10);
            let e = Engine::new(events);
            let g = Game::new(Color::B, -5, "auto".into());
            let error = e.analyze(&s, &g, &g.root, 1).await.unwrap_err();
            assert!(error.contains(expected), "{error}");
            assert_eq!(e.status.lock().unwrap().analysis.state, "error");
            assert!(e.status.lock().unwrap().analysis.pid.is_none());
            e.restart().await;
            assert_eq!(e.status.lock().unwrap().analysis.state, "starting");
            e.shutdown().await;
        }
    }
    #[tokio::test]
    async fn cancelling_search_stops_the_child() {
        let (_dir, s) = fixture("for line in sys.stdin:\n time.sleep(30)");
        let (events, _) = broadcast::channel(10);
        let e = Arc::new(Engine::new(events));
        let g = Game::new(Color::B, -5, "auto".into());
        let task = {
            let e = e.clone();
            tokio::spawn(async move { e.analyze(&s, &g, &g.root, 64).await })
        };
        tokio::time::sleep(Duration::from_millis(150)).await;
        e.cancel_analysis();
        assert_eq!(task.await.unwrap().unwrap_err(), "Analysis cancelled");
        assert!(e.status.lock().unwrap().analysis.pid.is_none());
    }
    #[test]
    fn real_human_fixture_has_correct_policy_shape() {
        let value: Value =
            serde_json::from_str(include_str!("../tests/fixtures/human-5k.json")).unwrap();
        let policy = decode_policy(&value).unwrap();
        assert_eq!(policy.len(), 362);
        assert!((policy.iter().filter(|&&p| p > 0.).sum::<f64>() - 1.).abs() < 0.001);
    }
    #[tokio::test]
    async fn ordinary_strong_model_cannot_masquerade_as_human_sl() {
        let (_dir, s) = fixture(
            "for line in sys.stdin:\n q=json.loads(line);print(json.dumps({'id':q['id'],'models':[{'usesHumanSLProfile':False}]}),flush=True)",
        );
        let (events, _) = broadcast::channel(10);
        let e = Engine::new(events);
        let error = e.probe_human(&s).await.unwrap_err();
        assert!(error.contains("not rank-conditioned"));
        assert!(e.status.lock().unwrap().human.supported_ranks.is_empty());
        e.shutdown().await;
    }
    #[test]
    fn search_controls_passing_without_replacing_rank_conditioned_board_moves() {
        let mut policy = vec![0.; POINTS + 1];
        policy[0] = 0.2;
        policy[1] = 0.6;
        policy[POINTS] = 0.2;
        // The strong engine prefers C19, but ordinary moves must still come from human policy.
        let mut response = json!({"rootInfo":{"visits":65},"humanPolicy":policy,"moveInfos":[{"order":1,"move":"pass"},{"order":0,"move":"C19"}]});
        let p = Position::default();
        assert_eq!(select_human_move(&response, &p, 0.1).unwrap(), Some(0));
        assert_eq!(select_human_move(&response, &p, 0.3).unwrap(), Some(1));
        assert_eq!(select_human_move(&response, &p, 0.99).unwrap(), Some(1));
        response["moveInfos"][1]["move"] = json!("pass");
        response["humanPolicy"][POINTS] = json!(0.);
        assert_eq!(select_human_move(&response, &p, 0.99).unwrap(), None);
        response["rootInfo"]["visits"] = json!(1);
        assert!(select_human_move(&response, &p, 0.99).is_err());
    }
    #[test]
    fn equivalent_cleanup_can_end_only_after_a_confident_human_pass() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/fixtures/endgame-pass.json")).unwrap();
        let mut p = Position::default();
        p.play(Some(0)).unwrap();
        p.play(Some(360)).unwrap();
        assert!(!equivalent_pass(&fixture["search"], &fixture["pass"], &p).unwrap());
        p.play(None).unwrap();
        assert_eq!(p.turn, Color::W);
        assert!(equivalent_pass(&fixture["search"], &fixture["pass"], &p).unwrap());
        let mut one_root_visit = fixture["pass"].clone();
        one_root_visit["moveInfos"][0]["visits"] = json!(255);
        assert!(equivalent_pass(&fixture["search"], &one_root_visit, &p).unwrap());
        let mut worse = fixture["pass"].clone();
        worse["moveInfos"][0]["scoreLead"] = json!(-8.);
        assert!(!equivalent_pass(&fixture["search"], &worse, &p).unwrap());
        worse = fixture["pass"].clone();
        worse["moveInfos"][0]["winrate"] = json!(0.1);
        assert!(!equivalent_pass(&fixture["search"], &worse, &p).unwrap());
        let mut uncertain = fixture["search"].clone();
        uncertain["rootInfo"]["scoreStdev"] = json!(5.);
        assert!(!equivalent_pass(&uncertain, &fixture["pass"], &p).unwrap());
        uncertain = fixture["search"].clone();
        uncertain["ownership"][0] = json!(0.1);
        assert!(!equivalent_pass(&uncertain, &fixture["pass"], &p).unwrap());
        worse = fixture["pass"].clone();
        worse["moveInfos"][0]["visits"] = json!(2);
        assert!(!equivalent_pass(&fixture["search"], &worse, &p).unwrap());
        // Check the same loss calculation from Black's perspective.
        p.turn = Color::B;
        worse = fixture["pass"].clone();
        worse["moveInfos"][0]["scoreLead"] = json!(-12.);
        assert!(!equivalent_pass(&fixture["search"], &worse, &p).unwrap());
    }
    #[tokio::test]
    async fn opponent_enters_scoring_even_when_the_human_model_never_passes() {
        let (_dir, s) = fixture(include_str!("../tests/fake_katago.py"));
        let (events, _) = broadcast::channel(10);
        let engine = Engine::new(events);
        let mut g = Game::new(Color::B, -5, "auto".into());
        let root = g.root.clone();
        let head = g.add(&root, None, false).unwrap();
        let point = engine.generate(&s, &g).await.unwrap();
        assert_eq!(point, None);
        g.add_opponent(&head, point).unwrap();
        assert_eq!(g.status, "scoring");
        assert!(g.result.is_none());
        engine.shutdown().await;
    }
    #[tokio::test]
    #[ignore = "requires WEIQI_ENDGAME_GAME_JSON, local models and Metal GPU access; never mutates the saved game"]
    async fn real_engine_passes_in_reported_settled_endgame() {
        let path =
            std::env::var("WEIQI_ENDGAME_GAME_JSON").expect("Provide a saved Game JSON snapshot");
        let mut g: Game = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        if g.position(&g.active_head).unwrap().turn == g.human {
            let head = g.active_head.clone();
            g.add(&head, None, false).unwrap();
        }
        assert_eq!(g.position(&g.active_head).unwrap().passes, 1);
        let (events, _) = broadcast::channel(10);
        let engine = Engine::new(events);
        let s = Settings::default();
        for _ in 0..5 {
            assert_eq!(engine.generate(&s, &g).await.unwrap(), None);
        }
        let point = None;
        let head = g.active_head.clone();
        g.add_opponent(&head, point).unwrap();
        assert_eq!(g.status, "scoring");
        assert!(g.result.is_none());
        engine.shutdown().await;
    }
}
