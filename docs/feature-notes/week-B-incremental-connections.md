# Week B — Incremental connections

The handler now takes `(&mut TcpStream, &mut ConnectionContext)` and returns
`Result<(), ConnectionError>`. Each worker constructs one context with 64 KiB
of initialized receive storage. The handler resets its occupied length and parser
before reading a new connection, builds the response, and writes it before returning.
All responses include Connection: close. Response building and serialization still allocate.

`RequestParser` is crate-private append-only progress, shared by the public stateless
parser and connections. It keeps a three-byte delimiter-search overlap; after head
validation it retains method, target/header offsets, and the expected message length.
For a head declaring three bytes followed by `ab`, it waits; appending `c` produces
a Request borrowing those buffer bytes without rescanning the head. The context
cannot be reused until the handler releases that borrow. Limits remain constants
with a private numeric-argument helper for focused tests, as requested by the owner.

Read and write phases each get one absolute five-second deadline. Remaining time is
computed before every blocking call and elapsed time checked after successful I/O.
Interrupted operations retry, short writes advance, and WriteZero/errors terminate.
EOF distinguishes an empty connection from a truncated request. The pool closes
streams by dropping them after the handler returns.

Validation: rustfmt on changed Rust files; `cargo check --all-targets`; `cargo test -q`
with loopback permission: 13 passed, seven obsolete timing tests ignored. Tests cover
byte-at-a-time parser progress, representative staged TCP delivery with no early 200,
EOF, oversized declared body, buffer reuse, idle/trickling reads, and blocked writes.
Lifecycle tests run in child processes with a 15-second kill-and-reap watchdog.
The initial sandbox-only socket run failed with PermissionDenied; the authorized
run passed. Eleven pre-existing scaffold/nightly warnings remain.

Tradeoff: fixed blocking workers are easy to trace but each slow connection occupies
a worker until its deadline. OS scheduling can delay wakeups; these are I/O deadlines,
not real-time latency guarantees. No deterministic latency or throughput is claimed.
Task C must retain worker handles, remove the disconnected-channel spin, and make
admission and shutdown explicit. The production pool still has those old lifecycle
limitations at this stage. Interrupted I/O and WriteZero handling were inspected;
they are not artificially injected by the socket regressions.
