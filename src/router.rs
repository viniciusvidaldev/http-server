use std::{collections::HashMap, pin::Pin};

use crate::{
    Handler,
    conn::{Method, Request, Response},
};

type BoxFuture<'a> = Pin<Box<dyn Future<Output = Response> + Send + 'a>>;

trait DynHandler<S>: Send + Sync {
    fn call<'a>(&'a self, state: S, req: Request) -> BoxFuture<'a>;
}

impl<S, H> DynHandler<S> for H
where
    S: Send + 'static,
    H: Handler<S>,
{
    fn call<'a>(&'a self, state: S, req: Request) -> BoxFuture<'a> {
        Box::pin(self.handle(state, req))
    }
}

pub struct Router<S> {
    routes: HashMap<String, HashMap<Method, Box<dyn DynHandler<S>>>>,
    state: S,
}

impl<S> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(state: S) -> Self {
        Self {
            routes: HashMap::new(),
            state,
        }
    }

    pub fn route(mut self, method: Method, path: &str, handler: impl Handler<S>) -> Self {
        self.routes
            .entry(path.to_string())
            .or_default()
            .insert(method, Box::new(handler));
        self
    }

    pub fn get(self, path: &str, handler: impl Handler<S>) -> Self {
        self.route(Method::Get, path, handler)
    }

    pub fn post(self, path: &str, handler: impl Handler<S>) -> Self {
        self.route(Method::Post, path, handler)
    }

    pub(crate) async fn dispatch(&self, req: Request) -> Response {
        let Some(methods) = self.routes.get(req.target.as_str()) else {
            return Response::text(404, "not found\n");
        };

        match methods.get(&req.method) {
            Some(h) => h.call(self.state.clone(), req).await,
            None => {
                let allow: Vec<&str> = methods.keys().map(|m| m.as_str()).collect();
                let mut resp = Response::text(405, "method not allowed\n");
                resp.headers.push(("Allow".to_string(), allow.join(", ")));
                resp
            }
        }
    }
}
