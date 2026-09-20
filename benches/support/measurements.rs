use super::*;
use std::{hint::black_box, process::Command, time::Instant};

const WARMUP: usize = 50_000;
const ITERATIONS: usize = 200_000;
const SAMPLES: usize = 7;

fn metadata(label: &str, command: &str, args: &[&str]) {
    let result = Command::new(command)
        .args(args)
        .output()
        .expect("metadata command");
    assert!(
        result.status.success(),
        "metadata command failed: {command}"
    );
    println!(
        "{label}: {}",
        String::from_utf8_lossy(&result.stdout).trim()
    );
}

fn measure(name: &str, bytes: usize, mut operation: impl FnMut()) {
    for _ in 0..WARMUP {
        operation();
    }
    let mut samples = [0.0; SAMPLES];
    for sample in &mut samples {
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            operation();
        }
        *sample = start.elapsed().as_nanos() as f64 / ITERATIONS as f64;
    }
    samples.sort_by(f64::total_cmp);
    let median = samples[SAMPLES / 2];
    println!(
        "{name}: median_ns={median:.2}, range_ns={:.2}..{:.2}, operations_per_second={:.0}, logical_GB_per_second={:.4}",
        samples[0],
        samples[SAMPLES - 1],
        1e9 / median,
        bytes as f64 / median
    );
}

fn scalar_delimiter(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

pub fn run() {
    println!("Silk parser measurements (single-threaded, release bench profile)");
    metadata("commit", "git", &["rev-parse", "HEAD"]);
    metadata("dirty_status", "git", &["status", "--porcelain"]);
    metadata("compiler", "rustc", &["+stable", "-vV"]);
    if cfg!(target_os = "macos") {
        metadata("OS", "sw_vers", &[]);
        metadata("CPU", "sysctl", &["-n", "machdep.cpu.brand_string"]);
    } else {
        println!("OS: {} {}", std::env::consts::OS, std::env::consts::ARCH);
        if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
            println!(
                "CPU: {}",
                info.lines()
                    .find(|line| line.starts_with("model name"))
                    .unwrap_or("see /proc/cpuinfo")
            );
        }
    }
    println!(
        "RUSTFLAGS: {:?}",
        std::env::var("RUSTFLAGS").unwrap_or_default()
    );
    println!("command: cargo +stable bench --bench parser");
    println!("warmup={WARMUP}, iterations_per_sample={ITERATIONS}, samples={SAMPLES}");
    println!(
        "GB/s uses logical head bytes once, excluding body; not memory traffic or network throughput."
    );

    let short = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec();
    let typical = b"GET /api/users?page=1 HTTP/1.1\r\nHost: example.com\r\nUser-Agent: silk-bench\r\nAccept: application/json\r\nAccept-Encoding: gzip\r\nConnection: close\r\n\r\n".to_vec();
    let mut many = b"GET / HTTP/1.1\r\nHost: example.com\r\n".to_vec();
    for i in 0..40 {
        many.extend_from_slice(format!("X-Field-{i}: value-{i}-abcdef\r\n").as_bytes());
    }
    many.extend_from_slice(b"\r\n");
    let mut body =
        b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 8192\r\n\r\n".to_vec();
    body.extend_from_slice(&[b'x'; 8192]);
    let fixtures = [
        ("short", short),
        ("typical", typical),
        ("many_headers", many),
        ("body_8192", body),
    ];

    for (name, bytes) in &fixtures {
        let head_end = find(bytes, b"\r\n\r\n").unwrap() + 4;
        let ParseStatus::Complete { request, consumed } = Request::parse(bytes).unwrap() else {
            panic!("invalid fixture")
        };
        assert_eq!(consumed, bytes.len());
        assert_eq!(request.body().len(), bytes.len() - head_end);
        let head = parse_head(bytes, head_end, MAX_HEADER_FIELDS, MAX_REQUEST_BYTES).unwrap();
        assert_eq!(head.consumed, consumed);
        // Verify the actual library entry point agrees with the source-included harness.
        let silk::http::request::ParseStatus::Complete {
            consumed: library_consumed,
            ..
        } = silk::http::request::Request::parse(bytes).unwrap()
        else {
            panic!()
        };
        assert_eq!(consumed, library_consumed);
        let mut incremental = RequestParser::default();
        for end in 0..bytes.len() {
            black_box(incremental.parse(&bytes[..end]).unwrap());
        }
        assert!(matches!(
            incremental.parse(bytes),
            Ok(ParseStatus::Complete { .. })
        ));
        let head_bytes = &bytes[..head_end];
        assert_eq!(scalar_delimiter(head_bytes), find(head_bytes, b"\r\n\r\n"));
        println!(
            "\nfixture={name}, total_bytes={}, head_bytes={head_end}, body_bytes={}",
            bytes.len(),
            bytes.len() - head_end
        );
        measure("full_parser", head_end, || {
            // Measure the library itself, not a substitute implementation.
            let parsed = silk::http::request::Request::parse(black_box(bytes));
            let silk::http::request::ParseStatus::Complete { request, consumed } = parsed.unwrap()
            else {
                panic!()
            };
            black_box((request, consumed));
        });
        measure("head_validation_only", head_end, || {
            black_box(
                parse_head(
                    black_box(bytes),
                    black_box(head_end),
                    MAX_HEADER_FIELDS,
                    MAX_REQUEST_BYTES,
                )
                .unwrap(),
            );
        });
        measure("delimiter_scalar", head_end, || {
            black_box(scalar_delimiter(black_box(head_bytes)));
        });
        measure("delimiter_memchr", head_end, || {
            black_box(find(black_box(head_bytes), black_box(b"\r\n\r\n")));
        });
    }

    // Sanity-check all three callbacks using explicit allocation calls. The layouts
    // and returned pointers are paired, checked for null, and released with System's contract.
    let sanity = crate::allocations(|| unsafe {
        use std::alloc::{Layout, alloc, alloc_zeroed, dealloc, realloc};
        let layout = Layout::from_size_align(32, 8).unwrap();
        let pointer = black_box(alloc(layout));
        assert!(!pointer.is_null());
        let grown = black_box(realloc(pointer, layout, 64));
        assert!(!grown.is_null());
        dealloc(grown, Layout::from_size_align(64, 8).unwrap());
        let zeroed = black_box(alloc_zeroed(layout));
        assert!(!zeroed.is_null());
        dealloc(zeroed, layout);
    });
    assert_eq!(sanity, (1, 1, 1));
    println!("\nallocator_sanity (alloc, zeroed, realloc): {sanity:?}");
    let incomplete = b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 3\r\n\r\nab";
    let invalid = b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\n";
    assert!(matches!(
        silk::http::request::Request::parse(incomplete),
        Ok(silk::http::request::ParseStatus::Incomplete)
    ));
    assert!(silk::http::request::Request::parse(invalid).is_err());
    for (label, bytes) in fixtures
        .iter()
        .map(|(name, bytes)| (*name, bytes.as_slice()))
        .chain([
            ("incomplete_body", incomplete.as_slice()),
            ("invalid_duplicate_length", invalid.as_slice()),
            ("incomplete_head", b"GET / HTTP/1.1\r\nHost:".as_slice()),
        ])
    {
        for _ in 0..WARMUP {
            black_box(silk::http::request::Request::parse(bytes)).ok();
        }
        let counts = crate::allocations(|| {
            for _ in 0..ITERATIONS {
                black_box(silk::http::request::Request::parse(black_box(bytes))).ok();
            }
        });
        assert_eq!(counts, (0, 0, 0), "parser allocated: {label}");
        println!("allocations {label}: {counts:?} across {ITERATIONS} parses");
    }
}
