use crate::ToBytes;
use std::io::Write;

#[derive(Clone)]
pub enum Header {
    ContentLength(usize),
    Custom(String, String),
}

impl ToBytes for Header {
    fn write_to(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Header::ContentLength(length) => write!(buffer, "Content-Length: {length}"),
            Header::Custom(name, value) => write!(buffer, "{name}: {value}"),
        }
    }
}
