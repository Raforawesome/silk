use std::io::Write;

pub mod http;

pub trait ToBytes {
    fn write_to(&self, buffer: &mut impl Write) -> std::io::Result<()>;

    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        self.write_to(&mut buffer).unwrap();
        buffer
    }
}
