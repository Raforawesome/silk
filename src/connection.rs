use std::{
    cell::RefCell,
    io::{Read as _, Write as _},
    net::TcpStream,
};

use crate::{
    ToBytes as _,
    http::{Version, code::Code, headers::Header, request::Request, response::ResponseBuilder},
};

thread_local! {
    static REQ_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(8192));
}

pub fn connection_dispatch(stream: TcpStream) {
    REQ_BUFFER.with_borrow_mut(|buf| handle_connection(stream, buf));
}

pub fn handle_connection(mut stream: TcpStream, buf: &mut Vec<u8>) {
    buf.resize(8192, 0);

    match stream.read(buf) {
        Ok(0) => eprintln!("socket closed"),
        Ok(_) => {}
        Err(e) => eprintln!("error reading from socket: {}", e),
    };

    let request = Request::parse(buf).unwrap();
    dbg!(
        request.method(),
        unsafe { str::from_utf8_unchecked(request.path()) },
        request.http_version()
    );

    let content = include_bytes!("../hello.html");

    let response = ResponseBuilder::new()
        .version(Version::Http1_1)
        .code(Code::Ok)
        .header(Header::ContentLength(content.len()))
        .extend_body(content)
        .build()
        .unwrap()
        .to_bytes();

    stream.write_all(&response).unwrap();
}
