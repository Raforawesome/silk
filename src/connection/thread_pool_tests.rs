use super::*;
use crate::{connection::handle_connection, test_support::bounded};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Duration,
};

fn pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    (client, listener.accept().unwrap().0)
}

fn announced_handler(
    stream: &mut TcpStream,
    context: &mut ConnectionContext,
) -> Result<(), ConnectionError> {
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    stream.write_all(b"active").unwrap();
    handle_connection(
        stream,
        context,
        Duration::from_millis(300),
        Duration::from_secs(1),
    )
}

#[test]
fn queue_saturation_and_shutdown_discard_waiting_connections() {
    bounded(
        "connection::thread_pool::tests::queue_saturation_and_shutdown_discard_waiting_connections",
        || {
            let mut pool = ThreadPool::new(1, 1, announced_handler);
            let (mut active, server) = pair();
            pool.submit_task(server).unwrap();
            let mut marker = [0; 6];
            active.read_exact(&mut marker).unwrap();
            assert_eq!(&marker, b"active"); // Worker owns the first connection before queueing more.
            let (mut queued, server) = pair();
            pool.submit_task(server).unwrap();
            let (mut rejected, server) = pair();
            assert_eq!(pool.submit_task(server), Err(SubmitError::Full));
            assert_eq!(rejected.read(&mut [0]).unwrap(), 0);
            pool.shutdown().unwrap();
            // The active connection expires, while the queued connection never enters the handler.
            assert_eq!(active.read(&mut [0]).unwrap(), 0);
            assert_eq!(queued.read(&mut [0]).unwrap(), 0);
            assert!(pool.workers.is_empty());
            pool.shutdown().unwrap();
            let (_client, server) = pair();
            assert_eq!(pool.submit_task(server), Err(SubmitError::Closed));
        },
    );
}

#[test]
fn idle_shutdown_and_sender_disconnection_join_workers() {
    bounded(
        "connection::thread_pool::tests::idle_shutdown_and_sender_disconnection_join_workers",
        || {
            let mut pool = ThreadPool::new(2, 2, announced_handler);
            pool.shutdown().unwrap();
            assert!(pool.workers.is_empty());
            let mut pool = ThreadPool::new(2, 2, announced_handler);
            pool.sender.take();
            // Join directly to prove recv exits on disconnection without needing the stop flag.
            for worker in pool.workers.drain(..) {
                worker.join().unwrap();
            }
            pool.shutdown().unwrap();
        },
    );
}

#[test]
fn drop_joins_active_workers_and_reports_panics_without_panicking() {
    bounded(
        "connection::thread_pool::tests::drop_joins_active_workers_and_reports_panics_without_panicking",
        || {
            let pool = ThreadPool::new(1, 1, announced_handler);
            let (mut client, server) = pair();
            pool.submit_task(server).unwrap();
            client.read_exact(&mut [0; 6]).unwrap();
            drop(pool);
            assert_eq!(client.read(&mut [0]).unwrap(), 0);
            let mut pool = ThreadPool::new(1, 1, |stream, _| {
                stream.write_all(b"x").unwrap();
                panic!("deliberate worker panic");
            });
            let (mut client, server) = pair();
            pool.submit_task(server).unwrap();
            client.read_exact(&mut [0]).unwrap();
            assert!(pool.shutdown().is_err());
            pool.shutdown().unwrap();
        },
    );
}

#[test]
#[should_panic(expected = "at least one worker")]
fn zero_workers_are_rejected() {
    ThreadPool::new(0, 1, announced_handler);
}
