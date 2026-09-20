use std::net::TcpStream;

use crossbeam_channel::{Receiver, Sender};

use super::{ConnectionContext, ConnectionError};

pub struct ThreadPool {
    sender: Sender<TcpStream>,
    // receiver: Receiver<TcpStream>,
    conn_handler: fn(&mut TcpStream, &mut ConnectionContext) -> Result<(), ConnectionError>,
}

impl ThreadPool {
    pub fn new(
        threads: usize,
        conn_handler: fn(&mut TcpStream, &mut ConnectionContext) -> Result<(), ConnectionError>,
    ) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(threads);

        for thread_id in 0..threads {
            let receiver = receiver.clone(); // create a new handle to move into the thread

            std::thread::spawn(move || {
                let mut context = ConnectionContext::default();
                loop {
                    while let Ok(mut stream) = receiver.recv() {
                        if let Err(error) = conn_handler(&mut stream, &mut context) {
                            match error {
                                ConnectionError::PeerClosed
                                | ConnectionError::IncompleteRequest => (),
                                error => eprintln!("connection failed: {error:?}"),
                            }
                        }
                        // Dropping the stream closes it, including after a failed write.
                    }

                    eprintln!("thread with id {thread_id} quit due to sender disconnection");
                }
            });
        }

        Self {
            sender,
            // receiver,
            conn_handler,
        }
    }
}

impl ThreadPool {
    pub fn submit_task(&self, task: TcpStream) {
        match self.sender.send(task) {
            Ok(_) => (),
            Err(e) => eprintln!("failed to send task: {e}"),
        }
    }
}
