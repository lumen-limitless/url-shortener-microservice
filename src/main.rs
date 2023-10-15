use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
    Form,
};
use hyper::{Body, Method, StatusCode};
use serde_json::json;

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
        .route("/api/shorturl", axum::routing::post(shorten_url))
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

#[derive(serde::Deserialize)]
struct UrlRequestBody {
    url: String,
}

async fn shorten_url(
    State(shared_state): State<SharedState>,
    Form(body): Form<UrlRequestBody>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let url = body.url;

    match url.starts_with("http") {
        true => {
            let mut state = shared_state.write().unwrap();
            let id = state.db.len() as u32;
            state.db.insert(id, url.clone());

            let res = json!({
                "original_url": url,
                "short_url": id
            });

            Ok((StatusCode::OK, Json(res)))
        }
        _ => {
            let res = json!(
                {
                    "error": "invalid url"
                }
            );

            Ok((StatusCode::OK, Json(res)))
        }
    }
}

async fn redirect(
    State(state): State<SharedState>,
    path: axum::extract::Path<u32>,
) -> impl IntoResponse {
    let db = &state.read().unwrap().db;
    match db.get(&path.0) {
        Some(url) => axum::http::Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", url)
            .body(Body::empty())
            .unwrap(),
        None => axum::http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}
