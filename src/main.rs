use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use myhttp::{Request, Response, Router, serve};

#[derive(Clone)]
struct AppState {
    hits: Arc<AtomicU64>,
}

async fn count(state: AppState, _req: Request) -> Response {
    let n = state.hits.fetch_add(1, Ordering::Relaxed) + 1;
    Response::text(200, format!("visits: {n}\n"))
}

async fn echo(_state: AppState, req: Request) -> Response {
    Response::new(200).body(req.body)
}

async fn hello(_state: AppState, _req: Request) -> Response {
    Response::text(200, "hello world")
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let state = AppState {
        hits: Arc::new(AtomicU64::new(0)),
    };

    let app = Router::new(state)
        .get("/", hello)
        .get("/count", count)
        .post("/echo", echo);

    serve("127.0.0.1:8080", app).await
}
