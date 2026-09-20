use std::{
    net::TcpStream,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crossbeam_channel::{Sender, TrySendError};

use super::{ConnectionContext, ConnectionError};

#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError {
    Full,
    Closed,
}

pub struct ThreadPool {
    sender: Option<Sender<TcpStream>>,
    workers: Vec<JoinHandle<()>>,
    stopping: Arc<AtomicBool>,
}

impl ThreadPool {
    pub fn new(
        threads: usize,
        queue_capacity: usize,
        conn_handler: fn(&mut TcpStream, &mut ConnectionContext) -> Result<(), ConnectionError>,
    ) -> Self {
        assert!(threads > 0, "thread pool needs at least one worker");
        let (sender, receiver) = crossbeam_channel::bounded::<TcpStream>(queue_capacity);
        let stopping = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::with_capacity(threads);

        for _ in 0..threads {
            let receiver = receiver.clone();
            let stopping = Arc::clone(&stopping);
            workers.push(thread::spawn(move || {
                let mut context = ConnectionContext::default();
                while let Ok(mut stream) = receiver.recv() {
                    // A connection past this check is active; its handler must enforce deadlines.
                    if stopping.load(Ordering::Acquire) {
                        break;
                    }
                    if let Err(error) = conn_handler(&mut stream, &mut context) {
                        match error {
                            ConnectionError::PeerClosed | ConnectionError::IncompleteRequest => (),
                            error => eprintln!("connection failed: {error:?}"),
                        }
                    }
                }
                // On stop, receiver destruction discards queued sockets once all workers exit.
            }));
        }
        Self {
            sender: Some(sender),
            workers,
            stopping,
        }
    }

    /// Rejected sockets are dropped here; admission never waits for a worker or writes a response.
    pub fn submit_task(&self, stream: TcpStream) -> Result<(), SubmitError> {
        let Some(sender) = &self.sender else {
            return Err(SubmitError::Closed);
        };
        sender.try_send(stream).map_err(|error| match error {
            TrySendError::Full(_) => SubmitError::Full,
            TrySendError::Disconnected(_) => SubmitError::Closed,
        })
    }

    /// Stop admission, discard queued work, then join active workers. Safe to call repeatedly.
    /// A custom handler must bound its own I/O, as connection_dispatch does.
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.stopping.store(true, Ordering::Release);
        self.sender.take();
        let mut panicked = false;
        for worker in self.workers.drain(..) {
            panicked |= worker.join().is_err();
        }
        if panicked {
            Err("a worker panicked")
        } else {
            Ok(())
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
#[path = "thread_pool_tests.rs"]
mod tests;
