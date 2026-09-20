use bytes::{BufMut as _, BytesMut};

use crate::ToBytes;

#[derive(Clone)]
pub enum Header {
    ContentLength(usize),
    Custom(String, String),
}

impl ToBytes for Header {
    fn write_to(&self, buffer: &mut BytesMut) {
        match self {
            Header::ContentLength(length) => format_content_length(buffer, length),
            Header::Custom(name, value) => format_custom_header(buffer, name, value),
        }
    }
}

fn format_content_length(buffer: &mut BytesMut, length: &usize) {
    buffer.put_slice(b"Content-Length: ");
    buffer.put_slice(itoa::Buffer::new().format(*length).as_bytes());
}

fn format_custom_header(buffer: &mut BytesMut, name: &String, value: &String) {
    buffer.put_slice(name.as_bytes());
    buffer.put_slice(b": ");
    buffer.put_slice(value.as_bytes());
}

/// A validated request field borrowed from the receive buffer.
#[derive(Debug, Clone, Copy)]
pub struct RequestHeader<'a> {
    name: &'a [u8],
    value: &'a [u8],
}

impl<'a> RequestHeader<'a> {
    pub fn name(&self) -> &'a [u8] {
        self.name
    }
    pub fn value(&self) -> &'a [u8] {
        self.value
    }
}

pub struct RequestHeaders<'a> {
    remaining: &'a [u8],
}

impl<'a> RequestHeaders<'a> {
    // only constructed after request-head validation
    pub(crate) fn new(block: &'a [u8]) -> Self {
        Self { remaining: block }
    }
}

impl<'a> Iterator for RequestHeaders<'a> {
    type Item = RequestHeader<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let end = memchr::memmem::find(self.remaining, b"\r\n")?;
        let line = &self.remaining[..end];
        let colon = memchr::memchr(b':', line)?;
        self.remaining = &self.remaining[end + 2..];
        Some(RequestHeader {
            name: &line[..colon],
            value: trim_ows(&line[colon + 1..]),
        })
    }
}

pub(crate) fn trim_ows(value: &[u8]) -> &[u8] {
    let start = value
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(value.len());
    let value = &value[start..];
    let end = value
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t')
        .map_or(0, |i| i + 1);
    &value[..end]
}

pub(crate) fn is_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}
