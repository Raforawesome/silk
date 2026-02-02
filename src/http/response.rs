use std::fmt;

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

pub enum ResponseError {
    MissingVersion,
    MissingCode,
    MissingBody,
}

pub struct Response {
    version: http::Version,
    code: Code,
    headers: Vec<Header>,
    body: String,
}

#[derive(Default)]
pub struct ResponseBuilder {
    version: Option<http::Version>,
    code: Option<Code>,
    headers: Vec<Header>,
    body: Option<String>,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}\r\n", &self.version, &self.code)?;

        for header in &self.headers {
            write!(f, "{header}\r\n")?;
        }

        write!(f, "\r\n")?;
        write!(f, "{}", &self.body)?;

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

    pub fn body(mut self, body: String) -> Self {
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

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Code::Ok => write!(f, "200 OK"),
            Code::NotFound => write!(f, "404 Not Found"),
            Code::InternalServerError => write!(f, "500 Internal Server Error"),
            Code::BadRequest => write!(f, "400 Bad Request"),
            Code::Unauthorized => write!(f, "401 Unauthorized"),
            Code::Forbidden => write!(f, "403 Forbidden"),
            Code::MethodNotAllowed => write!(f, "405 Method Not Allowed"),
        }
    }
}
