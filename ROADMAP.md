# Silk: agent execution plan

> Application-week scope: when executing tasks from
> [docs/ONE_WEEK_PLAN.md](docs/ONE_WEEK_PLAN.md), that document takes precedence
> for scope, ordering, verification, and completion criteria. The owner has
> approved focused regression checks for that plan; the historical no-test
> instructions below do not apply to those tasks. Keep this document as design
> context and the longer backlog, not an instruction to complete all units.

## Target and stopping point

Finish a compact Rust HTTP/1.1 server whose implementation you can explain end to end. The core story is borrowed request data, explicit message framing, reusable buffers, and bounded blocking workers. Keep `memchr` and Crossbeam: writing a SIMD library or a channel is outside the core plan.

Implement the feature units below in order, one agent task at a time. Each unit must leave the library and executable coherent and compilable. Do not launch the whole roadmap as a single autonomous implementation job. Estimates are rough effort budgets, not agent deadlines; expect roughly 30–45 focused hours including human review.

The owner considers the current demo working and does not want test work. Do not add or run tests, fuzzers, test frameworks, or load tests under this plan. Compilation and owner-requested manual demonstrations are sufficient checkpoints here. Unit 10 is a separately authorized measurement task, not permission to claim measured performance without running it. Compilation alone does not establish protocol correctness.

No source changes are authorized by the existence of this document. The owner selects a unit to execute.

## Style contract for every agent

Follow the surrounding code, especially `Request`, `ResponseBuilder`, `ToBytes`, and the small formatting helpers.

- Keep the existing module tree. Add a file only when a substantial concept has a clear owner.
- Prefer small concrete structs, enums, plain helper functions, and separate `impl` blocks for separate responsibilities.
- Retain descriptive names, private fields with useful getters, builder methods, borrowed byte slices, and byte-oriented parsing.
- Continue using `BytesMut`, `itoa`, `memchr`, and the existing `Code` serialization where appropriate.
- Use traits only where there is an actual shared behavior. Do not add generic service layers, executor traits, middleware, dependency injection, or an async runtime.
- Preserve the response builder. A narrowly justified lifetime parameter is preferable to a second response framework.
- Do not introduce `Arc<Mutex<_>>` state when ownership or the existing channel suffices.
- Handle client input and I/O failures explicitly. Startup invariants may still use a clear `expect`; do not expand this into a generic error-handling rewrite.
- Keep comments focused on ownership, framing, boundaries, and non-obvious decisions. Avoid narrating obvious code.
- No broad renaming, unrelated cleanup, formatting churn, speculative extension points, or new dependency without explaining why existing facilities are insufficient.
- Preserve existing uncommitted work. Inspect the current tree before editing; never reset it or commit automatically.
- If a required prerequisite is missing, report it rather than implementing several later units silently.

## Responsibility map

| Location | Responsibility |
| --- | --- |
| `src/http/request.rs` | Parse bytes into borrowed request fields; no socket I/O or routing |
| `src/http/headers.rs` | Preserve the response `Header` enum; add clearly named borrowed request-header types if needed |
| `src/http/response.rs` | Response representation and serialization |
| `src/http/code.rs` | Status codes and their wire representation |
| `src/connection.rs` | Socket reads/writes, reusable buffers, request lifetime, connection policy |
| `src/connection/thread_pool.rs` | Workers, task admission, channel ownership, shutdown and joining |
| `src/router.rs` | Small exact method/path dispatch |
| `src/main.rs` | Configuration, listener, server assembly and process lifecycle |

The final data flow should be easy to trace:

```text
listener -> bounded connection queue -> worker
  -> receive into reusable buffer
  -> frame and parse borrowed request
  -> route -> build borrowed/owned response
  -> write using reusable response buffer
  -> release borrows -> reuse or compact request buffer
```

## Shared decisions: keep units compatible

These are intended contracts, not a demand to implement future abstractions early.

1. Keep blocking I/O and a fixed worker pool.
2. Allocate request and response buffer storage once per worker; do not promise every server operation is allocation-free.
3. Support HTTP/1.1, origin-form request targets, current GET/POST/PUT methods, and bounded fixed-length bodies. Return an explicit unsupported-method response for other syntactically valid methods. Do not advertise full HTTP compliance.
4. Reject unsupported transfer encodings and close. Reject ambiguous framing; never reuse a connection after a framing error.
5. Use distinct incomplete and invalid outcomes. A complete result includes the exact number of consumed bytes.
6. Keep request data borrowed. Preserve raw `headers()` access where practical; offer an iterator for individual validated headers.
7. Keep `Header` for response headers; call a borrowed request field `RequestHeader` to avoid an unrelated rename.
8. Separate protocol parsing from connection deadlines, socket errors, and routing.
9. Use exact routes, registered at startup. Strip the query portion for route lookup without allocating; preserve the original target through the existing accessor.
10. Keep one worker responsible for a connection until it closes. Document the concurrency limitation of idle keep-alive connections.

Choose a few explicit defaults during implementation, for example 8 KiB of headers, 64 header fields, and 64 KiB total buffered request bytes. Define what each limit counts. Treat these as demonstration defaults, not performance recommendations.

## Feature-unit index

| Unit | Deliverable | Depends on | Budget |
| --- | --- | --- | --- |
| 01 | Explicit connection failures | Current tree | 1–2 h |
| 02 | Worker lifecycle and joining | 01 | 2–3 h |
| 03 | Borrowed request-header parsing | 01 | 3–4 h |
| 04 | Complete/incomplete framing API | 03 | 3–5 h |
| 05 | Incremental connection receive loop | 02, 04 | 4–5 h |
| 06 | Borrowed responses and reusable output | 05 | 3–4 h |
| 07 | Finished exact-match router | 06 | 2–3 h |
| 08 | Bounded admission and operational shutdown | 02, 07 | 3–4 h |
| 09 | Sequential keep-alive | 05–08 | 3–4 h |
| 10 | Reproducible parser measurements | 09 | 2–4 h |
| 11 | Repository and interview handoff | 01–10, or documented omissions | 2–3 h |

The default execution order is the table order. Dependencies do not authorize concurrent edits; parser, connection, and pool signatures overlap. Use another subagent for read-only review only when explicitly requested by the owner.

## Unit 01 — Explicit connection failures

**Goal:** make the existing one-request demo's failure behavior readable without redesigning it.

**Primary files:** `connection.rs`, `connection/thread_pool.rs`, `http/code.rs` only if necessary.

**Implementation:**

- Replace unchecked UTF-8 conversion with byte-safe handling; remove request debug output from the normal path.
- Replace parsing, writing, and shutdown `unwrap()` calls reachable from client behavior.
- Distinguish malformed requests, read failures, and peer closure with a small connection-local error representation where useful.
- Send a small error response when appropriate, then close; never attempt another response after a failed write.
- Remove per-connection worker logging. Retain useful startup/error output outside timed paths.
- Keep current request/response APIs and one-read behavior for this unit.

**Completion checkpoint:** the successful response remains the same; known client error branches no longer unwind via `unwrap` or rely on invalid UTF-8. Compile affected targets.

**Explain back:** where an input error becomes an HTTP response, where I/O errors simply terminate a connection, and why unchecked UTF-8 was unnecessary.

**Do not include:** framing, routing, buffering changes, test work.

## Unit 02 — Owned worker lifecycle

**Goal:** give the thread pool a finite, understandable lifetime.

**Primary files:** `connection/thread_pool.rs`; minimal call-site changes in `main.rs`.

**Implementation:**

- Store worker `JoinHandle`s in `ThreadPool`.
- Replace the outer infinite receive loop with a loop that exits on channel disconnection.
- Validate nonzero worker count.
- Drop all pool-owned senders before joining workers. An `Option<Sender<TcpStream>>` is a reasonable way to take the sender during shutdown.
- Offer one clear explicit shutdown path; a `Drop` implementation must reuse ownership logic without double-joining or panicking.
- Remove the stored handler if it is used only to start workers.
- Report join failures without inventing worker respawning or panic recovery.

**Completion checkpoint:** source ownership shows that closing admission disconnects receivers and each worker handle is joined once. Compile the server.

**Important boundary:** joins can still wait on blocking connections at this stage. Do not claim bounded shutdown until units 05 and 08.

**Explain back:** why a disconnected receiver remains disconnected and why the old outer loop could spin forever.

## Unit 03 — Borrowed request headers

**Goal:** turn the raw header block into usable validated metadata while retaining the zero-copy representation.

**Primary files:** `http/request.rs`, `http/headers.rs`, `http.rs` as needed.

**Implementation:**

- Fix request-line/header boundary handling, including the no-header case at the syntax layer.
- Require consistent CRLF syntax; reject obsolete folded headers and whitespace before a header colon.
- Add `RequestHeader<'a> { name: &'a [u8], value: &'a [u8] }` and a borrowed iterator or similarly small abstraction.
- Validate names and permitted value bytes. Trim permitted surrounding spaces/tabs using slices.
- Preserve raw header access and provide clearly named iteration/lookup methods.
- Header lookup is ASCII case-insensitive and allocation-free; do not lowercase into a new buffer.
- Collect framing-relevant information during validation rather than making many independent scans.
- Keep syntax validation separate from server policy such as requiring Host.

**Completion checkpoint:** inspect a short example showing raw headers, iteration, and lookup; no owned strings or per-request map is introduced. Compile.

**Explain back:** borrowed header lifetimes, case-insensitive names, and why a linear scan is appropriate for bounded small header sets.

**Do not include:** chunked decoding, header indexing structures, response-header redesign.

## Unit 04 — Exact request framing

**Goal:** distinguish incomplete input, invalid input, and exactly one complete message.

**Primary files:** `http/request.rs`, `http.rs`, `http/code.rs`; minimal compile-preserving connection changes.

**Suggested API shape:**

```rust
pub enum ParseStatus<T> {
    Incomplete,
    Complete { value: T, consumed: usize },
}

// Limits can be supplied to an explicitly named parse_with_limits method.
// Preserve a convenient default parse entry point if that keeps usage simple.
pub fn parse(input: &[u8]) -> Result<ParseStatus<Request<'_>>, ParseError>;
```

**Implementation:**

- Use a small `ParseError` enum for syntax, unsupported behavior, and size limits. Translate it to `Code` at the connection boundary.
- Determine body length with checked decimal parsing and checked length arithmetic.
- Conservatively reject duplicate Content-Length fields, including identical duplicates; document the stricter policy.
- Reject conflicting framing and unsupported Transfer-Encoding, then require connection closure.
- No body framing means zero request-body bytes; extra bytes remain unconsumed.
- Reject empty/invalid targets and unsupported target forms. Enforce the supported version, Host requirements, and explicit header/body limits.
- Return `Incomplete` only when more bytes could produce a supported request within limits.
- Borrow only the framed body, not every remaining input byte.
- Update the existing demo call site in the same unit so it compiles; it may still close on incomplete input until unit 05.
- If existing timing functions require signature changes, make mechanical compatibility edits only; do not run them or add tests.

**Completion checkpoint:** document the exact result for a short request, partial body, concatenated requests, absent body length, and ambiguous framing. Compile.

**Explain back:** `consumed`, why body framing is separate from finding the header terminator, and why malformed framing always closes the connection.

## Unit 05 — Incremental reads with worker-owned buffers

**Goal:** receive one complete bounded request across any number of reads.

**Primary files:** `connection.rs`, `connection/thread_pool.rs`, `main.rs`.

**Implementation:**

- Replace thread-local `RefCell<Vec<u8>>` with an explicit `ConnectionContext` owned by each worker and created once inside its worker closure.
- Context owns initialized request storage, an occupied-length cursor, and receive progress. Keep unused capacity out of the parser input.
- The pool handler becomes a concrete function pointer receiving the stream and context. Move response writing into the connection handler so later borrowed responses need not escape it; the handler returns a connection result rather than `Response`.
- Read into the unused portion of the bounded buffer until the parser reports completion or failure.
- Distinguish EOF before any request from EOF during an incomplete request; handle interrupted reads appropriately.
- Use a request deadline based on `Instant`; set each blocking read timeout from the remaining time. A fresh full timeout on every byte is insufficient.
- Set a bounded write timeout. Reject a request that cannot fit; do not grow storage silently.
- Preserve delimiter-search progress with overlap. Once the head is parsed, retain offsets/body length rather than references that prevent more reads. Keep this state small and concrete.
- Reset context between connections. Still close after one response in this unit and explicitly send `Connection: close`.

**Completion checkpoint:** explain the receive cursor and state transitions with a request arriving in three fragments. Compile. A manual socket demonstration is optional only if requested.

**Explain back:** why worker ownership removes the need for `RefCell`, and why offsets survive buffer writes while borrowed slices cannot.

## Unit 06 — Borrowed response bodies and reusable output

**Goal:** avoid allocating/copying the demo body and allocating a fresh serialization buffer per response.

**Primary files:** `http/response.rs`, `connection.rs`, `http/headers.rs` only as needed.

**Implementation:**

- Preserve `ResponseBuilder` and its chained methods.
- Use `Cow<'a, [u8]>` for body storage, or one small borrowed/owned body enum if it fits the implementation more clearly. Choose one, not both.
- Let `with_content` accept static content, borrowed request bodies, and owned bytes without forcing `to_vec()`.
- Add reusable output storage to `ConnectionContext`; clear it between responses without releasing capacity.
- Add a header-only serialization method so the connection layer can `write_all` the head and then the body. Preserve `ToBytes` for callers that intentionally want a contiguous response.
- Prevent contradictory Content-Length output by having one explicit owner of that header.
- Keep ordinary custom response-header allocation supported and documented. Do not broaden the claim to all handlers being allocation-free.
- Explain that the response header vector may still allocate. Eliminate that only if it is small, directly useful work; otherwise retain it and scope the claim to parsing and buffer reuse.

**Completion checkpoint:** the embedded page body is borrowed, serialization storage is reused, and a borrowed body is written before the request buffer can be reused. Compile.

**Explain back:** the difference between borrowed body storage, reusable output storage, and a fully allocation-free request lifecycle.

## Unit 07 — Finish a small router

**Goal:** give the demo a complete, readable application layer.

**Primary files:** `router.rs`, `main.rs`, `connection.rs`; small method getter/equality changes if needed.

**Implementation:**

- Finish `HashRouter` as one concrete exact-match router. Remove the unusable trait signatures and unfinished tree router rather than maintaining two designs.
- Use method plus path as the key. Perform registration allocations at startup, never for lookup.
- Keep handlers as plain functions unless an actual demo requirement needs captured state. A handler can borrow `Request` and return `Response` with a body tied to the request buffer; write down that lifetime contract before implementing it.
- If existing boxed closures can express the contract simply, retaining them is acceptable. Do not add generic handler traits to solve hypothetical future needs.
- Pass the initialized router as immutable shared server state. `Arc` is justified here if workers need owned handles; a mutex is not.
- Register `GET /`, `GET /health`, and `POST /echo`.
- Match the path portion before `?`; preserve the complete target in `Request`.
- Return 404 for missing paths and 405 with appropriate Allow information for a supported method used on a known path that does not accept it.

**Completion checkpoint:** trace one endpoint from registration through lookup to response writing; no public `todo!()` router remains. Compile.

**Explain back:** why exact routing is enough here and how an echo body can remain borrowed until the write completes.

## Unit 08 — Admission, configuration, and process shutdown

**Goal:** make resource limits and termination explicit from the entry point.

**Primary files:** `connection/thread_pool.rs`, `connection.rs`, `main.rs`, `Cargo.toml` only if justified.

**Implementation:**

- Add small configuration structs for workers/queue capacity and connection limits/timeouts. Keep configuration ownership close to the module using it.
- Separate queue capacity from worker count.
- Have submission return a small Full/Closed result using `try_send`. On Full, close the newly accepted stream without a blocking error-response write in the accept loop.
- Add only useful shared atomic counters, such as active connections and rejected admissions; do not introduce a metrics dependency or per-request logging.
- Add a stop signal shared by the accept loop and workers. A small established signal-handling dependency is acceptable only with an explanation; avoid handwritten unsafe signal handling.
- Ensure the listener can observe stop even when no clients connect, for example a nonblocking accept loop with a short bounded idle sleep.
- On shutdown: stop admitting, signal stop, close the sender, discard queued work according to a documented policy, finish or deadline-close active work, and join workers.
- Prevent workers from draining every queued slow connection during shutdown. Check stop before starting the next queued task.
- Active reads/writes remain bounded by unit 05 deadlines; don't claim immediate cancellation of blocking syscalls.

**Completion checkpoint:** describe behavior for Full, Closed, normal completion, and Ctrl-C with idle/active/queued connections. Compile.

**Explain back:** queue memory bounds, overload rejection versus blocking submission, and the difference between shutdown signalling and actually joining threads.

## Unit 09 — Sequential persistent connections

**Goal:** reuse an established connection without losing message boundaries or buffer ownership clarity.

**Primary files:** `connection.rs`, request connection-header helpers, response connection headers.

**Implementation:**

- Loop over requests on one connection, one response at a time.
- Respect Connection header tokens case-insensitively; close if any applicable token requests close.
- Set an idle keep-alive timeout and a maximum requests-per-connection count.
- Release request/response borrows before compacting consumed input. Retain already-read bytes for the next request.
- Process multiple buffered requests in order; do not spawn per-request tasks or write responses out of order.
- Handle a client half-close after a complete request without discarding that request before its response is written.
- End persistence on shutdown, framing error, unsupported framing, deadline, or write failure.
- State when the next request's receive deadline begins, including bytes already received with the previous request.

**Completion checkpoint:** walk through two concatenated requests and show exactly when bytes are consumed, borrows end, and storage is compacted. Compile.

**Explain back:** why keep-alive saves connection overhead but idle clients still occupy blocking workers. The queue bounds resource use; it does not remove this scaling limit.

## Unit 10 — Reproducible measurements (explicit opt-in)

**Goal:** replace an ambiguous throughput claim with a precisely defined measurement.

**Primary files:** a small benchmark executable or `benches/` harness, `BENCHMARKS.md`; existing timing-only functions if migrating them.

**Authorization boundary:** selecting this unit authorizes implementing a benchmark harness. Run it only if the owner's task also requests measurement. Do not run tests or install a test suite.

**Implementation:**

- Separate delimiter search from validated request-head parsing. Report ns/request and requests/s; label throughput using the actual bytes processed.
- Do not count an untouched large borrowed body as scanned parser bytes.
- Validate fixtures and consume results outside/inside timing as appropriate. Never silently time a parse error as successful parsing.
- Use several valid request sizes and header counts, warmup, repeated runs, and explicit units. Reuse the owner's small `Instant`/`black_box` style unless a benchmark dependency clearly improves the result.
- Record CPU, OS, compiler, release flags, commit plus dirty state, commands, and measurement limitations.
- Compare a simple scalar delimiter search with `memchr` only for identical work. A comparison with `httparse` is optional and must identify validation differences.
- If allocation measurement is requested, keep it in a dedicated single-threaded measurement binary with setup and reporting outside the measured region. Count parser allocations separately from response/connection operations.
- Clearly mark unmeasured fields as pending. Never invent a result, target 6 GB/s as a pass criterion, or infer deterministic latency.

**Completion checkpoint:** another person can follow the documented command; any recorded number has an actual run and a named measurement boundary.

**Explain back:** what is being timed, what is excluded, and why the old large-body throughput could exaggerate parser work.

## Unit 11 — Repository and interview handoff

**Goal:** make the completed implementation easy to understand without the agent transcript.

**Primary files:** `README.md`, `DESIGN.md`, `BENCHMARKS.md`, and a small `docs/feature-notes/` directory.

**Implementation:**

- README: purpose, build/run commands, endpoints, supported protocol subset, limits, architecture, known limitations.
- DESIGN: follow the actual data flow and link to concrete types/functions. Explain ownership, framing, admission, persistence, and shutdown.
- Include a compact allocation table: startup, per worker, parse, routing, response construction, serialization. Separate inspection-based observations from measurements.
- Audit public `todo!()` entries and stale comments; document or remove abandoned scaffolding within scope.
- Remove unnecessary nightly features if no implemented code needs them. State the toolchain actually checked rather than asserting stable support without a build.
- Summarize benchmark evidence honestly, or explicitly say measurement is pending.
- Suggest resume wording tied to completed capabilities. Credit `memchr` for SIMD and Crossbeam for channel machinery.
- Prepare a two-minute project explanation and answers to: why borrowed slices, why blocking workers, what happens at overload, how framing works, where allocations remain, what the benchmark measures, and what production support would require.

**Completion checkpoint:** a reader can trace the implementation and its limitations from the docs. No feature or performance claim exceeds the implemented/measured state.

## Optional unit S1 — One custom SIMD experiment

Only select this after the core project is finished and only if writing intrinsics is personally valuable. It is not necessary to justify using `memchr`.

Implement one delimiter primitive with a scalar fallback and one backend for hardware you can execute on. Explain feature selection, bounds, vector tails, and matches crossing vector boundaries. Compare with the same scalar operation and `memchr` using unit 10's measurement rules. Do not replace the main parser path unless evidence supports it.

Unsafe SIMD work needs a separate correctness-verification agreement with the owner; the current no-test scope does not authorize introducing an unverified unsafe backend. Leave this unit deferred unless that scope changes.

## Reusable prompt for an implementation subagent

Copy this and fill in the unit number:

> Implement only Unit XX from ROADMAP.md in the current Silk working tree. Read its dependencies and the shared style/responsibility contracts first. Inspect existing uncommitted changes and preserve them. Before editing, briefly name the exact files and any public signature changes this unit needs. Keep each abstraction close to the existing Rust style: small concrete types, borrowed bytes, helpers, builders, and explicit ownership. Update all necessary call sites so this unit stands alone. Do not implement later units, introduce broad refactors, create commits, spawn more agents, or add/run tests. Compile the affected targets with cargo check and format only changed Rust files; if a tool is blocked, report the limitation instead of claiming success. Benchmark execution is allowed only when separately requested. Finish with the handoff format below, including a short explanation a human can repeat in an interview.

If a unit needs a different design to remain simple, explain the smallest deviation and update this document's affected future contracts in the same change. Do not silently leave later agents with stale API assumptions.

## Required handoff after every unit

Write a short `docs/feature-notes/XX-name.md` using this format, and summarize it in the final response:

1. **Behavior:** what changed, with one concrete input or lifecycle example.
2. **Code map:** the few types/functions to read, in order.
3. **Ownership:** who owns the bytes/state, who borrows it, and when reuse is allowed.
4. **Decision:** the main tradeoff and why it fits this project.
5. **Validation:** exact compile/format commands and outcomes; explicitly state that tests were not run. Record any separately requested manual check or benchmark without overstating it.
6. **Boundary:** what this unit intentionally leaves to the next unit and any actual limitation.
7. **Interview explanation:** 3–5 plain-language sentences; include one likely follow-up question and answer.

Keep each note short, around 250–400 words. These are implementation notes, not another documentation framework.

## Human review gate

Before selecting the next unit, read the note and the main changed functions. You should be able to answer:

- What new behavior exists?
- Which type owns the relevant memory or workers?
- What happens on incomplete input, error, or shutdown?
- Why is this abstraction present?

If those answers are unclear, ask the same agent to simplify or explain the current unit. Do not accumulate features you cannot explain. Once units 01–11 are finished, stop and return attention to your flagship project.
