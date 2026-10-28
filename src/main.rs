mod api;
mod app;
mod domain;
mod engine;
mod preview;
mod repository;
mod settings;
mod sgf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = settings::data_dir();
    std::fs::create_dir_all(&dir)?;
    let repo = repository::Repository::open(&dir.join("trainer.sqlite3"))?;
    let app = app::App::new(repo, dir.to_string_lossy().into());
    app.background();
    app.setup();
    let dist = std::env::var("WEIQI_DIST_DIR").unwrap_or_else(|_| "dist".into());
    let router = api::router(app.clone()).fallback_service(
        tower_http::services::ServeDir::new(&dist).not_found_service(
            tower_http::services::ServeFile::new(format!("{dist}/index.html")),
        ),
    );
    let port = std::env::var("WEIQI_PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse::<u16>()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!(
        "Weiqi Trainer → http://127.0.0.1:{port}\nData → {}",
        dir.display()
    );
    let shutdown_app = app.clone();
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_app.engine.events.send("shutdown".into());
            shutdown_app.engine.shutdown().await;
        })
        .await?;
    app.engine.shutdown().await;
    Ok(())
}
