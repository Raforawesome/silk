pub mod thread_pool;

use std::{cell::RefCell, io::Read as _, net::TcpStream};

use crate::http::{
    Version,
    code::Code,
    request::Request,
    response::{Response, ResponseBuilder},
};

thread_local! {
    static REQ_BUFFER: RefCell<Vec<u8>> = RefCell::new(vec![0; 8192]);
}

pub fn connection_dispatch(stream: &mut TcpStream) -> Result<Response, ()> {
    REQ_BUFFER.with_borrow_mut(|buf| handle_connection(stream, buf))
}

fn handle_connection(stream: &mut TcpStream, buf: &mut [u8]) -> Result<Response, ()> {
    let Ok(bytes_read) = stream.read(buf) else {
        eprintln!("error reading from socket");
        return Err(());
    };
    if bytes_read == 0 {
        eprintln!("socket closed");
        return Err(());
    }

    let buf = &mut buf[..bytes_read]; // trim empty bytes at end of buffer
    let request = Request::parse(buf).unwrap();
    dbg!(
        request.method(),
        unsafe { str::from_utf8_unchecked(request.path()) },
        request.http_version(),
    );

    let content = include_bytes!("../hello.html");

    let response = ResponseBuilder::new()
        .with_content(content.to_vec())
        .version(Version::Http1_1)
        .code(Code::Ok)
        .build()
        .unwrap();

    Ok(response)
}
