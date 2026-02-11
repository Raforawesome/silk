use std::net::TcpListener;

fn main() {
    let bind_address = "127.0.0.1:7878";
    let listener = TcpListener::bind(bind_address).unwrap();
    println!("Listening on {bind_address}");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        silk::connection::connection_dispatch(stream);
    }
}
