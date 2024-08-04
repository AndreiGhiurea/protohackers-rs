use std::net::TcpListener;

fn main() {
    // Start tcp listener
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move || {
                    handle_connection(stream);
                });
            }
            Err(_) => {}
        }
    }
}
