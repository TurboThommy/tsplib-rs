//! REST API for the TSP library.
mod errors;
mod models;
mod routes;
mod state;

use axum::{Router, http::StatusCode, routing::get};
use tower_http::{
    LatencyUnit,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "tsplib_server=info,tsplib_solver=debug,tower_http=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // initialize app state
    let state = state::AppState::new();

    // build application with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .merge(routes::problems::router())
        .merge(routes::solver_algorithms::router())
        .merge(routes::solver::router())
        .merge(routes::cancel::router())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .with_state(state);

    // bind to address
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tracing::info!("Listening on http://{}", listener.local_addr().unwrap());

    // serve the application
    axum::serve(listener, app).await.unwrap();
}

/// Health check endpoint.
async fn health_check() -> StatusCode {
    StatusCode::OK
}
