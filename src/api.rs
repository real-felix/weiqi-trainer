use crate::{
    app::{App, scoring},
    domain::*,
    settings::Settings,
    sgf,
};
use axum::{
    Json, Router,
    extract::Request,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

pub struct ApiError(pub StatusCode, pub String);
impl From<String> for ApiError {
    fn from(e: String) -> Self {
        Self(
            if e == "Game not found" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::CONFLICT
            },
            e,
        )
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
type ApiResult<T> = std::result::Result<T, ApiError>;
pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/state", get(state))
        .route("/api/events", get(events))
        .route("/api/settings", put(settings))
        .route("/api/engine/restart", post(restart))
        .route("/api/games", post(create))
        .route("/api/games/import", post(import))
        .route("/api/games/{id}", get(game))
        .route("/api/games/{id}/moves", post(moves))
        .route("/api/games/{id}/continue", post(activate))
        .route("/api/games/{id}/retry", post(retry))
        .route("/api/games/{id}/analysis", post(analyze))
        .route("/api/games/{id}/analysis/cancel", post(cancel))
        .route("/api/games/{id}/resign", post(resign))
        .route("/api/games/{id}/score", post(score))
        .route("/api/games/{id}/sgf", get(export))
        .route("/api/games/{id}/audit", get(audit))
        .layer(DefaultBodyLimit::max(1_100_000))
        .layer(middleware::from_fn(local_only))
        .with_state(app)
}
async fn local_only(req: Request, next: Next) -> Response {
    let headers = req.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let local = host.starts_with("127.0.0.1:") || host.starts_with("localhost:");
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let valid_origin = origin.is_none_or(|o| {
        o == format!("http://{host}")
            || o == "http://127.0.0.1:5173"
            || o == "http://localhost:5173"
    });
    if !local || !valid_origin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"Only local, same-origin access is allowed"})),
        )
            .into_response();
    }
    next.run(req).await
}
async fn state(State(a): State<Arc<App>>) -> ApiResult<Json<Value>> {
    let r = a.repo.lock().unwrap();
    let p = r.profile()?;
    let games = r.list()?;
    let settings = r.settings()?;
    Ok(Json(
        json!({"schema_version":1,"games":games,"opponent_rank":p.auto_rank,"initialized":p.initialized,"settings":settings,"engine":a.engine.status.lock().unwrap().clone(),"data_dir":a.data_dir}),
    ))
}
async fn events(
    State(a): State<Arc<App>>,
) -> Sse<impl tokio_stream::Stream<Item = std::result::Result<Event, Infallible>>> {
    let rx = a.engine.events.subscribe();
    let initial = tokio_stream::once(Ok(Event::default().event("sync").data("initial")));
    let updates = BroadcastStream::new(rx)
        .take_while(|r| !r.as_ref().is_ok_and(|s| s == "shutdown"))
        .map(|r| {
            Ok(Event::default()
                .event("sync")
                .data(r.unwrap_or_else(|_| "resync".into())))
        });
    Sse::new(initial.chain(updates)).keep_alive(KeepAlive::default())
}
async fn settings(State(a): State<Arc<App>>, Json(s): Json<Settings>) -> ApiResult<Json<Value>> {
    a.repo.lock().unwrap().save_settings(&s)?;
    a.setup();
    Ok(Json(json!({"saved":true})))
}
async fn restart(State(a): State<Arc<App>>) -> Json<Value> {
    a.setup();
    Json(json!({"restarting":true}))
}
#[derive(Deserialize)]
struct NewGame {
    mode: String,
    rank: Option<i32>,
    starting_rank: Option<i32>,
    color: Option<Color>,
}
async fn create(State(a): State<Arc<App>>, Json(n): Json<NewGame>) -> ApiResult<Json<Value>> {
    if n.mode != "auto" && n.mode != "manual" {
        return Err("Choose Auto or Manual".to_string().into());
    }
    let g = {
        let mut r = a.repo.lock().unwrap();
        let profile = r.profile()?;
        let rank = if n.mode == "manual" {
            n.rank
                .ok_or_else(|| ApiError::from("Select an opponent rank".to_string()))?
        } else if !profile.initialized {
            n.starting_rank.unwrap_or(-5)
        } else {
            profile.auto_rank
        };
        if !a
            .engine
            .status
            .lock()
            .unwrap()
            .human
            .supported_ranks
            .contains(&rank)
        {
            return Err(
                "Configure and validate the human SL worker in Settings before starting a game"
                    .to_string()
                    .into(),
            );
        }
        let human = n.color.unwrap_or(if r.count()?.is_multiple_of(2) {
            Color::B
        } else {
            Color::W
        });
        let g = Game::new(human, rank, n.mode);
        r.initialize(n.starting_rank.unwrap_or(-5))?;
        r.save(&g)?;
        g
    };
    let s = a.repo.lock().unwrap().settings()?;
    a.enqueue(&g.id, &g.root, s.quick_visits, false);
    a.kick(&g.id);
    a.changed();
    Ok(Json(json!({"id":g.id})))
}
async fn game(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult<Json<crate::app::GameView>> {
    Ok(Json(a.view(&id, q.get("node").map(String::as_str))?))
}
#[derive(Deserialize)]
struct MoveRequest {
    parent: String,
    point: Option<usize>,
    review: bool,
    expected_head: Option<String>,
}
async fn moves(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    Json(m): Json<MoveRequest>,
) -> ApiResult<Json<Value>> {
    let node = {
        let mut r = a.repo.lock().unwrap();
        let mut g = r.game(&id)?;
        if !m.review && m.expected_head.as_deref() != Some(&g.active_head) {
            return Err("The active position changed; refresh and try again"
                .to_string()
                .into());
        }
        let node = g.add(&m.parent, m.point, m.review)?;
        r.save(&g)?;
        node
    };
    let s = a.repo.lock().unwrap().settings()?;
    a.enqueue(&id, &m.parent, s.quick_visits, false);
    a.enqueue(
        &id,
        &node,
        if m.review {
            s.deep_visits
        } else {
            s.quick_visits
        },
        m.review,
    );
    if !m.review {
        a.kick(&id);
    }
    a.changed();
    Ok(Json(json!({"node":node})))
}
#[derive(Deserialize)]
struct Head {
    node: String,
    expected_head: String,
    #[serde(default)]
    resume: bool,
}
async fn activate(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    Json(h): Json<Head>,
) -> ApiResult<Json<Value>> {
    {
        let mut r = a.repo.lock().unwrap();
        let mut g = r.game(&id)?;
        if g.active_head != h.expected_head {
            return Err("The active position changed".to_string().into());
        }
        g.activate(&h.node, h.resume)?;
        r.save(&g)?;
    }
    a.kick(&id);
    a.changed();
    Ok(Json(json!({"active_head":h.node})))
}
async fn retry(State(a): State<Arc<App>>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    a.repo.lock().unwrap().game(&id)?;
    a.kick(&id);
    Ok(Json(json!({"retrying":true})))
}
#[derive(Deserialize)]
struct AnalysisRequest {
    node: String,
    #[serde(default)]
    deep: bool,
}
async fn analyze(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    Json(q): Json<AnalysisRequest>,
) -> ApiResult<Json<Value>> {
    let s = {
        let r = a.repo.lock().unwrap();
        r.game(&id)?.position(&q.node)?;
        r.settings()?
    };
    a.enqueue(
        &id,
        &q.node,
        if q.deep {
            s.deep_visits
        } else {
            s.quick_visits
        },
        true,
    );
    Ok(Json(json!({"queued":true})))
}
async fn cancel(State(a): State<Arc<App>>, Path(id): Path<String>) -> Json<Value> {
    a.cancel_queued(&id);
    Json(json!({"cancelled":true}))
}
#[derive(Deserialize)]
struct Finalize {
    head: String,
}
async fn resign(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    Json(q): Json<Finalize>,
) -> ApiResult<Json<Value>> {
    let g = {
        let mut r = a.repo.lock().unwrap();
        let g = r.game(&id)?;
        let result = format!("{}+R", g.human.other());
        r.finalize(&id, &q.head, &result)?
    };
    a.changed();
    Ok(Json(json!({"result":g.result})))
}
#[derive(Deserialize)]
struct ScoreRequest {
    head: String,
    dead: Vec<usize>,
    #[serde(default)]
    confirm: bool,
}
async fn score(
    State(a): State<Arc<App>>,
    Path(id): Path<String>,
    Json(q): Json<ScoreRequest>,
) -> ApiResult<Json<Value>> {
    let (g, e) = {
        let r = a.repo.lock().unwrap();
        let g = r.game(&id)?;
        if g.result.is_some() {
            return Ok(Json(json!({"result":g.result})));
        }
        if g.active_head != q.head || g.status != "scoring" {
            return Err("Scoring is only available on the active two-pass position"
                .to_string()
                .into());
        }
        let e = r
            .evaluations(&id)?
            .get(&q.head)
            .filter(|e| e.visits >= r.settings().map(|s| s.deep_visits).unwrap_or(256))
            .cloned();
        (g, e)
    };
    let Some(e) = e else {
        let s = a.repo.lock().unwrap().settings()?;
        a.enqueue(&id, &q.head, s.deep_visits, true);
        return Ok(Json(
            json!({"pending":true,"message":"Deep analysis queued to check the opponent's stone status. Try again when the analyzer is ready."}),
        ));
    };
    let scoring = scoring(&g.position(&q.head)?, &e, &q.dead)?;
    if q.confirm && scoring.agreed {
        a.repo
            .lock()
            .unwrap()
            .finalize(&id, &q.head, &scoring.score.result)?;
        a.changed();
    }
    Ok(Json(
        json!({"scoring":scoring,"finished":q.confirm && scoring.agreed}),
    ))
}
async fn export(State(a): State<Arc<App>>, Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    let text = sgf::export(&a.repo.lock().unwrap().game(&id)?);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/x-go-sgf; charset=utf-8".parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=weiqi-{id}.sgf")
            .parse()
            .unwrap(),
    );
    Ok((headers, text))
}
#[derive(Deserialize)]
struct Import {
    sgf: String,
}
async fn import(State(a): State<Arc<App>>, Json(q): Json<Import>) -> ApiResult<Json<Value>> {
    let g = sgf::import(&q.sgf)?;
    a.repo.lock().unwrap().save(&g)?;
    a.changed();
    Ok(Json(json!({"id":g.id})))
}
async fn audit(State(a): State<Arc<App>>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(a.repo.lock().unwrap().audit(&id)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn rejects_cross_origin_and_has_initial_state() {
        let r = crate::repository::Repository::open(std::path::Path::new(":memory:")).unwrap();
        let app = App::new(r, "test".into());
        let router = router(app);
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/state")
                    .header("host", "127.0.0.1:3000")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/state")
                    .header("host", "127.0.0.1:3000")
                    .header("origin", "https://other.example")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
