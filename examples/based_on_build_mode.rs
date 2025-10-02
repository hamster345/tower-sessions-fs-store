use std::net::{Ipv4Addr, SocketAddr};

use axum::{response::IntoResponse, routing::get, Router};
use serde::{Deserialize, Serialize};
use tower_sessions::{Session, SessionManagerLayer};

const COUNTER_KEY: &str = "counter";

#[derive(Default, Deserialize, Serialize)]
struct Counter(u64);

async fn handler(session: Session) -> impl IntoResponse {
    let counter: Counter = session.get(COUNTER_KEY).await.unwrap().unwrap_or_default();
    assert_eq!(counter.0, 0);
    session.insert(COUNTER_KEY, counter.0 + 1).await.unwrap();
    assert_eq!(counter.0, 1);
}

#[tokio::main]
async fn main() {
    #[cfg(not(debug_assertions))]
    let session_store = tower_sessions::MemoryStore::default();

    #[cfg(debug_assertions)]
    let session_store = tower_sessions_fs_store::FileStore::default();

    let mut session_layer = SessionManagerLayer::new(session_store);

    if cfg!(debug_assertions) {
        session_layer = session_layer.with_http_only(false).with_secure(false);
    }

    let app = Router::new().route("/", get(handler)).layer(session_layer);

    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap()
}
