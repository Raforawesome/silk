pub mod connection;
pub mod http;
pub mod router;

use bytes::BytesMut;

/// Optional const generic parameter `S` represents initial size of byte buffer
pub trait ToBytes<const S: usize = 4096> {
    fn write_to(&self, buffer: &mut BytesMut);

    fn to_bytes(&self) -> BytesMut {
        let mut buffer = BytesMut::with_capacity(S);
        self.write_to(&mut buffer);
        buffer
    }
}

#[cfg(test)]
mod test_support;
