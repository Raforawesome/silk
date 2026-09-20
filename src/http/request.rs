use memchr::{memchr, memmem::find};

use crate::http::{
    Method, Version,
    headers::{RequestHeaders, is_token, trim_ows},
};

// Head bytes include CRLFCRLF; request bytes count only the first message.
const MAX_HEAD_BYTES: usize = 8 * 1024;
const MAX_HEADER_FIELDS: usize = 64;
const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    BadRequest,
    UnsupportedMethod,
    UnsupportedVersion,
    InvalidHost,
    InvalidContentLength,
    DuplicateContentLength,
    ConflictingFraming,
    UnsupportedTransferEncoding,
    HeadTooLarge,
    TooManyHeaders,
    RequestTooLarge,
}

#[derive(Debug)]
pub enum ParseStatus<'a> {
    Incomplete,
    Complete {
        request: Request<'a>,
        consumed: usize,
    },
}

/// Every slice borrows the caller's input; no body bytes are copied or scanned.
#[derive(Debug)]
pub struct Request<'a> {
    method: Method,
    path: &'a [u8],
    http_version: Version,
    headers: &'a [u8],
    body: &'a [u8],
}

impl<'a> Request<'a> {
    pub fn parse(buffer: &'a [u8]) -> Result<ParseStatus<'a>, ParseError> {
        Self::parse_with_limits(buffer, MAX_HEAD_BYTES, MAX_HEADER_FIELDS, MAX_REQUEST_BYTES)
    }

    fn parse_with_limits(
        buffer: &'a [u8],
        max_head_bytes: usize,
        max_header_fields: usize,
        max_request_bytes: usize,
    ) -> Result<ParseStatus<'a>, ParseError> {
        let search_limit = max_head_bytes.min(max_request_bytes);
        let head_end = match find(&buffer[..buffer.len().min(search_limit)], b"\r\n\r\n") {
            Some(end) => end + 4,
            None => {
                validate_crlf(&buffer[..buffer.len().min(search_limit)], true)?;
                if buffer.len() >= search_limit {
                    return Err(if max_head_bytes <= max_request_bytes {
                        ParseError::HeadTooLarge
                    } else {
                        ParseError::RequestTooLarge
                    });
                }
                return Ok(ParseStatus::Incomplete);
            }
        };
        let head = &buffer[..head_end - 2];
        validate_crlf(head, false)?;
        let line_end = find(head, b"\r\n").ok_or(ParseError::BadRequest)?;
        let (method, path) = parse_request_line(&head[..line_end])?;
        // Includes each field's CRLF; the empty block is valid syntax.
        let headers = &head[line_end + 2..];
        validate_headers(headers, max_header_fields)?;
        let mut host = None;
        let mut content_length = None;
        let mut transfer_encoding = false;
        for field in RequestHeaders::new(headers) {
            if field.name().eq_ignore_ascii_case(b"host") {
                if host.replace(field.value()).is_some() {
                    return Err(ParseError::InvalidHost);
                }
            } else if field.name().eq_ignore_ascii_case(b"content-length") {
                if content_length.is_some() {
                    return Err(ParseError::DuplicateContentLength);
                }
                content_length = Some(parse_length(field.value())?);
            } else if field.name().eq_ignore_ascii_case(b"transfer-encoding") {
                transfer_encoding = true;
            }
        }
        if transfer_encoding {
            return Err(if content_length.is_some() {
                ParseError::ConflictingFraming
            } else {
                ParseError::UnsupportedTransferEncoding
            });
        }
        if !host.is_some_and(valid_host) {
            return Err(ParseError::InvalidHost);
        }
        let consumed = head_end
            .checked_add(content_length.unwrap_or(0))
            .filter(|&size| size <= max_request_bytes)
            .ok_or(ParseError::RequestTooLarge)?;
        if buffer.len() < consumed {
            return Ok(ParseStatus::Incomplete);
        }
        Ok(ParseStatus::Complete {
            request: Self {
                method,
                path,
                http_version: Version::Http1_1,
                headers,
                body: &buffer[head_end..consumed],
            },
            consumed,
        })
    }

    pub fn method(&self) -> Method {
        self.method
    }
    /// Full origin-form target, including any query string.
    pub fn path(&self) -> &[u8] {
        self.path
    }
    pub fn http_version(&self) -> Version {
        self.http_version
    }
    /// Raw fields, including each field's trailing CRLF.
    pub fn headers(&self) -> &[u8] {
        self.headers
    }
    pub fn header_iter(&self) -> RequestHeaders<'a> {
        RequestHeaders::new(self.headers)
    }
    pub fn header(&self, name: &[u8]) -> Option<&'a [u8]> {
        self.header_iter()
            .find(|field| field.name().eq_ignore_ascii_case(name))
            .map(|field| field.value())
    }
    pub fn body(&self) -> &[u8] {
        self.body
    }
}

fn validate_crlf(bytes: &[u8], partial: bool) -> Result<(), ParseError> {
    for (i, &byte) in bytes.iter().enumerate() {
        if byte == b'\n' && (i == 0 || bytes[i - 1] != b'\r') {
            return Err(ParseError::BadRequest);
        }
        if byte == b'\r' && bytes.get(i + 1) != Some(&b'\n') && !(partial && i + 1 == bytes.len()) {
            return Err(ParseError::BadRequest);
        }
    }
    Ok(())
}

fn parse_request_line(line: &[u8]) -> Result<(Method, &[u8]), ParseError> {
    let first = memchr(b' ', line).ok_or(ParseError::BadRequest)?;
    let second = first + 1 + memchr(b' ', &line[first + 1..]).ok_or(ParseError::BadRequest)?;
    let token = &line[..first];
    if token.is_empty() || !token.iter().copied().all(is_token) {
        return Err(ParseError::BadRequest);
    }
    let target = &line[first + 1..second];
    if !target.starts_with(b"/") || !valid_uri_bytes(target, true) {
        return Err(ParseError::BadRequest);
    }
    let version = &line[second + 1..];
    if version.len() != 8
        || &version[..5] != b"HTTP/"
        || !version[5].is_ascii_digit()
        || version[6] != b'.'
        || !version[7].is_ascii_digit()
    {
        return Err(ParseError::BadRequest);
    }
    if version != b"HTTP/1.1" {
        return Err(ParseError::UnsupportedVersion);
    }
    let method = match token {
        b"GET" => Method::Get,
        b"POST" => Method::Post,
        b"PUT" => Method::Put,
        _ => return Err(ParseError::UnsupportedMethod),
    };
    Ok((method, target))
}

fn validate_headers(mut block: &[u8], max_fields: usize) -> Result<(), ParseError> {
    let mut count = 0;
    while !block.is_empty() {
        let end = find(block, b"\r\n").ok_or(ParseError::BadRequest)?;
        let line = &block[..end];
        let colon = memchr(b':', line).ok_or(ParseError::BadRequest)?;
        if colon == 0
            || !line[..colon].iter().copied().all(is_token)
            || !line[colon + 1..]
                .iter()
                .all(|&b| b == b'\t' || (b >= 32 && b != 127))
        {
            return Err(ParseError::BadRequest);
        }
        count += 1;
        if count > max_fields {
            return Err(ParseError::TooManyHeaders);
        }
        block = &block[end + 2..];
    }
    Ok(())
}

fn parse_length(value: &[u8]) -> Result<usize, ParseError> {
    if value.is_empty() {
        return Err(ParseError::InvalidContentLength);
    }
    value.iter().try_fold(0usize, |length, &digit| {
        if !digit.is_ascii_digit() {
            return Err(ParseError::InvalidContentLength);
        }
        length
            .checked_mul(10)
            .and_then(|n| n.checked_add((digit - b'0') as usize))
            .ok_or(ParseError::RequestTooLarge)
    })
}

fn valid_uri_bytes(bytes: &[u8], target: bool) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            if !bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                || !bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit)
            {
                return false;
            }
            i += 3;
            continue;
        }
        if !(b.is_ascii_alphanumeric()
            || b"-._~!$&'()*+,;=".contains(&b)
            || (target && b":@/?".contains(&b)))
        {
            return false;
        }
        i += 1;
    }
    true
}

fn valid_host(value: &[u8]) -> bool {
    let value = trim_ows(value);
    // Reject comma-joined Host values under the strict single-authority policy.
    if value.is_empty() || value.contains(&b',') {
        return false;
    }
    let port = if value[0] == b'[' {
        let Some(end) = memchr(b']', value) else {
            return false;
        };
        // This subset accepts IPv6 literals, but not IPvFuture or zone identifiers.
        if std::str::from_utf8(&value[1..end])
            .ok()
            .and_then(|s| s.parse::<std::net::Ipv6Addr>().ok())
            .is_none()
        {
            return false;
        }
        &value[end + 1..]
    } else {
        let end = memchr(b':', value).unwrap_or(value.len());
        if end == 0 || !valid_uri_bytes(&value[..end], false) {
            return false;
        }
        &value[end..]
    };
    port.is_empty() || (port[0] == b':' && port[1..].iter().all(u8::is_ascii_digit))
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
