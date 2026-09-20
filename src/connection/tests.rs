use super::*;
use crate::test_support::bounded;
use std::{
    net::{Shutdown, TcpListener},
    sync::mpsc,
    thread,
};

fn pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let server = listener.accept().unwrap().0;
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    (client, server)
}

#[test]
fn fragmented_request_waits_for_body() {
    bounded(
        "connection::tests::fragmented_request_waits_for_body",
        || {
            let raw = b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 3\r\n\r\nabc";
            let (mut client, mut server) = pair();
            let handle = thread::spawn(move || {
                connection_dispatch(&mut server, &mut ConnectionContext::default())
            });
            client.write_all(&raw[..15]).unwrap();
            client.write_all(&raw[15..raw.len() - 1]).unwrap();
            client
                .set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let error = client.read(&mut [0]).unwrap_err();
            assert!(matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ));
            client.write_all(&raw[raw.len() - 1..]).unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut fragmented = Vec::new();
            client.read_to_end(&mut fragmented).unwrap();
            handle.join().unwrap().unwrap();
            let (mut client, mut server) = pair();
            let handle = thread::spawn(move || {
                connection_dispatch(&mut server, &mut ConnectionContext::default())
            });
            client.write_all(raw).unwrap();
            let mut contiguous = Vec::new();
            client.read_to_end(&mut contiguous).unwrap();
            handle.join().unwrap().unwrap();
            assert_eq!(fragmented, contiguous);
            assert!(fragmented.starts_with(b"HTTP/1.1 200"));
            assert!(
                fragmented
                    .windows(19)
                    .any(|w| w == b"Connection: close\r\n")
            );
        },
    );
}

#[test]
fn eof_limits_and_context_reuse() {
    bounded("connection::tests::eof_limits_and_context_reuse", || {
        let mut context = ConnectionContext::default();
        let storage = context.buffer.as_ptr();
        for bytes in [b"".as_slice(), b"GET / HTTP/1.1\r\n"] {
            let (mut client, mut server) = pair();
            client.write_all(bytes).unwrap();
            client.shutdown(Shutdown::Write).unwrap();
            let error = connection_dispatch(&mut server, &mut context).unwrap_err();
            assert!(matches!(
                error,
                ConnectionError::PeerClosed | ConnectionError::IncompleteRequest
            ));
        }
        for (bytes, status) in [
            (
                b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 65536\r\n\r\n".as_slice(),
                b"HTTP/1.1 413".as_slice(),
            ),
            (b"GET / HTTP/1.1\r\nHost: a\r\n\r\n", b"HTTP/1.1 200"),
        ] {
            let (mut client, mut server) = pair();
            client.write_all(bytes).unwrap();
            connection_dispatch(&mut server, &mut context).unwrap();
            drop(server);
            let mut response = Vec::new();
            client.read_to_end(&mut response).unwrap();
            assert!(response.starts_with(status));
            assert_eq!(context.buffer.as_ptr(), storage);
        }
    });
}

#[test]
fn idle_trickling_and_blocked_writes_expire() {
    bounded(
        "connection::tests::idle_trickling_and_blocked_writes_expire",
        || {
            let (_client, mut server) = pair();
            let error = handle_connection(
                &mut server,
                &mut ConnectionContext::default(),
                Duration::from_millis(150),
                WRITE_TIMEOUT,
            )
            .unwrap_err();
            assert!(matches!(error, ConnectionError::Read(_)));

            let (mut client, mut server) = pair();
            let (done_tx, done_rx) = mpsc::channel();
            let handle = thread::spawn(move || {
                let result = handle_connection(
                    &mut server,
                    &mut ConnectionContext::default(),
                    Duration::from_millis(200),
                    WRITE_TIMEOUT,
                );
                done_tx.send(result).unwrap();
            });
            loop {
                match done_rx.recv_timeout(Duration::from_millis(20)) {
                    Ok(result) => {
                        assert!(matches!(result, Err(ConnectionError::Read(_))));
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let _ = client.write_all(b"G");
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            handle.join().unwrap();
            let (_client, mut server) = pair();
            // More than the loopback send buffer; the peer intentionally never reads.
            let bytes = vec![0; 16 * 1024 * 1024];
            let error = write_response(
                &mut server,
                &bytes,
                Instant::now() + Duration::from_millis(150),
            )
            .unwrap_err();
            assert!(matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ));
        },
    );
}
