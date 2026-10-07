use std::io::{self, Write};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

const MAX_LINE: u64 = 8 * 1024;
const MAX_BODY: usize = 1024 * 1024;
const MAX_HEADERS: usize = 100;

pub struct Connection {
    stream: BufReader<TcpStream>,
    line: String,
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub target: String,
    pub version: String,
    pub headers: Headers,
    pub body: Body,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

type Headers = Vec<(String, String)>;
type Body = Vec<u8>;

pub struct Response {
    pub status: u16,
    pub headers: Headers,
    pub body: Body,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self::new(status)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(body.into())
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: BufReader::new(stream),
            line: String::with_capacity(1024),
        }
    }
    pub async fn write_response(&mut self, resp: &Response) -> io::Result<()> {
        let mut buf = Vec::with_capacity(256 + resp.body.len());

        write!(buf, "HTTP/1.1 {} \r\n", resp.status)?;
        for (name, value) in &resp.headers {
            write!(buf, "{name}: {value}\r\n")?;
        }
        write!(buf, "Content-Length: {}\r\n\r\n", resp.body.len())?;
        buf.extend_from_slice(&resp.body);

        self.stream.write_all(&buf).await?;
        self.stream.flush().await?;

        Ok(())
    }

    pub async fn read_request(&mut self) -> io::Result<Option<Request>> {
        if !self.fill_line().await? {
            return Ok(None);
        }

        let mut parts = self.line.split(' ');
        let raw = parts.next().ok_or_else(|| invalid("missing method"))?;
        let method: Method = raw
            .parse()
            .map_err(|_| invalid(format!("invalid method: {raw}")))?;

        let target = parts
            .next()
            .ok_or_else(|| invalid("missing target"))?
            .to_string();
        let version = parts
            .next()
            .ok_or_else(|| invalid("missing version"))?
            .to_string();

        if parts.next().is_some() {
            return Err(invalid("malformed request line"));
        }

        let headers = self.read_headers().await?;
        let body = self.read_body(&headers).await?;

        let req = Request {
            method,
            target,
            version,
            headers,
            body,
        };
        Ok(Some(req))
    }

    /// Reads a line into `self.line`, CRLF stripped. `Ok(false)` on clean EOF.
    async fn fill_line(&mut self) -> io::Result<bool> {
        self.line.clear();
        let n = (&mut self.stream)
            .take(MAX_LINE)
            .read_line(&mut self.line)
            .await?;

        if n == 0 {
            return Ok(false);
        }

        if !self.line.ends_with("\r\n") {
            return Err(invalid("line too long or not terminated by CRLF"));
        };

        let len = self.line.len() - 2;
        self.line.truncate(len);

        Ok(true)
    }

    async fn read_headers(&mut self) -> io::Result<Headers> {
        let mut headers = Vec::new();
        loop {
            if !self.fill_line().await? {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }

            if self.line.is_empty() {
                return Ok(headers);
            }

            if headers.len() == MAX_HEADERS {
                return Err(invalid("too many headers"));
            }

            let (name, value) = self
                .line
                .split_once(':')
                .ok_or_else(|| invalid("malformed header"))?;

            headers.push((name.to_string(), value.trim().to_string()));
        }
    }

    async fn read_body(&mut self, headers: &Headers) -> io::Result<Body> {
        let mut len: Option<usize> = None;
        for (name, value) in headers {
            if !name.eq_ignore_ascii_case("content-length") {
                continue;
            }
            let parsed: usize = value.parse().map_err(|_| invalid("bad content-length"))?;
            if len.is_some_and(|prev| prev != parsed) {
                return Err(invalid("conflicting content-length"));
            }
            len = Some(parsed);
        }

        let len = len.unwrap_or(0);
        if len > MAX_BODY {
            return Err(invalid("body too large"));
        }

        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body).await?;
        Ok(body)
    }
}

fn invalid<E>(msg: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Patch,
    Options,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Options => "OPTIONS",
        }
    }
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::Get),
            "HEAD" => Ok(Method::Head),
            "POST" => Ok(Method::Post),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            "PATCH" => Ok(Method::Patch),
            "OPTIONS" => Ok(Method::Options),
            _ => Err(()),
        }
    }
}
