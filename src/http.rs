pub mod headers;
pub mod response;

use std::fmt;

pub enum Version {
    Http1_1,
    Http2_0,
}

pub enum Method {
    Post,
    Get,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Version::Http1_1 => write!(f, "HTTP/1.1"),
            Version::Http2_0 => write!(f, "HTTP/2.0"),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Method::Post => write!(f, "POST"),
            Method::Get => write!(f, "GET"),
        }
    }
}
