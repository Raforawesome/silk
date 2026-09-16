use std::{io::Write as _, net::TcpStream};

use crossbeam_channel::{Receiver, Sender};

use crate::{ToBytes as _, http::response::Response};

pub struct ThreadPool {
    sender: Sender<TcpStream>,
    // receiver: Receiver<TcpStream>,
    conn_handler: fn(&mut TcpStream) -> Result<Response, ()>,
}

impl ThreadPool {
    pub fn new(threads: usize, conn_handler: fn(&mut TcpStream) -> Result<Response, ()>) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(threads);

        for thread_id in 0..threads {
            let receiver = receiver.clone(); // create a new handle to move into the thread

            std::thread::spawn(move || {
                loop {
                    while let Ok(mut stream) = receiver.recv() {
                        println!("handling connection from thread {thread_id}");
                        let response = conn_handler(&mut stream);

                        match response {
                            Ok(response) => stream.write_all(&response.to_bytes()).unwrap(),
                            Err(()) => stream.shutdown(std::net::Shutdown::Both).unwrap(),
                        }
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
