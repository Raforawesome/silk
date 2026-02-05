pub mod headers;
pub mod response;

use crate::ToBytes;
use std::io::Write;

pub enum Version {
    Http1_1,
    Http2_0,
}

pub enum Method {
    Post,
    Get,
}

impl ToBytes for Version {
    fn write_to(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Version::Http1_1 => write!(buffer, "HTTP/1.1"),
            Version::Http2_0 => write!(buffer, "HTTP/2.0"),
        }
    }
}

impl ToBytes for Method {
    fn write_to(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Method::Post => write!(buffer, "POST"),
            Method::Get => write!(buffer, "GET"),
        }
    }
}
