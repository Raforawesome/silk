pub mod code;
pub mod headers;
pub mod response;

use bytes::BytesMut;

use crate::{ToBytes, http::code::Code};

#[derive(Debug, Copy, Clone)]
pub enum Version {
    Http1_1,
}

#[derive(Debug, Copy, Clone)]
pub enum Method {
    Post,
    Get,
    Put,
}

impl TryFrom<&[u8]> for Version {
    type Error = Code;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"HTTP/1.1" => Ok(Version::Http1_1),
            _ => Err(Code::HttpVersionNotSupported),
        }
    }
}

impl TryFrom<&[u8]> for Method {
    type Error = Code;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"POST" => Ok(Method::Post),
            b"GET" => Ok(Method::Get),
            b"PUT" => Ok(Method::Put),
            _ => Err(Code::BadRequest),
        }
    }
}

impl ToBytes for Version {
    fn write_to(&self, buffer: &mut BytesMut) {
        match self {
            Version::Http1_1 => buffer.extend_from_slice(b"HTTP/1.1"),
            // Version::Http2_0 => buffer.extend_from_slice(b"HTTP/2.0"),
        }
    }
}

impl ToBytes for Method {
    fn write_to(&self, buffer: &mut BytesMut) {
        match self {
            Method::Post => buffer.extend_from_slice(b"POST"),
            Method::Get => buffer.extend_from_slice(b"GET"),
            Method::Put => buffer.extend_from_slice(b"PUT"),
        }
    }
}
