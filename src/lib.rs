use std::{io, sync::Arc};

mod conn;
mod encoding;
mod router;

use tokio::net::{TcpListener, ToSocketAddrs};

pub use crate::conn::{Request, Response};
pub use crate::router::Router;

use crate::conn::Connection;
use crate::encoding::Encoding;

pub trait Handler<S>: Send + Sync + 'static {
    fn handle(&self, state: S, req: Request) -> impl Future<Output = Response> + Send;
}

impl<F, Fut, S> Handler<S> for F
where
    F: Fn(S, Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send,
{
    fn handle(&self, state: S, req: Request) -> impl Future<Output = Response> + Send {
        self(state, req)
    }
}

pub async fn serve<A, S>(addr: A, router: Router<S>) -> io::Result<()>
where
    A: ToSocketAddrs,
    S: Clone + Send + Sync + 'static,
{
    let listener = TcpListener::bind(addr).await?;
    let router = Arc::new(router);

    loop {
        let (stream, _addr) = listener.accept().await?;
        let router = router.clone();

        tokio::spawn(async move {
            let mut conn = Connection::new(stream);
            if let Err(e) = handle_conn(&mut conn, &router).await {
                eprintln!("connection error: {e}");
            }
        });
    }
}

async fn handle_conn<S>(conn: &mut Connection, router: &Router<S>) -> io::Result<()>
where
    S: Clone + Send + Sync + 'static,
{
    let Some(req) = conn.read_request().await? else {
        return Ok(());
    };
    let encoding = req
        .header("Accept-Encoding")
        .and_then(|v| Encoding::from_header(v).first().copied());

    let mut resp = router.dispatch(req).await;
    if let Some(encoding) = encoding {
        resp.apply_encoding(encoding)?;
    }
    conn.write_response(&resp).await
}
