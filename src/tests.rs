use std::hint::black_box;
use std::time::{Duration, Instant};

use crate::http::request::Request;

const ITERATIONS: u32 = 500_000;
const WARMUP_ITERATIONS: u32 = 50_000;

/// Runs a benchmark over `iterations` calls, returning the total elapsed duration.
fn bench<F: Fn()>(warmup: u32, iterations: u32, f: F) -> Duration {
    // Warmup phase: let the CPU caches and branch predictors settle.
    for _ in 0..warmup {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed()
}

fn report(label: &str, elapsed: Duration, iterations: u32, input_size: usize) {
    let total_secs = elapsed.as_secs_f64();
    let ns_per_op = (elapsed.as_nanos() as f64) / iterations as f64;
    let ops_per_sec = iterations as f64 / total_secs;
    let throughput_mb = (input_size as f64 * iterations as f64) / (1024.0 * 1024.0 * total_secs);

    eprintln!("  {label}");
    eprintln!("    {iterations} iterations in {total_secs:.3}s");
    eprintln!("    {ns_per_op:.1} ns/op");
    eprintln!("    {ops_per_sec:.0} ops/sec");
    eprintln!("    {throughput_mb:.1} MB/s");
    eprintln!("    input size: {input_size} bytes");
    eprintln!();
}

#[test]
fn perf_parse_minimal_get() {
    let raw = b"GET / HTTP/1.1\r\n\r\n";

    let elapsed = bench(WARMUP_ITERATIONS, ITERATIONS, || {
        let result = Request::parse(black_box(raw));
        black_box(result).ok();
    });

    report("Minimal GET request", elapsed, ITERATIONS, raw.len());
}

#[test]
fn perf_parse_typical_get() {
    let raw = b"GET /api/v1/users/12345/profile HTTP/1.1\r\n\
Host: www.example.com\r\n\
User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36\r\n\
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
Accept-Language: en-US,en;q=0.5\r\n\
Accept-Encoding: gzip, deflate, br\r\n\
Connection: keep-alive\r\n\
Cache-Control: no-cache\r\n\
\r\n";

    let elapsed = bench(WARMUP_ITERATIONS, ITERATIONS, || {
        let result = Request::parse(black_box(raw));
        black_box(result).ok();
    });

    report("Typical GET with headers", elapsed, ITERATIONS, raw.len());
}

#[test]
fn perf_parse_post_with_body() {
    let body = r#"{"username":"john_doe","email":"john@example.com","password":"s3cret!"}"#;
    let raw_string = format!(
        "POST /api/v1/auth/login HTTP/1.1\r\n\
Host: api.example.com\r\n\
Content-Type: application/json\r\n\
Content-Length: {}\r\n\
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\r\n\
Accept: application/json\r\n\
\r\n\
{body}",
        body.len()
    );
    let raw = raw_string.as_bytes();

    let elapsed = bench(WARMUP_ITERATIONS, ITERATIONS, || {
        let result = Request::parse(black_box(raw));
        black_box(result).ok();
    });

    report("POST with JSON body", elapsed, ITERATIONS, raw.len());
}

#[test]
fn perf_parse_many_headers() {
    let mut request = String::from("GET /resource HTTP/1.1\r\n");
    for i in 0..30 {
        request.push_str(&format!(
            "X-Custom-Header-{i}: value-{i}-some-reasonably-long-header-value\r\n"
        ));
    }
    request.push_str("\r\n");
    let raw = request.as_bytes();

    let elapsed = bench(WARMUP_ITERATIONS, ITERATIONS, || {
        let result = Request::parse(black_box(raw));
        black_box(result).ok();
    });

    report("GET with 30 custom headers", elapsed, ITERATIONS, raw.len());
}

#[test]
fn perf_parse_large_body() {
    let body = "x".repeat(8192);
    let raw_string = format!(
        "PUT /api/v1/upload HTTP/1.1\r\n\
Host: upload.example.com\r\n\
Content-Type: application/octet-stream\r\n\
Content-Length: {}\r\n\
\r\n\
{body}",
        body.len()
    );
    let raw = raw_string.as_bytes();

    let elapsed = bench(WARMUP_ITERATIONS, ITERATIONS, || {
        let result = Request::parse(black_box(raw));
        black_box(result).ok();
    });

    report("PUT with 8KB body", elapsed, ITERATIONS, raw.len());
}

#[test]
fn perf_parse_long_url() {
    let path = format!("/search?{}", "key=value&".repeat(100));
    let raw_string = format!(
        "GET {path} HTTP/1.1\r\n\
Host: www.example.com\r\n\
Accept: */*\r\n\
\r\n"
    );
    let raw = raw_string.as_bytes();

    let elapsed = bench(WARMUP_ITERATIONS, ITERATIONS, || {
        let result = Request::parse(black_box(raw));
        black_box(result).ok();
    });

    report(
        "GET with long URL (~1KB query string)",
        elapsed,
        ITERATIONS,
        raw.len(),
    );
}

#[test]
fn perf_parse_comparison_summary() {
    eprintln!();
    eprintln!("=== Request Parsing Performance Summary ===");
    eprintln!();

    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("Minimal GET", b"GET / HTTP/1.1\r\n\r\n".to_vec()),
        (
            "Typical GET",
            b"GET /api/v1/users HTTP/1.1\r\n\
Host: www.example.com\r\n\
User-Agent: Mozilla/5.0\r\n\
Accept: text/html\r\n\
Connection: keep-alive\r\n\
\r\n"
                .to_vec(),
        ),
        (
            "POST + body",
            b"POST /submit HTTP/1.1\r\n\
Content-Type: application/json\r\n\
Content-Length: 27\r\n\
\r\n\
{\"key\":\"value\",\"num\":42}"
                .to_vec(),
        ),
    ];

    for (label, raw) in &cases {
        let elapsed = bench(WARMUP_ITERATIONS, ITERATIONS, || {
            let result = Request::parse(black_box(raw));
            black_box(result).ok();
        });

        report(label, elapsed, ITERATIONS, raw.len());
    }
}
