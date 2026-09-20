# Unit 01 — Explicit connection failures

## Behavior

The one-read demo still returns the same HTTP/1.1 200 response and embedded
page for accepted input. A parser rejection now produces the parser's status
code, a `Request rejected\n` body, its Content-Length, and `Connection: close`.
For example, `garbage\r\n\r\n` produces 400 instead of panicking.
EOF closes quietly. Read and response-construction errors are reported without
writing a response. A failed response write is reported and ends the connection;
no fallback response is attempted. Request debug output and per-connection
worker logging are removed.

## Code map

Read `ConnectionError` and `connection_dispatch` in `src/connection.rs`, then
`handle_connection` and `error_response`. Finally, read the worker's response
match in `src/connection/thread_pool.rs`. The dispatch function and pool handler
function pointer now return `Result<Response, ConnectionError>`; request and
response APIs are unchanged.

## Ownership

Each worker owns the received TcpStream until its loop iteration ends. Dropping
that stream closes the connection without a fallible explicit shutdown call.
The existing thread-local buffer is mutably borrowed during dispatch; parsing
borrows only the bytes read. Responses own their bodies, so no request-buffer
borrow escapes dispatch. Serialization still allocates its own output buffer.

## Decision

Parser errors become HTTP responses at the connection boundary. A small enum
preserves read-error details and distinguishes EOF and builder failures. Byte
parsing needs no UTF-8 conversion, so removing debug output also removes the
unchecked conversion. Existing status codes and response serialization suffice.

## Validation

- `rustfmt --edition 2024 --config skip_children=true src/connection.rs src/connection/thread_pool.rs` — succeeded.
- `cargo check --lib --bins` — succeeded with 11 existing unused-import,
  unused-variable, dead-code, and unused-feature warnings.

Tests were not added or run. No manual socket demonstration or benchmark was
run; compilation does not establish protocol correctness.

## Boundary

One-read behavior can still reject partial requests. Parser validation, framing,
timeouts, buffer reuse changes, and routing remain deferred. Worker joining and
the existing disconnected-channel loop remain Unit 02 work.

## Interview explanation

Client input now reaches explicit branches instead of unchecked conversions or
unwraps. Parser failures can receive an HTTP response, while socket failures
simply end the connection. The worker owns the socket and closes it by dropping
it. Why not send an error after a failed write? Part of the response may already
have been sent, so a second response would not reliably repair the exchange.
