use std::{io, net::TcpListener};

use silk::connection::{connection_dispatch, thread_pool::ThreadPool};

fn main() -> io::Result<()> {
    let bind_address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7878".into());
    let listener = TcpListener::bind(&bind_address)?;
    let thread_count = num_cpus::get();
    let mut thread_pool = ThreadPool::new(thread_count, thread_count * 2, connection_dispatch);

    println!(
        "Starting {thread_count} workers with {} queue slots",
        thread_count * 2
    );
    println!("Listening on {}", listener.local_addr()?);

    let result = loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = thread_pool.submit_task(stream);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => break Err(error),
        }
    };
    if let Err(error) = thread_pool.shutdown() {
        eprintln!("shutdown failed: {error}");
    }
    result
}
