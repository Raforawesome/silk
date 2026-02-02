use std::fmt;

#[derive(Clone)]
pub enum Header {
    ContentLength(usize),
    Custom(String, String),
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Header::ContentLength(length) => write!(f, "Content-Length: {length}"),
            Header::Custom(name, value) => write!(f, "{name}: {value}"),
        }
    }
}
