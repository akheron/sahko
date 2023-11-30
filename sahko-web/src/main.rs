mod date;
mod lock;
mod response;
mod routes;

use crate::lock::WriteLock;
use axum::routing::get;
use axum::routing::post;
use axum::{Extension, Router};
use eyre::{Context, Result};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::routes::index::index_route;
use crate::routes::schedule::update_schedule_route;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sahko_web=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Global write lock
    let write_lock = WriteLock::new();

    let assets_path = std::env::current_dir().unwrap();
    let app = Router::new()
        .route("/", get(index_route))
        .route("/schedule", post(update_schedule_route))
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", assets_path.to_str().unwrap())).precompressed_gzip(),
        )
        .layer(Extension(write_lock));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .unwrap();
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    info!("Starting server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("Error starting server")?;

    Ok(())
}
