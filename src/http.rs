pub mod code;
pub mod headers;
pub mod response;

use bytes::BytesMut;

use crate::ToBytes;

pub enum Version {
    Http1_1,
    Http2_0,
}

pub enum Method {
    Post,
    Get,
}

impl ToBytes for Version {
    fn write_to(&self, buffer: &mut BytesMut) {
        match self {
            Version::Http1_1 => buffer.extend_from_slice(b"HTTP/1.1"),
            Version::Http2_0 => buffer.extend_from_slice(b"HTTP/2.0"),
        }
    }
}

impl ToBytes for Method {
    fn write_to(&self, buffer: &mut BytesMut) {
        match self {
            Method::Post => buffer.extend_from_slice(b"POST"),
            Method::Get => buffer.extend_from_slice(b"GET"),
        }
    }
}
