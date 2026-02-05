use crate::ToBytes;
use std::io::Write;

use crate::http::{self, headers::Header};

pub enum Code {
    Ok,
    NotFound,
    InternalServerError,
    BadRequest,
    Unauthorized,
    Forbidden,
    MethodNotAllowed,
}

#[derive(Debug)]
pub enum ResponseError {
    MissingVersion,
    MissingCode,
    MissingBody,
}

pub struct Response {
    version: http::Version,
    code: Code,
    headers: Vec<Header>,
    body: Vec<u8>,
}

#[derive(Default)]
pub struct ResponseBuilder {
    version: Option<http::Version>,
    code: Option<Code>,
    headers: Vec<Header>,
    body: Option<Vec<u8>>,
}

impl ToBytes for Response {
    fn write_to(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        self.version.write_to(buffer)?;
        buffer.write_all(b" ")?;
        self.code.write_to(buffer)?;
        buffer.write_all(b"\r\n")?;

        for header in &self.headers {
            header.write_to(buffer)?;
            buffer.write_all(b"\r\n")?;
        }

        buffer.write_all(b"\r\n")?;
        buffer.write_all(&self.body)?;

        Ok(())
    }
}

impl ResponseBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(mut self, version: http::Version) -> Self {
        self.version = Some(version);
        self
    }

    pub fn code(mut self, code: Code) -> Self {
        self.code = Some(code);
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn header(mut self, header: Header) -> Self {
        self.headers.push(header);
        self
    }

    pub fn headers(mut self, headers: &[Header]) -> Self {
        self.headers.extend_from_slice(headers);
        self
    }

    pub fn build(self) -> Result<Response, ResponseError> {
        let version = self.version.ok_or(ResponseError::MissingVersion)?;
        let code = self.code.ok_or(ResponseError::MissingCode)?;
        let body = self.body.ok_or(ResponseError::MissingBody)?;

        Ok(Response {
            version,
            code,
            headers: self.headers,
            body,
        })
    }
}

impl ToBytes for Code {
    fn write_to(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Code::Ok => write!(buffer, "200 OK"),
            Code::NotFound => write!(buffer, "404 Not Found"),
            Code::InternalServerError => write!(buffer, "500 Internal Server Error"),
            Code::BadRequest => write!(buffer, "400 Bad Request"),
            Code::Unauthorized => write!(buffer, "401 Unauthorized"),
            Code::Forbidden => write!(buffer, "403 Forbidden"),
            Code::MethodNotAllowed => write!(buffer, "405 Method Not Allowed"),
        }
    }
}
