use memchr::{memchr, memmem::find as memmem};

use crate::http::{Method, Version, code::Code};

/// Represents a request parsed from a buffer. The lifetime
/// of this struct is tied to its buffer.
#[derive(Debug)]
pub struct Request<'a> {
    method: Method,
    path: &'a [u8],
    http_version: Version,
    headers: &'a [u8],
    body: &'a [u8],
}

/// Separates the head from the body of a request, returning [None] if
/// no `\r\n\r\n` boundary was found.
fn guillotine(request: &[u8]) -> Option<(&[u8], &[u8])> {
    let boundary = memmem(request, b"\r\n\r\n");
    boundary.map(|i| (&request[..i], &request[i + 4..]))
}

/// Holds information about the status line (first line) of an HTTP request
struct StatusLine<'a> {
    method: Method,
    path: &'a [u8],
    http_version: Version,
}

fn parse_status_line<'a>(status_line: &'a [u8]) -> Result<StatusLine<'a>, Code> {
    let s1 = memchr(b' ', status_line).ok_or(Code::BadRequest)?;
    let s2 = s1 + 1 + memchr(b' ', &status_line[s1 + 1..]).ok_or(Code::BadRequest)?;

    let method = Method::try_from(&status_line[0..s1])?;
    let path = &status_line[s1 + 1..s2];
    let http_version = Version::try_from(&status_line[s2 + 1..])?;

    Ok(StatusLine {
        method,
        path,
        http_version,
    })
}

fn parse_head<'a>(head: &'a [u8]) -> Result<(StatusLine<'a>, &'a [u8]), Code> {
    let mut end = memchr(b'\n', head).ok_or(Code::BadRequest)?;
    if end > 0 && head[end - 1] == b'\r' {
        end -= 1;
    }

    let status_line = &head[..end];
    let header_block = head.get(end + 2..).unwrap_or_default();

    let status_line = parse_status_line(status_line)?;
    Ok((status_line, header_block))
}

impl<'a> Request<'a> {
    pub fn parse(buffer: &'a [u8]) -> Result<Request<'a>, Code> {
        let (head, body) = guillotine(buffer).ok_or(Code::BadRequest)?;
        let (status_line, headers) = parse_head(head)?;

        let StatusLine {
            method,
            path,
            http_version,
        } = status_line;

        let request = Request {
            method,
            path,
            http_version,
            headers,
            body,
        };
        Ok(request)
    }
}

impl<'a> Request<'a> {
    pub fn method(&self) -> Method {
        self.method
    }

    pub fn path(&self) -> &[u8] {
        self.path
    }

    pub fn http_version(&self) -> Version {
        self.http_version
    }

    pub fn headers(&self) -> &[u8] {
        self.headers
    }

    pub fn body(&self) -> &[u8] {
        self.body
    }
}
