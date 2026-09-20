# Week C — Owned workers and bounded admission

`ThreadPool::new(threads, queue_capacity, handler)` rejects zero workers and keeps
all JoinHandles. Queue capacity is independent of worker count. `submit_task`
returns `Result<(), SubmitError>` with Full/Closed; rejected streams drop immediately
without socket writes. `shutdown(&mut self) -> Result<(), &'static str>` reports
worker panic, joins every handle, and is idempotent. Drop calls the same logic and
never propagates a join error.

The pool owns the sole sender, handles, and a shared atomic stop flag. Workers own
receiver clones and one ConnectionContext each. Shutdown sets the flag before
dropping the sender. A worker past its stop check is active and finishes its handler;
otherwise it drops the received socket and exits. Once receivers disappear, any
remaining queued sockets drop. With one active client and one queued client, a
third submission returns Full. Shutdown lets the active client's deadline expire,
never starts the queued handler, and joins the worker.

Validation: rustfmt; `cargo check --all-targets`; authorized `cargo test -q`:
17 passed, seven ignored legacy timing checks. Watchdog-bounded tests demonstrate
saturation/rejected socket closure, idle shutdown, direct sender disconnection,
active and queued shutdown, Drop, repeated shutdown, and joining after a deliberate
worker panic. Nine pre-existing router/nightly warnings remain. Expected panic and
timeout diagnostics in the child tests are intentional, not suite failures.

Tradeoff: shutdown waits for active work rather than interrupting it; the built-in
handler bounds read and write phases at five seconds each, subject to OS scheduling.
Custom handlers supplied to the pool must impose their own bounds. The sample main
now supplies queue capacity and handles rejected admissions; a usable signal-driven
shutdown is not implemented or claimed. Task D replaces routing scaffolding and
finishes the executable interface.
