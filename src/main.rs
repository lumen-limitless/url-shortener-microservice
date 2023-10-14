use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use hyper::{Body, Method};
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct AppState {
    db: HashMap<u32, String>,
}

type SharedState = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let cors = tower_http::cors::CorsLayer::new()
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_origin(tower_http::cors::Any);

    let shared_state = Arc::new(RwLock::new(AppState::default()));

    // build our application with a route
    let app = axum::Router::new()
        .route("/", axum::routing::get(root))
        .route("/api/shorturl", axum::routing::post(short_url))
        .with_state(shared_state.clone())
        .route("/api/shorturl/:id", axum::routing::get(redirect))
        .with_state(shared_state.clone())
        .layer(cors);

    // read the port from env or use the port default port(8080)
    let port = std::env::var("PORT").unwrap_or(String::from("8080"));
    // convert the port to a socket address
    let addr = std::net::SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
        port.parse().unwrap(),
    );
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}

// basic handler that responds with a static string
async fn root() -> impl IntoResponse {
    "Hello, World!"
}

#[derive(Deserialize)]
struct ShortUrlRequest {
    url: String,
}

#[derive(Serialize)]
struct ShortUrlResponse {
    original_url: String,
    short_url: String,
}

async fn short_url(
    State(state): State<SharedState>,
    Json(body): Json<ShortUrlRequest>,
) -> impl IntoResponse {
    let len = state.read().unwrap().db.len();
    state
        .write()
        .unwrap()
        .db
        .insert(len as u32, body.url.clone());

    Json(ShortUrlResponse {
        original_url: body.url,
        short_url: format!("http://localhost:8080/{}", len),
    })
}

async fn redirect(
    State(state): State<SharedState>,
    path: axum::extract::Path<u32>,
) -> impl IntoResponse {
    let db = &state.read().unwrap().db;
    let url = db.get(&path.0).unwrap();
    axum::http::Response::builder()
        .status(axum::http::StatusCode::FOUND)
        .header("Location", url)
        .body(Body::empty())
        .unwrap()
}
