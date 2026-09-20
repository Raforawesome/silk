# Week A — Borrowed parsing and exact framing

## Behavior and code map

`src/http/request.rs` now exposes:

- `Request::parse(&[u8]) -> Result<ParseStatus<'_>, ParseError>`.
- Private `Request::parse_with_limits(buffer, max_head_bytes, max_header_fields, max_request_bytes)` shares the implementation with small-boundary tests.
- `ParseStatus::{Incomplete, Complete { request, consumed }}`; invalid input is `Err(ParseError)`.
- Three module constants set defaults: 8192 bytes through CRLFCRLF, 64 fields, and 65536 total framed bytes. No public limits configuration type.
- Existing accessors, plus `header_iter()` and case-insensitive `header(&[u8])`.

`src/http/headers.rs` adds `RequestHeader` and `RequestHeaders`. Raw `headers()` now includes each field's trailing CRLF, excluding the final empty line. Header syntax accepts an empty block; HTTP/1.1 policy separately requires exactly one valid, nonempty Host. Targets retain their query and validate origin-form URI bytes and percent escapes. Supported methods remain GET/POST/PUT.

`src/connection.rs` translates parser errors at the socket boundary: malformed input/incomplete input to 400, unsupported methods/transfer encoding to 501, unsupported versions to 505, head/field limits to 431, and total size to 413. The latter status variants were added in `src/http/code.rs`. Error responses already send Connection: close and the worker drops the socket.

## Ownership and trace

The caller owns all bytes. Target, raw fields, trimmed values, and body borrow slices from that input. For a POST declaring length 3 followed by `abcNEXT`, completion borrows `abc`; `consumed` points at `N`. With only `ab`, parsing returns Incomplete. Without Content-Length the body is empty and all bytes after the head remain unconsumed. No parser allocation is visible in this implementation; allocation instrumentation remains Task E.

## Verification

On rustc 1.96.0-nightly (e0e95a718 2026-04-04):

- `rustfmt --edition 2024 src/http/request.rs src/http/request_tests.rs src/http/headers.rs src/http/code.rs src/connection.rs src/tests.rs` completed.
- `cargo check --all-targets` passed.
- `cargo test` passed: nine regression tests, seven legacy timing tests ignored, no failures. Cases cover every split point, borrowed pointers, concatenation, Host, malformed lines/fields/targets, framing conflicts, overflow, and exact/smaller limits.
- `git diff --check` passed.

The eleven pre-existing warnings remain in the pool/router and unused feature gates. Timing tests in `src/tests.rs` are explicitly ignored because their fixtures/accounting are unreliable; Task E must replace them. No timing, allocation, live socket, or stable-toolchain evidence is claimed here.

## Tradeoff and Task B handoff

Strict duplicate Content-Length rejection sacrifices tolerance for an unambiguous body boundary. All Transfer-Encoding is unsupported. Host rejects comma-joined values and supports IPv6 literals but not IPvFuture or zone identifiers. This is a documented subset, not full HTTP compliance. Reference: [RFC 9112](https://www.rfc-editor.org/rfc/rfc9112.html).

The public parser is stateless and searches/validates again on each call; syntax validation largely waits for a complete head (bad CRLF can fail earlier). Task B needs shared internal head metadata expressed as offsets and expected length, with delimiter-search overlap, so body arrivals do not trigger reparsing. Do not store a borrowed Request across buffer writes.

The temporary connection implementation still reads once into 8192 bytes and rejects incomplete requests with 400. Incremental reads, deadlines, and worker-owned contexts remain Task B; successful responses still need its explicit Connection: close header. Review this milestone before adding that lifecycle work.

Suggested commit: `feat(http): validate borrowed requests and enforce exact body framing`.
