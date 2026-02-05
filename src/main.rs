use std::{
    io::{BufRead as _, BufReader, Write as _},
    net::{TcpListener, TcpStream},
};

use silk::{
    ToBytes,
    http::{
        Version,
        headers::Header,
        response::{Code, ResponseBuilder},
    },
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        connection_dispatch(stream);
    }
}

pub fn connection_dispatch(mut stream: TcpStream) {
    let reader = BufReader::new(&stream);
    let http_request: Vec<_> = reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

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
