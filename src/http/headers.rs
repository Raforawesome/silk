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
