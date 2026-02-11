use crate::ToBytes;
use bytes::BytesMut;

#[derive(Debug)]
pub enum Code {
    Ok,
    NotFound,
    InternalServerError,
    BadRequest,
    Unauthorized,
    Forbidden,
    MethodNotAllowed,
    HttpVersionNotSupported,
}

impl ToBytes for Code {
    fn write_to(&self, buffer: &mut BytesMut) {
        match self {
            Code::Ok => buffer.extend_from_slice(b"200 OK"),
            Code::NotFound => buffer.extend_from_slice(b"404 Not Found"),
            Code::InternalServerError => buffer.extend_from_slice(b"500 Internal Server Error"),
            Code::BadRequest => buffer.extend_from_slice(b"400 Bad Request"),
            Code::Unauthorized => buffer.extend_from_slice(b"401 Unauthorized"),
            Code::Forbidden => buffer.extend_from_slice(b"403 Forbidden"),
            Code::MethodNotAllowed => buffer.extend_from_slice(b"405 Method Not Allowed"),
            Code::HttpVersionNotSupported => {
                buffer.extend_from_slice(b"505 HTTP Version Not Supported")
            }
        }
    }
}
