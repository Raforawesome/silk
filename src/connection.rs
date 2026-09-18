pub mod thread_pool;

use std::{cell::RefCell, io, io::Read as _, net::TcpStream};

use crate::http::{
    Version,
    code::Code,
    headers::Header,
    request::Request,
    response::{Response, ResponseBuilder, ResponseError},
};

thread_local! {
    static REQ_BUFFER: RefCell<Vec<u8>> = RefCell::new(vec![0; 8192]);
}

#[derive(Debug)]
pub enum ConnectionError {
    PeerClosed,
    Read(io::Error),
    ResponseBuild(ResponseError),
}

pub fn connection_dispatch(stream: &mut TcpStream) -> Result<Response, ConnectionError> {
    REQ_BUFFER.with_borrow_mut(|buf| handle_connection(stream, buf))
}

fn handle_connection(stream: &mut TcpStream, buf: &mut [u8]) -> Result<Response, ConnectionError> {
    let bytes_read = stream.read(buf).map_err(ConnectionError::Read)?;
    if bytes_read == 0 {
        return Err(ConnectionError::PeerClosed);
    }

    let buf = &buf[..bytes_read]; // trim empty bytes at end of buffer
    if let Err(code) = Request::parse(buf) {
        return error_response(code);
    }

    let content = include_bytes!("../hello.html");

    ResponseBuilder::new()
        .with_content(content.to_vec())
        .version(Version::Http1_1)
        .code(Code::Ok)
        .build()
        .map_err(ConnectionError::ResponseBuild)
}

fn error_response(code: Code) -> Result<Response, ConnectionError> {
    ResponseBuilder::new()
        .with_content(b"Request rejected\n".to_vec())
        .version(Version::Http1_1)
        .code(code)
        .header(Header::Custom("Connection".into(), "close".into()))
        .build()
        .map_err(ConnectionError::ResponseBuild)
}
