use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub use silk::ToBytes;

// Include the actual source so head-only measurements can call private helpers
// without adding benchmark hooks or configuration to the library's public API.
pub mod http {
    pub use silk::http::{Method, Version};
    pub mod headers {
        include!("../src/http/headers.rs");
    }
    // harness=false compiles the included #[cfg(test)] regression helpers without a test runner.
    #[allow(dead_code)]
    pub mod request {
        include!("../src/http/request.rs");
        pub mod measurements {
            include!("support/measurements.rs");
        }
    }
}

struct CountingAllocator;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOC: AtomicUsize = AtomicUsize::new(0);
static ZEROED: AtomicUsize = AtomicUsize::new(0);
static REALLOC: AtomicUsize = AtomicUsize::new(0);

// Safety: all pointers, layouts and sizes are forwarded unchanged to System.
// Counters use only atomics and never allocate or unwind inside allocator callbacks.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOC.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ZEROED.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            REALLOC.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocations(action: impl FnOnce()) -> (usize, usize, usize) {
    ALLOC.store(0, Ordering::Relaxed);
    ZEROED.store(0, Ordering::Relaxed);
    REALLOC.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
    action();
    COUNTING.store(false, Ordering::Relaxed);
    (
        ALLOC.load(Ordering::Relaxed),
        ZEROED.load(Ordering::Relaxed),
        REALLOC.load(Ordering::Relaxed),
    )
}

fn main() {
    http::request::measurements::run();
}
