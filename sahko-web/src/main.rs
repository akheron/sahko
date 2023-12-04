mod date;
mod lock;
mod routes;

use crate::lock::WriteLock;
use crate::routes::email::send_email_route;
use axum::routing::get;
use axum::routing::post;
use axum::{Extension, Router};
use eyre::{Context, Result};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
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
        .route("/email", post(send_email_route))
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", assets_path.to_str().unwrap())).precompressed_gzip(),
        )
        .layer(Extension(write_lock))
        .layer(CompressionLayer::new());

    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8000".to_string());
    let addr = SocketAddr::from_str(&bind)?;
    let listener = TcpListener::bind(&addr).await?;

    info!("Starting server on {}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .context("Error starting server")?;

    Ok(())
}
