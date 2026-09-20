use super::*;

fn complete(raw: &[u8]) -> (Request<'_>, usize) {
    match Request::parse(raw).expect("valid request") {
        ParseStatus::Complete { request, consumed } => (request, consumed),
        ParseStatus::Incomplete => panic!("expected complete request"),
    }
}

fn invalid(raw: &[u8], expected: ParseError) {
    assert_eq!(Request::parse(raw).unwrap_err(), expected, "{raw:?}");
}

#[test]
fn every_split_point_of_get_and_fixed_length_body() {
    for raw in [
        b"GET /?q=x HTTP/1.1\r\nHost: example.com\r\n\r\n".as_slice(),
        b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\n\r\na\0\r\n",
    ] {
        for split in 0..raw.len() {
            assert!(
                matches!(Request::parse(&raw[..split]), Ok(ParseStatus::Incomplete)),
                "split {split}"
            );
        }
        assert_eq!(complete(raw).1, raw.len());
    }
}

#[test]
fn borrows_target_headers_and_exact_body() {
    let raw = b"POST /echo?q=yes HTTP/1.1\r\nhOsT:\t localhost \t\r\ncontent-LENGTH: 3\r\nX-Empty:\t\r\n\r\nabcNEXT";
    let (request, consumed) = complete(raw);
    assert_eq!(request.path(), b"/echo?q=yes");
    assert_eq!(request.header(b"HOST"), Some(b"localhost".as_slice()));
    assert_eq!(request.header(b"x-empty"), Some(b"".as_slice()));
    assert_eq!(request.header_iter().count(), 3);
    assert_eq!(request.body(), b"abc");
    assert_eq!(&raw[consumed..], b"NEXT");
    for slice in [
        request.path(),
        request.headers(),
        request.body(),
        request.header(b"host").unwrap(),
    ] {
        let offset = slice.as_ptr() as usize - raw.as_ptr() as usize;
        assert!(offset + slice.len() <= raw.len());
        assert_eq!(&raw[offset..offset + slice.len()], slice);
    }
}

#[test]
fn concatenated_requests_and_unframed_bytes_are_not_a_body() {
    let first = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let raw = [first.as_slice(), first.as_slice()].concat();
    let (request, consumed) = complete(&raw);
    assert!(request.body().is_empty());
    assert_eq!(consumed, first.len());
    assert_eq!(complete(&raw[consumed..]).1, first.len());
    assert!(
        complete(b"PUT / HTTP/1.1\r\nHost: a\r\nContent-Length: 0\r\n\r\nignored")
            .0
            .body()
            .is_empty()
    );
}

#[test]
fn empty_header_syntax_is_separate_from_host_policy() {
    assert_eq!(validate_headers(b"", 0), Ok(()));
    invalid(b"GET / HTTP/1.1\r\n\r\n", ParseError::InvalidHost);
}

#[test]
fn rejects_malformed_lines_targets_and_headers() {
    for line in [
        "GET  HTTP/1.1",
        "GET * HTTP/1.1",
        "GET http://a/ HTTP/1.1",
        "GET /#fragment HTTP/1.1",
        "GET /%zz HTTP/1.1",
        "GET /% HTTP/1.1",
        "GET /a\\b HTTP/1.1",
        "G@T / HTTP/1.1",
        "GET / HTTP/1.1 extra",
        "GET / HTTP/one",
        "GET /\t HTTP/1.1",
    ] {
        invalid(
            format!("{line}\r\nHost: a\r\n\r\n").as_bytes(),
            ParseError::BadRequest,
        );
    }
    for field in [
        b"NoColon".as_slice(),
        b"Bad Name: x",
        b"Host : a",
        b": x",
        b" folded",
        b"X: a\0b",
        b"X: \x7f",
    ] {
        let raw = [
            b"GET / HTTP/1.1\r\nHost: a\r\n".as_slice(),
            field,
            b"\r\n\r\n",
        ]
        .concat();
        invalid(&raw, ParseError::BadRequest);
    }
    for raw in [
        b"GET / HTTP/1.1\nHost: a\n\n".as_slice(),
        b"GET / HTTP/1.1\rX",
        b"GET / HTTP/1.1\r\nHost: a\nb\r\n\r\n",
    ] {
        invalid(raw, ParseError::BadRequest);
    }
    complete(b"GET /a%20b?x=? HTTP/1.1\r\nHost: a\r\nX: \xff\r\n\r\n");
}

#[test]
fn unsupported_methods_and_versions_are_distinct() {
    invalid(
        b"PATCH / HTTP/1.1\r\nHost: a\r\n\r\n",
        ParseError::UnsupportedMethod,
    );
    invalid(
        b"GET / HTTP/1.0\r\nHost: a\r\n\r\n",
        ParseError::UnsupportedVersion,
    );
}

#[test]
fn validates_host_authority_and_duplicates() {
    for host in [
        "", "a b", "a,b", "user@a", "a/path", "a:bad", "[bad]", "a%xy", "::1",
    ] {
        invalid(
            format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes(),
            ParseError::InvalidHost,
        );
    }
    for host in ["localhost", "example.com:8080", "127.0.0.1", "[::1]:80"] {
        complete(format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes());
    }
    invalid(
        b"GET / HTTP/1.1\r\nHost: a\r\nhost: a\r\n\r\n",
        ParseError::InvalidHost,
    );
}

#[test]
fn rejects_ambiguous_or_unsupported_framing() {
    for (fields, error) in [
        (
            "Content-Length: 1\r\nContent-Length: 1",
            ParseError::DuplicateContentLength,
        ),
        (
            "Content-Length: 1\r\nContent-Length: 2",
            ParseError::DuplicateContentLength,
        ),
        ("Content-Length: 1, 1", ParseError::InvalidContentLength),
        ("Content-Length: -1", ParseError::InvalidContentLength),
        ("Content-Length: +1", ParseError::InvalidContentLength),
        ("Content-Length:", ParseError::InvalidContentLength),
        ("Content-Length: 1 0", ParseError::InvalidContentLength),
        (
            "Transfer-Encoding: chunked",
            ParseError::UnsupportedTransferEncoding,
        ),
        (
            "Transfer-Encoding: identity\r\nContent-Length: 0",
            ParseError::ConflictingFraming,
        ),
        (
            "Content-Length: 0\r\nTransfer-Encoding: chunked",
            ParseError::ConflictingFraming,
        ),
        (
            "Content-Length: 999999999999999999999999999999999999",
            ParseError::RequestTooLarge,
        ),
        ("Content-Length: 65536", ParseError::RequestTooLarge),
    ] {
        invalid(
            format!("POST / HTTP/1.1\r\nHost: a\r\n{fields}\r\n\r\n").as_bytes(),
            error,
        );
    }
    let raw = format!(
        "POST / HTTP/1.1\r\nHost: a\r\nContent-Length: {}\r\n\r\n",
        usize::MAX
    );
    invalid(raw.as_bytes(), ParseError::RequestTooLarge);
}

#[test]
fn exact_limits_and_oversized_partial_heads() {
    let raw = b"GET / HTTP/1.1\r\nHost: a\r\n\r\n";
    assert!(matches!(
        Request::parse_with_limits(raw, raw.len(), 1, raw.len()),
        Ok(ParseStatus::Complete { .. })
    ));
    for (head_bytes, fields, request_bytes, error) in [
        (raw.len() - 1, 1, raw.len(), ParseError::HeadTooLarge),
        (raw.len(), 0, raw.len(), ParseError::TooManyHeaders),
        (raw.len(), 1, raw.len() - 1, ParseError::RequestTooLarge),
    ] {
        assert_eq!(
            Request::parse_with_limits(raw, head_bytes, fields, request_bytes).unwrap_err(),
            error
        );
    }
    assert_eq!(
        Request::parse_with_limits(b"GET /long", 8, 1, raw.len()).unwrap_err(),
        ParseError::HeadTooLarge
    );
    let body = b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 2\r\n\r\nxy";
    assert!(matches!(
        Request::parse_with_limits(body, MAX_HEAD_BYTES, MAX_HEADER_FIELDS, body.len()),
        Ok(ParseStatus::Complete { .. })
    ));
    assert_eq!(
        Request::parse_with_limits(body, MAX_HEAD_BYTES, MAX_HEADER_FIELDS, body.len() - 1)
            .unwrap_err(),
        ParseError::RequestTooLarge
    );
}
