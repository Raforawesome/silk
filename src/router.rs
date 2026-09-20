use crate::http::{
    Method, Version,
    code::Code,
    headers::Header,
    request::Request,
    response::{Response, ResponseBuilder, ResponseError},
};

/// Exact routes use the path before '?' while Request preserves the full target.
pub fn route(request: &Request<'_>) -> Result<Response, ResponseError> {
    let target = request.path();
    let path = &target[..memchr::memchr(b'?', target).unwrap_or(target.len())];
    let (code, body, allow) = match (request.method(), path) {
        (Method::Get, b"/") => (Code::Ok, include_bytes!("../hello.html").as_slice(), None),
        (Method::Get, b"/health") => (Code::Ok, b"ok\n".as_slice(), None),
        (Method::Post, b"/echo") => (Code::Ok, request.body(), None),
        (_, b"/" | b"/health") => (
            Code::MethodNotAllowed,
            b"Method not allowed\n".as_slice(),
            Some("GET"),
        ),
        (_, b"/echo") => (
            Code::MethodNotAllowed,
            b"Method not allowed\n".as_slice(),
            Some("POST"),
        ),
        _ => (Code::NotFound, b"Not found\n".as_slice(), None),
    };
    let mut builder = ResponseBuilder::new()
        .version(Version::Http1_1)
        .code(code)
        .with_content(body.to_vec())
        .header(Header::Custom("Connection".into(), "close".into()));
    if let Some(methods) = allow {
        builder = builder.header(Header::Custom("Allow".into(), methods.into()));
    }
    builder.build()
}
