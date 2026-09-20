# Silk: application-week plan

Prepared September 17, 2026 from the current working tree, including existing
uncommitted connection changes. This is a proposed, reduced execution plan;
it does not authorize implementing every task automatically. Preserve ROADMAP.md
as the longer backlog. When the owner selects a task here, use its scope and
dependencies rather than silently pulling in all units from the older roadmap.

For tasks selected here, this document controls scope, verification, ordering,
and completion criteria. ROADMAP.md supplies coding style and design context;
its no-test rules and requirement to finish keep-alive before benchmarking do
not apply. Selecting a task authorizes its implementation and the checks listed
here, including local socket demonstrations and measurement execution for E.
Do not ask for another task-level confirmation; tool permission prompts still
apply. Merely reading this plan does not select a task.

## Execution status (September 20, 2026)

The owner explicitly authorized completing B–E in stages, committing after each
stage and pushing to Raforawesome/silk on master. That instruction supersedes the
per-task selection/review gates and no-commit prompt below for this execution.
A uses three constants and a private numeric-argument helper instead of a public
ParseLimits configuration type, per the owner's September 19 preference.
A–D are implemented and verified; E evidence and final handoff are in progress.
Feature notes record historical checks at each stage, not the final API contract.

## Target

A small, explainable Rust HTTP/1.1-subset demo with borrowed request fields,
correct fixed-length framing, bounded blocking workers, and honest measurements.
The owner confirmed 15–20 focused hours and focused regression checks for this
plan. Include implementation, verification, and owner review in that budget.
This is an effort cap, not a promise that every item will fit. Reserve the final
four hours for evidence, documentation, and owner rehearsal. If correctness work overruns, cut optional
features and the numeric resume claim. Do not rush protocol code to hit a date.

One request per connection is enough for this week. Send Connection: close on
every response. Defer keep-alive, a tree router, custom SIMD, async I/O, TLS,
chunked decoding, a generic framework, and full-server allocation elimination.
Document unsupported HTTP behavior explicitly; do not claim full compliance.

Required result: correct borrowed parsing, a reliable one-request demo, explicit
worker ownership, focused regression checks, and reproducible parser evidence.
Optional result: borrowed responses and reusable output from ROADMAP Unit 06,
only after the required result is complete and time remains. Do not take time
from evidence or review to add it. Agent speed does not remove owner review time.

## Audit baseline

- cargo check --all-targets succeeded on rustc 1.96.0-nightly
  (e0e95a718 2026-04-04), with 11 library warnings, duplicated for the test target.
  No tests, benchmarks, allocation measurements, or live demos were run.
- Request borrows path, raw headers, and body. No parser-owned heap allocation
  is visible in the current implementation. This is source inspection, not an
  allocation-counter result or end-to-end zero-copy claim.
- Byte searching uses memchr; there are no custom SIMD intrinsics in src/.
- Headers are not validated and Content-Length is not used for framing.
- Connection handling reads once into an 8192-byte buffer with no deadline.
- Response construction copies the static body, allocates header storage, and
  serialization allocates a BytesMut. The whole request lifecycle allocates.
- Workers use a bounded Crossbeam channel, but handles are discarded and the
  outer receive loop spins/logs forever after sender disconnection.
- Router APIs contain todo!(); the demo does not use them.
- src/tests.rs contains timing functions that discard errors, not correctness
  assertions. Its minimal GET fixture reaches BadRequest because splitting at
  CRLFCRLF leaves no newline for parse_head. That fixture also lacks the Host
  field required for an HTTP/1.1 request. Large-body timing counts bytes the
  parser only borrows. Reported MB/s actually uses a MiB denominator.

## Tasks, in order

### A — Parser correctness and explicit framing (days 1–2; 5–6 hours)

Files: src/http/request.rs; src/http/headers.rs and src/http.rs only as needed;
minimal connection and timing call-site updates to keep the tree compilable.
Use ROADMAP units 03–04 as design context, not authorization for later units.

- Distinguish Complete { request, consumed }, Incomplete, and Invalid.
- Retain borrowed fields; validate request line, origin-form target, CRLF,
  header names/values, and HTTP/1.1 Host requirements.
- Parse Content-Length case-insensitively with checked arithmetic. Reject
  duplicates under a documented strict policy, conflicting framing, and
  unsupported Transfer-Encoding; connection handling must close on rejection.
- No framing means a zero-length request body. Borrow exactly the declared
  body; incomplete bodies must not produce success. Preserve trailing bytes
  through consumed even though this week's server closes after one response.
- Set explicit header, field-count, and total-request limits. Separate syntax
  parsing of an empty header block from Host validation.
- Use initial defaults of 8 KiB request head (through CRLFCRLF), 64 header
  fields, and 64 KiB total request (head plus declared body). Keep defaults in
  one place and allow smaller limits in checks. Use HTTP/1.1, origin-form
  targets, and the existing GET/POST/PUT method subset. Distinguish unsupported
  valid methods from malformed method tokens. Preserve the full target; route
  on the path before '?' later. Do not broaden protocol support during this task.
- Keep parsing independent of status-code serialization: use small parse
  outcomes/errors and translate errors at the connection boundary. Preserve
  useful accessors. Record the exact public API for B in the handoff note.
- A must compile on its own. Until B adds incremental reads, incomplete input
  must close or receive an explicit rejection; it must never be treated as a
  successful request. Mark this temporary behavior in the handoff.

Done: focused parser checks pass for complete input, partial headers/body,
malformed input, an oversized length, and two concatenated messages. Check that
consumed ends at the first request and borrowed slices point into the original
input. The owner can trace these cases through the result API.

### B — Reliable one-request connections (day 3; 3–4 hours)

Depends on A. Files: connection.rs, thread_pool.rs, and required call sites.

- Read incrementally until complete, invalid, EOF, deadline, or size limit.
- Keep initialized, bounded reusable storage per worker; use offsets or a
  small progress state when later reads would invalidate borrowed references.
- Enforce an absolute receive deadline, including clients trickling bytes;
  enforce an absolute write deadline too and handle interrupted reads. Compute
  remaining time before each I/O attempt; a per-write timeout restarted by
  write_all is not an overall write deadline. Handle short writes and WriteZero.
- Send Connection: close consistently. Respond only after the supported request
  is complete. Keep existing explicit socket/response error handling.
- Have each worker own a concrete ConnectionContext and pass it to the handler.
  Move response writing into that handler and return a connection result, so
  later borrowed responses need not escape buffer ownership. Response storage
  may still allocate this week. Reset occupied length and progress per connection.
- Preserve delimiter search progress with overlap rather than rescanning the
  whole accumulated input on each byte. Once the head is validated, retain
  offsets and expected length while awaiting the body. Avoid a second parser.

Done: a split request reaches the same response as a contiguous one; a partial
body does not get an early 200; idle and oversized inputs release the worker.
Verify these with bounded regression checks; compilation alone is insufficient.

### C — Worker ownership and bounded admission (day 4; 2–3 hours)

Depends on B. Files: thread_pool.rs and main.rs.

- Retain JoinHandles; exit when the channel disconnects; remove the outer
  infinite loop and unused stored handler. Reject zero workers.
- Separate queue capacity from worker count and return Full/Closed on try_send.
  Close rejected sockets without blocking the accept loop on response writes.
- Provide an explicit shutdown path that closes admission, discards queued work
  under a documented policy, and joins active workers after their deadlines.
- Set a stop flag before disconnecting the sender; workers check it before
  starting queued work. Define work already past that check as active and bounded
  by B's deadlines. Merely dropping the sender would otherwise drain the queue.
  Make shutdown idempotent and Drop reuse the ownership logic without panicking.
- Do not claim graceful Ctrl-C unless it is wired and demonstrated. A process
  signal handler is optional; explicit pool shutdown is the required scope.

Done: bounded checks demonstrate queue saturation, sender disconnection, and
shutdown with idle, active, and queued connections. The owner explains sender
and handle ownership and why shutdown finishes. No scheduling-policy claim is added.

### D — Finish the demo surface (day 5; 1–2 hours)

Depends on B–C. Files: router.rs, connection.rs, minimal entry-point changes.

- Replace unfinished routing scaffolding with simple exact dispatch. A small
  match is sufficient; a reusable router framework is not required.
- Provide GET / and GET /health, plus unknown-path 404 and a clear policy for
  unsupported methods. Add POST /echo only if it fits after framing is verified.
- Return 405 with Allow for a supported method on a known path that disallows
  it; keep unsupported methods distinct from this routing case. Preserve correct
  Content-Length and Connection: close on successful and error responses.
- Remove unused nightly feature gates and check an available stable toolchain;
  otherwise document the exact toolchain checked. Remove warnings arising from
  abandoned scaffolding rather than globally suppressing warnings.

Done: execute the documented start command and curl examples locally, then stop
the server started for verification. Check endpoint bodies, status, length, and
connection closure. Do not disturb an existing server on the default port; use
an available loopback port for verification instead.
Borrowed responses and reusable output buffers are optional follow-up work,
not prerequisites for a parser-only allocation claim.

### E — Evidence and interview handoff (days 6–7; 3–5 hours)

Depends on A; measure the final parser after any later parser edits. This does
not depend on keep-alive or borrowed responses. Files: a small dedicated timing
harness, BENCHMARKS.md, README.md, and a concise design note if needed. Selecting
E includes implementing AND running the timing and allocation measurements.

- Validate every fixture before timing, consume successful results, use release
  builds and repeated samples, and report ns/request and requests/second.
- Separate head parsing from delimiter search. Define bytes counted; do not
  count a large untouched body as scanned bytes. Distinguish GB/s from GiB/s.
- Include a scalar-versus-memchr delimiter-search comparison performing exactly
  the same operation on identical inputs. Verify equal results before timing.
  Label it separately from parser results; do not attribute the entire parser's
  performance to SIMD or imply comparison against a full HTTP parser.
- Cover at least a short valid request, a typical head, and a many-header head.
  Use warmup and at least five measured samples; report median and range, fixture
  sizes, and iteration count. Include a fixed-length-body case but exclude
  untouched body bytes from scanned-byte throughput. Keep fixture construction
  outside timing. No minimum throughput or speedup is a completion requirement.
- Record CPU, OS, compiler, flags, fixture sizes, exact command, commit and
  dirty state. Keep parser throughput separate from network throughput.
- Measure allocations in an isolated single-threaded harness; exclude fixture
  construction and reporting, and specify initialization boundaries. Count
  allocation, zeroed-allocation, and reallocation calls, and sanity-check that
  an intentional allocation is detected. Measure successful, incomplete, and
  invalid parses separately. Restrict any allocator instrumentation to the
  harness, use a small reviewed implementation, and document unsafe forwarding
  if needed. This is not permission to introduce custom unsafe parser code.
- Move old timing-only tests out of normal regression runs or explicitly ignore
  them with a benchmark-only label. Remove obsolete numbers/fixtures rather
  than leaving a second misleading measurement path. Re-measure after any
  subsequent parser change; identify the exact source state measured.
- README: build/run, endpoints, supported subset, limits, dependencies,
  architecture, known limitations, demo commands, and benchmark evidence.
- Rehearse a two-minute explanation and answer: why borrowing, why memchr,
  why blocking workers, what happens to a slow client, where allocations remain,
  how body boundaries work, and what the benchmark actually measures.

Done: another person can reproduce each published number, and the owner can
explain all three resume bullets without relying on generated explanations.
If measurement is unfinished, publish no throughput number.

## Verification scope

The owner explicitly opted into focused parser and socket regression checks
on September 17, 2026. That instruction supersedes the older ROADMAP's no-test
preference for tasks selected from this plan. Add and run the relevant checks
alongside A–D; do not postpone correctness verification until Task E. Implementing
the tasks themselves still requires selecting them. Measurement execution is
part of selecting E, not a separate approval gate.

Required focused regression cases, distributed across A–D: valid GET with Host; every
split point of a small valid request; empty/invalid target; malformed header;
partial and exact body; duplicate/conflicting length; unsupported transfer
encoding; oversized input; peer EOF; deadline expiry; queue saturation; channel
disconnection and worker join. Use deterministic, bounded cases rather than a
large testing framework. These target actual risks in this implementation.

Put parser cases in A, socket cases in B, queue/shutdown cases in C, and endpoint
cases in D. Every split point belongs in parser checks; socket checks need only
representative fragmentation. Socket writes do not guarantee separate TCP reads,
so test incomplete prefixes directly and use coordination to exercise staged
delivery. Use ephemeral loopback ports, barriers/channels where useful, generous
bounded timeouts, and cleanup of sockets/threads/processes. Avoid fragile sleeps
and exact timing assertions. Put potentially hanging lifecycle checks behind a
bounded subprocess/watchdog so a broken implementation cannot hang the suite.

Each task: format changed files, run cargo check --all-targets and cargo test
for the affected checks. Before final handoff, run the full regression suite.
Report pre-existing warnings separately; do not claim a toolchain was verified
unless it actually ran. Use repository-local artifacts or temporary directories.

## Required agent handoff

Write one short docs/feature-notes/week-[A-E]-name.md note per task with:

1. Implemented behavior and files/public signatures changed.
2. Byte/worker ownership and one concrete input or lifecycle trace.
3. Exact checks/commands and outcomes, including any unverified behavior.
4. One tradeoff, actual limitations, and what the next task needs to know.

Update only affected future contracts in this plan if an API decision changes
them; do not expand scope. Stop at the selected task's boundary. If acceptance
checks fail, fix the task before marking it complete. The owner reviews the diff
and explains the design before selecting the next task.

## Agent task prompt

> Implement only Task [A/B/C/D/E] in docs/ONE_WEEK_PLAN.md. Read the existing
> working tree and preserve all uncommitted changes. Respect the small concrete
> Rust style in ROADMAP.md. Name the files and public API changes before editing.
> Update required call sites but do not implement later tasks or add unrelated
> abstractions, dependencies, custom SIMD, keep-alive, or async I/O. Do not commit
> or delegate automatically. Add and run the focused regression checks relevant
> to this task; the owner has authorized them despite the older roadmap's rule.
> Selecting Task E includes running its benchmarks and allocation measurements.
> Use this plan's precedence rules and required handoff format. Format
> changed files and compile relevant targets. Report exact commands and outcomes,
> remaining unverified cases, ownership of bytes/workers, and one tradeoff the
> owner should explain. Never tune a benchmark to recover a desired resume number.

Use one implementation agent/task at a time because these files and APIs
overlap. Review its diff and explain the behavior before selecting the next
task. An independent read-only review is useful if the owner requests one.

## Resume wording supported today

- Implemented a zero-copy Rust request parser using borrowed byte slices and
  memchr-based delimiter search.
- Kept request parsing free of heap allocation and reused per-thread receive
  buffers across connections.
- Built a fixed-size worker pool with Rust threads and a bounded Crossbeam
  channel for connection dispatch.

These statements describe the current narrow implementation, not complete HTTP
validation. Two consolidated bullets may serve a supporting project better.
Drop handwritten-intrinsics, deterministic-latency, and fine-grained-scheduling
claims. Add a number only after Task E supplies reproducible evidence.

References: [memchr 2.8.0 documentation](https://docs.rs/memchr/2.8.0/memchr/)
and [HTTP/1.1 syntax and framing, RFC 9112](https://www.rfc-editor.org/rfc/rfc9112.html).
