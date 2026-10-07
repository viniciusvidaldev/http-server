# myhttp

A small HTTP/1.1 server built from scratch on top of Tokio, written as a study project.

## Features

- Request parsing: request line, headers and `Content-Length` bodies
- Router with per-path, per-method handlers and shared state
- `404 Not Found` and `405 Method Not Allowed` (with an `Allow` header)
- Response compression with `gzip` and `deflate`, negotiated through `Accept-Encoding`
- Limits on line length (8 KiB), header count (100) and body size (1 MiB)

## Running

```sh
cargo run
```

The server listens on `127.0.0.1:8080`.

```sh
curl localhost:8080/                      # hello world
curl localhost:8080/count                 # visits: 1
curl -d 'ping' localhost:8080/echo        # ping
curl --compressed -v localhost:8080/      # Content-Encoding: gzip
```

## Usage

Handlers are async functions that take the router state and a `Request`, and return a `Response`:

```rust
use myhttp::{Request, Response, Router, serve};

async fn hello(_state: (), _req: Request) -> Response {
    Response::text(200, "hello world")
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = Router::new(()).get("/", hello);
    serve("127.0.0.1:8080", app).await
}
```

## Layout

| File              | Purpose                                         |
| ----------------- | ----------------------------------------------- |
| `src/lib.rs`      | `Handler` trait, `serve` and connection handling |
| `src/conn.rs`     | Request parsing and response writing            |
| `src/router.rs`   | Path/method routing                             |
| `src/encoding.rs` | `gzip`/`deflate` negotiation and compression    |
| `src/main.rs`     | Example app                                     |

## Limitations

- One request per connection (no keep-alive)
- No chunked transfer encoding
- `Accept-Encoding` q-values are ignored; the first supported encoding wins
