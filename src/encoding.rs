use std::{
    io::{self, Read},
    str::FromStr,
};

use flate2::{
    Compression,
    read::{GzEncoder, ZlibEncoder},
};

use crate::conn::Response;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Gzip,
    Deflate,
}

impl Encoding {
    pub fn as_str(self) -> &'static str {
        match self {
            Encoding::Gzip => "gzip",
            Encoding::Deflate => "deflate",
        }
    }

    /// Supported encodings from an `Accept-Encoding` value, in order.
    /// Parameters such as `;q=0.5` are ignored.
    pub fn from_header(value: &str) -> Vec<Self> {
        value
            .split(',')
            .filter_map(|s| s.split(';').next()?.trim().parse().ok())
            .collect()
    }
}

impl FromStr for Encoding {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gzip" => Ok(Encoding::Gzip),
            "deflate" => Ok(Encoding::Deflate),
            _ => Err(()),
        }
    }
}

impl Response {
    pub fn apply_encoding(&mut self, encoding: Encoding) -> io::Result<()> {
        let mut compressed = Vec::new();
        match encoding {
            Encoding::Gzip => {
                GzEncoder::new(self.body.as_slice(), Compression::default())
                    .read_to_end(&mut compressed)?;
            }
            // HTTP "deflate" is zlib-wrapped deflate, not raw deflate.
            Encoding::Deflate => {
                ZlibEncoder::new(self.body.as_slice(), Compression::default())
                    .read_to_end(&mut compressed)?;
            }
        }
        self.body = compressed;
        self.headers
            .push(("Content-Encoding".into(), encoding.as_str().into()));
        Ok(())
    }
}
