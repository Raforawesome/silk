# Week E — Reproducible evidence and handoff

`cargo +stable bench --bench parser` runs the dedicated single-threaded harness in
benches/. The old timing-only src/tests.rs module is removed. Successful fixtures
are checked first, results consumed, and each operation receives 50,000 warmups and
seven 200,000-iteration samples. Full parsing calls the library; head-only validation
includes the same source to access private helpers without adding public API.
Scalar/memchr searches run identical inputs with their results checked for equality.
Only head bytes count toward throughput, including for the 8192-byte body fixture.

The benchmark binary alone owns the counting allocator. It forwards unchanged
pointer/layout contracts to System; counters allocate nothing. An explicit sanity
check detects alloc, zeroed allocation, and reallocation once each. Fixture setup,
reporting, and warmup are outside the enabled counter. Successful, incomplete-head,
incomplete-body, and duplicate-length-invalid cases all measured zero allocator
calls over 200,000 parses each. Borrowed fields retain caller ownership; no new
server allocation claim follows from this result.

The final run measured clean commit 4233c3764f882fe55168139c8228658a9190a19c on Apple M2,
macOS 26.5.1, stable rustc 1.94.1. Full-parser medians span 137.15–2616.74 ns across
four fixtures, with logical head throughput 0.2552–0.4498 GB/s. BENCHMARKS.md records
all ranges, fixtures, commands, metadata, and limitations; raw output is archived.
No measurements were tuned to recover the old ≈6 GB/s claim.

Validation: `cargo +stable check --all-targets`, `cargo +stable test -q` (18 passed,
none ignored), and the release benchmark passed. Formatting and diff whitespace
checks passed. Task D's actual executable/curl demonstration remains applicable;
no production parser changes followed that stage. Lifecycle tests intentionally
emit deadline errors and one caught worker panic. There are no compiler warnings.

README covers commands, endpoints, dependencies, limits, and the project structure.
DESIGN covers ownership, remaining allocations, tradeoffs, a two-minute walkthrough,
and follow-up interview questions. Owner rehearsal is still a human responsibility;
this implementation does not establish personal interview readiness. Optional
borrowed responses, keep-alive, async I/O, and custom SIMD remain deferred.
