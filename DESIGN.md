# Ownership and design

## One connection through the server

1. Main accepts a TcpStream and calls `ThreadPool::submit_task`. `try_send` transfers
   ownership to the bounded Crossbeam channel or drops the rejected stream.
2. A worker receives it, checks the shared stop flag, and calls the handler with
   its own ConnectionContext. A worker handles one connection at a time.
3. The context resets its occupied length and RequestParser, retaining the same
   initialized receive allocation. Reads append into its unused portion.
4. The parser searches new head bytes with three bytes of overlap, enough for
   CRLFCRLF split across reads. CRLF validation also preserves a pending CR.
5. After finding the terminator, it validates the head once and retains offsets,
   the method, and expected total length. More body bytes do not trigger another
   head parse. The internal parser requires the same append-only input buffer.
6. Completion constructs a Request whose target, raw headers, and body borrow the
   buffer. A declared length of 3 followed by `abcNEXT` borrows only `abc` and reports
   `consumed` at `N`. With `ab` it is incomplete; absent framing means an empty body.
7. Routing borrows that Request. The current builder copies the selected response
   body and allocates headers. Serialization produces a BytesMut and the handler
   writes it using the remaining absolute write budget.
8. Returning releases request borrows; the worker drops the socket. Its context
   can now be reused for the next connection.

The public entry point is `Request::parse(&[u8]) -> Result<ParseStatus, ParseError>`.
ParseStatus is Incomplete or Complete { request, consumed }. The parser never emits
HTTP status bytes. The connection boundary maps errors to 400/413/431/501/505.
A private helper accepts numeric limits for small boundary tests; production uses
three constants. There is no public limits configuration object.

## Worker shutdown

The pool owns the sole Sender and all JoinHandles. Workers own Receiver clones and
share only an atomic stop flag. `shutdown` sets the flag, takes/drops the sender,
then joins all workers. A connection already past the stop check is active; its
handler completes under its I/O deadlines. Workers do not start remaining queued
connections. Receiver destruction drops queued sockets. Disconnected idle receivers
exit rather than spin. Every handle is drained once; repeated shutdown succeeds.
Drop uses the same logic and ignores join errors, while explicit shutdown reports
whether any worker panicked. No workers are automatically respawned.

This is a bounded FIFO dispatch arrangement, not a fine-grained scheduling policy.
Custom pool handlers must bound their own work. The built-in handler bounds receive
and write phases separately; OS scheduling and blocking diagnostics can add delay.
Explicit shutdown is tested; process signals are not wired to it.

## Allocation boundaries

| Operation | Allocation behavior |
| --- | --- |
| Startup | Channel, thread handles, runtime/OS thread resources |
| Worker startup | One initialized 64 KiB receive Vec |
| Parse and header lookup | Borrowed slices and integer offsets; measured parser fixtures allocate zero times |
| Exact route lookup | Match on borrowed bytes |
| Response construction | Copies body; allocates header vector and custom header strings |
| Serialization | Allocates BytesMut through ToBytes |

“Zero-copy parser” describes request fields borrowing the receive buffer. It does
not remove the socket read into that buffer, make echoed responses zero-copy, or
prove deterministic latency. Allocation instrumentation is confined to the bench
executable and is absent from the library/server binary.

## Two-minute walkthrough to practice

“I built a small Rust server around a fixed worker pool. The accept loop transfers
sockets through a bounded channel; when it fills, new sockets close immediately.
Each worker reuses its own receive buffer. The parser distinguishes incomplete,
invalid, and complete requests, and returns borrowed fields with an exact consumed
length. It validates the head once and retains offsets while waiting for a fixed-
length body. This makes fragmented input correct without copying request fields.

“Each receive and write phase has an absolute deadline, so sending one byte at a
time does not keep resetting a timeout. Shutdown closes admission, discards queued
work, and joins active workers. The demo deliberately closes after one request.
I measured the parser separately from sockets and counted allocations in an isolated
harness. Response construction still allocates. I use memchr and Crossbeam rather
than claiming I wrote their SIMD or channel machinery.”

Practice these follow-ups without reading the answer:

- **Why borrowing?** Request data already lives in the receive buffer; lifetimes
  prevent using slices after that storage is reused.
- **Why memchr?** It provides maintained byte-search routines. Its delimiter search
  wins on the larger measured heads but loses on some small ones. It is only one
  part of validation, not a full-parser speedup claim.
- **Why blocking workers?** Simple ownership and bounded resources; the tradeoff is
  one slow socket occupying a worker. An event loop could support more idle clients.
- **What happens to slow clients?** Remaining receive/write budget decreases after
  every operation. Exhaustion closes the socket; timeouts do not guarantee CPU scheduling.
- **Where do allocations remain?** Worker initialization, response bodies/headers,
  serialization, and library/OS runtime operations.
- **How are bodies bounded?** One strict Content-Length, checked arithmetic, a total
  size limit, and borrowing precisely that many bytes. Unframed bytes are not a body.
- **What does the benchmark measure?** Repeated in-memory parsing of validated warm
  fixtures, with successful results consumed. It excludes networking and counts logical
  head bytes, not untouched body bytes, for GB/s.

Owner rehearsal remains a human step; generated notes cannot establish that someone
can explain the implementation independently.

## Resume wording supported by this implementation

- Built a Rust HTTP/1.1-subset parser with borrowed request fields, strict fixed-length
  framing, and zero measured allocator calls across successful, incomplete, and invalid cases.
- Implemented a fixed worker pool with reusable receive buffers, bounded admission,
  absolute socket deadlines, and explicit worker shutdown and joining.

If a number is useful, qualify it: “137 ns median for a 35-byte request in an isolated
Apple M2 parser benchmark.” Do not claim custom SIMD intrinsics, ≈6 GB/s full-parser
throughput, allocation-free end-to-end handling, or deterministic concurrent latency.
