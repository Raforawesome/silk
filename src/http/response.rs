use crate::{
    ToBytes,
    http::{self, code::Code, headers::Header},
};
use bytes::BytesMut;

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
    fn write_to(&self, buffer: &mut BytesMut) {
        self.version.write_to(buffer);
        buffer.extend_from_slice(b" ");
        self.code.write_to(buffer);
        buffer.extend_from_slice(b"\r\n");

        for header in &self.headers {
            header.write_to(buffer);
            buffer.extend_from_slice(b"\r\n");
        }

        buffer.extend_from_slice(b"\r\n");
        buffer.extend_from_slice(&self.body);
    }
}

// basic builder methods
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

    pub fn extend_body(mut self, bytes: impl AsRef<[u8]>) -> Self {
        let bytes = bytes.as_ref();

        match &mut self.body {
            Some(body) => body.extend_from_slice(bytes),
            None => self.body = Some(bytes.to_vec()),
        };

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

// composite methods
impl ResponseBuilder {
    pub fn with_content(self, content: Vec<u8>) -> Self {
        self.header(Header::ContentLength(content.len()))
            .body(content)
    }
}
