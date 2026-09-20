# Silk

A small blocking HTTP/1.1-subset server in Rust. The project focuses on borrowed
request parsing, explicit message boundaries, bounded connection admission, and
worker ownership. It is a demo you can trace end to end, not a production HTTP stack.

## Build and run

Verified with stable Rust 1.94.1 on macOS (Apple M2). No nightly features are required.

```sh
cargo +stable build --release
cargo +stable run --release -- 127.0.0.1:7878
```

The address is optional; the default is `127.0.0.1:7878`. Use `127.0.0.1:0` to let the
OS select an unused port; Silk prints the actual address. Ctrl-C terminates the process.
There is no signal handler for graceful shutdown. The library exposes explicit
`ThreadPool::shutdown()` for callers that own the pool.

```sh
curl -i http://127.0.0.1:7878/
curl -i 'http://127.0.0.1:7878/health?probe=1'
curl -i --data-binary 'hello' http://127.0.0.1:7878/echo
curl -i http://127.0.0.1:7878/missing
curl -i -X PUT http://127.0.0.1:7878/health
```

| Route | Behavior |
| --- | --- |
| `GET /` | Embedded HTML page |
| `GET /health` | `ok\n` |
| `POST /echo` | Exact fixed-length request body |
| Unknown path | 404 |
| Supported method on a known path that disallows it | 405 with `Allow` |
| Syntactically valid unsupported method, such as PATCH | 501 |

All responses include Content-Length and Connection: close. Routing uses the raw
path before `?`; it does not percent-decode it. Requests retain the full target.

## Supported subset and bounds

- HTTP/1.1, origin-form targets, and GET/POST/PUT only.
- Strict CRLF, token header names, validated values, and one nonempty Host authority.
  Host supports names and IPv6 literals; IPvFuture and zone identifiers are unsupported.
- Content-Length is checked decimal arithmetic; duplicate fields (even identical)
  and comma-separated lengths are rejected. All Transfer-Encoding is rejected;
  a request with both framing fields is invalid.
- No framing means no body. A partial body is incomplete. Bytes after the first
  request remain outside `consumed`; the server closes without processing them.
- Maximum 8192 bytes through the end of the head, 64 header fields, and 65536 total
  request bytes including the body. Defaults live together in `request.rs`.
- One initialized 64 KiB receive buffer per worker; reused between connections.
- Five seconds to receive a request, then a separate five-second write deadline.
  Trickling bytes does not reset either deadline. EOF/timeouts close the connection.
- Worker count defaults to logical CPU count; queue capacity is twice that count.
  The library accepts independent counts. Full/closed admission drops the socket.

There is no keep-alive, pipelined request processing, chunked decoding, TLS, async
I/O, compression, or `100 Continue` handshake. Do not send `Expect: 100-continue`:
Silk waits for the body and eventually closes if it does not arrive. Blocking workers
limit concurrency; deadlines are subject to OS scheduling, not real-time guarantees.
Responses and serialization allocate; the whole server is not allocation-free.

## Structure

```text
src/main.rs                    listener, address, pool assembly
src/connection.rs              incremental reads, deadlines, response writing
src/connection/thread_pool.rs  bounded queue, worker ownership, shutdown
src/http/request.rs            borrowed parser, limits, incremental offsets
src/http/headers.rs            request-field iterator and response headers
src/http/response.rs           existing owned ResponseBuilder and serialization
src/http/code.rs               status serialization
src/router.rs                  exact route match
src/*tests.rs                  focused parser/socket/lifecycle regressions
benches/                       isolated timing and allocation harness
scripts/check_demo.py          start/curl/stop verification on an ephemeral port
```

Flow: listener → bounded channel → worker-owned buffer → parser → route → response
write → socket drop. See [DESIGN.md](DESIGN.md) for ownership and interview notes.
`memchr` supplies byte searches, Crossbeam supplies the bounded channel, `bytes`
supplies serialization storage, `itoa` formats lengths, and `num_cpus` selects the
default worker count. Silk implements the parsing, worker lifecycle, and routing;
it does not contain handwritten SIMD intrinsics or a custom channel implementation.

## Checks and measurements

```sh
cargo +stable check --all-targets
cargo +stable test
python3 scripts/check_demo.py
cargo +stable bench --bench parser
```

Socket/lifecycle tests use ephemeral loopback ports and child-process watchdogs.
The demo script requires Python 3 and curl, starts its own server, and stops it.
The measurement harness is single-threaded, validates fixtures, uses warmup and
seven samples, and checks its allocation counter with intentional allocations.

On the recorded Apple M2 run, the 35-byte request parsed in a median **137.15 ns**
(7.29 million requests/s in an isolated parser loop). Measured success, incomplete,
and invalid cases made zero allocator calls. These are parser-only observations,
not network throughput or concurrency results. The older ≈6 GB/s parser claim is
unsupported. See [BENCHMARKS.md](BENCHMARKS.md) for exact source, commands, ranges,
head-only byte accounting, and separate delimiter-search results.
