# Week D — Exact routes and a stable demo

`router::route(&Request) -> Result<Response, ResponseError>` replaces all unfinished
router abstractions with an exact match. GET / serves the embedded page, GET /health
returns `ok\n`, and POST /echo returns the framed body. The path before '?' is used
for lookup. Known paths with other supported methods return 405 and Allow; unknown
paths return 404. Unsupported method tokens remain parser-level 501 responses.

The connection handler borrows its buffer to route only after the request is
complete, then writes an owned response. Echo currently copies its body into the
existing ResponseBuilder; no claim of allocation-free server handling is made.
Content-Length comes from the selected body and all demo responses explicitly close.

Main accepts an optional bind address, including 127.0.0.1:0, and prints the actual
address. Accept errors lead to pool shutdown; Interrupted retries. The unused
nightly gates and abandoned router code are removed. No signal handler was added.

Validation: rustfmt; `cargo +stable check --all-targets`; `cargo +stable build --release`;
`cargo +stable test -q`: 18 passed and seven ignored obsolete timing checks, no warnings.
Toolchain: rustc 1.94.1 (e408947bf 2026-03-25), aarch64-apple-darwin.
`python3 scripts/check_demo.py` ran the documented cargo command and curl requests
on an ephemeral port: /, /health?probe=1, /echo, /missing, PUT /health, and PATCH /
returned expected statuses, bodies, lengths, and close headers. It terminated and
reaped only its own server process group. TCP endpoint tests additionally read to
EOF to verify closure. Ctrl-C/SIGTERM are process termination, not demonstrated
graceful pool shutdown.

Tradeoff: a small match is enough for three routes; adding dynamic registration or
a tree would obscure the current demo. Task E must measure this final parser and
replace the old timing fixtures. Borrowed responses remain optional and deferred.
