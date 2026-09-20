# Parser measurements

Recorded September 20, 2026 on Apple M2, macOS 26.5.1 (25F80), aarch64-apple-darwin.
Compiler: rustc 1.94.1 (e408947bf 2026-03-25), LLVM 21.1.8.
Source: `4233c3764f882fe55168139c8228658a9190a19c`; working tree **clean** when measured.
The later results/documentation commit does not change the measured source.
[Raw output](docs/benchmarks/2026-09-20-apple-m2.txt) records the full metadata.

```sh
cargo +stable bench --bench parser
```

Cargo's optimized bench profile uses its default optimization level 3, with no
repository profile overrides or explicit target-cpu flags. RUSTFLAGS was empty.
Dependency versions are fixed in Cargo.lock. To reproduce this exact source,
check out the commit above in a separate clone/worktree and run the command with
Rust 1.94.1 installed as stable. On another stable version, record that version.

## Method

Each fixture is constructed and validated before timing. Every operation gets 50,000
warmup iterations, then seven samples of 200,000 iterations. Results pass through
`black_box`; full-parser timing requires Complete and consumes Request plus consumed.
Median and min–max range are reported. There is no target throughput or speedup gate.
This is a single-threaded, warm-cache microbenchmark without CPU affinity or frequency
control; it is not a statistical confidence interval, network benchmark, or load test.

Full parsing calls the library's actual Request::parse. The separate head-validation
measurement includes the same source and calls its private helper with a precomputed
head boundary. This keeps benchmark-only API out of the library. Different compilation
context/inlining can affect those head-only timings; do not subtract them to infer an
exact delimiter cost. The harness includes a narrow dead-code allowance because
harness=false also sees the source's unused test helpers.

The scalar and memchr tests each find the first CRLFCRLF in identical head slices,
with equal results checked before timing. They include search setup per operation;
there is no reusable prebuilt finder. The scalar Rust loop may itself be optimized
by the compiler; “scalar” describes source code, not an assembly guarantee. This is
not a comparison against another complete HTTP parser.

Throughput counts the logical head bytes through CRLFCRLF **once**, regardless of
internal passes. It excludes untouched body bytes and is not measured memory traffic.
GB/s means 10^9 bytes/s, not GiB/s. Requests/s = 10^9 / ns/request. For delimiter tests,
operations/s means searches/s.

## Full parser

| Fixture | Head / body bytes | Median ns/request | Range ns | Requests/s | Logical GB/s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Short GET | 35 / 0 | 137.15 | 136.66–143.22 | 7,291,375 | 0.2552 |
| Typical GET, five fields | 145 / 0 | 378.03 | 372.33–384.06 | 2,645,280 | 0.3836 |
| GET with Host + 40 custom fields | 1177 / 0 | 2616.74 | 2613.37–2623.12 | 382,155 | 0.4498 |
| POST with fixed-length body | 62 / 8192 | 198.21 | 197.51–198.57 | 5,045,218 | 0.3128 |

## Head validation and delimiter search

Values are median ns/operation, followed by min–max in parentheses.

| Fixture | Head validation only | Scalar delimiter | memchr delimiter |
| --- | ---: | ---: | ---: |
| Short | 95.42 (94.89–95.70) | 10.44 (10.39–18.47) | 25.50 (24.48–27.53) |
| Typical | 357.38 (356.15–357.69) | 56.09 (55.95–56.54) | 19.65 (19.48–20.01) |
| Many headers | 2592.91 (2545.37–2755.80) | 384.73 (375.21–388.68) | 77.78 (77.45–77.94) |
| Fixed-body head | 180.28 (179.88–180.49) | 18.75 (18.58–18.88) | 60.56 (60.33–60.91) |

memchr search is faster on these larger fixtures and slower on the two smaller
heads. The faster delimiter-only figures cannot be attributed to the full parser.
The prior ≈6 GB/s parser claim is not supported by these measurements.

## Allocation counts

The harness installs a GlobalAlloc wrapper around System, forwarding pointer/layout
contracts unchanged and using atomics that do not allocate. It counts alloc,
alloc_zeroed, and realloc calls separately. Unsafe code exists only in the isolated
harness, including the intentional allocator sanity check; the parser has no unsafe code.

Fixture creation, metadata, reporting, and 50,000 initialization/warmup parses are
outside the enabled counter. Then 200,000 parses per case run on the main thread,
with only parser execution and black_box consumption inside the measured interval.
These are warmed parser counts, not cold-start or whole-server counts.

| Case | alloc | alloc_zeroed | realloc |
| --- | ---: | ---: | ---: |
| Intentional allocator sanity check | 1 | 1 | 1 |
| Each of the four complete fixtures | 0 | 0 | 0 |
| Incomplete fixed-length body | 0 | 0 | 0 |
| Incomplete head | 0 | 0 | 0 |
| Invalid duplicate Content-Length | 0 | 0 | 0 |

These observations support an allocation-free parsing claim for the measured cases.
They do not prove every possible input, eliminate response allocations, measure
allocator contention under concurrency, or establish deterministic latency.
