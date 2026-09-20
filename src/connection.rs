pub mod thread_pool;

use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

use crate::{
    ToBytes,
    http::{
        Version,
        code::Code,
        headers::Header,
        request::{MAX_REQUEST_BYTES, ParseError, ParseStatus, RequestParser},
        response::{Response, ResponseBuilder, ResponseError},
    },
};

const RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

pub struct ConnectionContext {
    buffer: Vec<u8>,
    occupied: usize,
    parser: RequestParser,
}

impl Default for ConnectionContext {
    fn default() -> Self {
        Self {
            buffer: vec![0; MAX_REQUEST_BYTES],
            occupied: 0,
            parser: RequestParser::default(),
        }
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    PeerClosed,
    IncompleteRequest,
    Read(io::Error),
    Write(io::Error),
    ResponseBuild(ResponseError),
}

pub fn connection_dispatch(
    stream: &mut TcpStream,
    context: &mut ConnectionContext,
) -> Result<(), ConnectionError> {
    handle_connection(stream, context, RECEIVE_TIMEOUT, WRITE_TIMEOUT)
}

fn handle_connection(
    stream: &mut TcpStream,
    context: &mut ConnectionContext,
    receive_timeout: Duration,
    write_timeout: Duration,
) -> Result<(), ConnectionError> {
    context.occupied = 0;
    context.parser = RequestParser::default();
    let deadline = Instant::now() + receive_timeout;
    let response = loop {
        match context.parser.parse(&context.buffer[..context.occupied]) {
            Ok(ParseStatus::Complete { request, .. }) => {
                break crate::router::route(&request).map_err(ConnectionError::ResponseBuild)?;
            }
            Err(error) => break error_response(error)?,
            Ok(ParseStatus::Incomplete) => (),
        }
        if context.occupied == context.buffer.len() {
            break error_response(ParseError::RequestTooLarge)?;
        }
        stream
            .set_read_timeout(Some(remaining(deadline).map_err(ConnectionError::Read)?))
            .map_err(ConnectionError::Read)?;
        match stream.read(&mut context.buffer[context.occupied..]) {
            Ok(0) => {
                return Err(if context.occupied == 0 {
                    ConnectionError::PeerClosed
                } else {
                    ConnectionError::IncompleteRequest
                });
            }
            Ok(count) => context.occupied += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ConnectionError::Read(error)),
        }
        // Enforce elapsed time even when the socket returned data at the deadline.
        remaining(deadline).map_err(ConnectionError::Read)?;
    };
    let deadline = Instant::now() + write_timeout;
    let bytes = response.to_bytes();
    write_response(stream, &bytes, deadline).map_err(ConnectionError::Write)
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection deadline expired"))
}

fn write_response(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        stream.set_write_timeout(Some(remaining(deadline)?))?;
        match stream.write(bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "socket stopped accepting bytes",
                ));
            }
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    remaining(deadline)?;
    Ok(())
}

pub(crate) fn response(code: Code, body: Vec<u8>) -> Result<Response, ConnectionError> {
    ResponseBuilder::new()
        .with_content(body)
        .version(Version::Http1_1)
        .code(code)
        .header(Header::Custom("Connection".into(), "close".into()))
        .build()
        .map_err(ConnectionError::ResponseBuild)
}

fn error_response(error: ParseError) -> Result<Response, ConnectionError> {
    let code = match error {
        ParseError::UnsupportedMethod | ParseError::UnsupportedTransferEncoding => {
            Code::NotImplemented
        }
        ParseError::UnsupportedVersion => Code::HttpVersionNotSupported,
        ParseError::HeadTooLarge | ParseError::TooManyHeaders => Code::RequestHeaderFieldsTooLarge,
        ParseError::RequestTooLarge => Code::ContentTooLarge,
        _ => Code::BadRequest,
    };
    response(code, b"Request rejected\n".to_vec())
}

#[cfg(test)]
#[path = "connection/tests.rs"]
mod tests;
