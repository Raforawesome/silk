#![allow(clippy::pedantic)]
use std::net::TcpListener;

use silk::connection::{connection_dispatch, thread_pool::ThreadPool};

fn main() {
    let bind_address = "127.0.0.1:7878";
    let listener = TcpListener::bind(bind_address).unwrap();

    let thread_count = num_cpus::get();
    let thread_pool = ThreadPool::new(thread_count, connection_dispatch);

    println!("starting thread pool with size {thread_count}");
    println!("Listening on {bind_address}");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        thread_pool.submit_task(stream);
    }
}
