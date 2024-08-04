use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn handle_client(mut stream: TcpStream) {
    let mut bytes = Vec::new();
    let res = stream.read_to_end(&mut bytes);

    if res.is_err() {
        return;
    }

    println!("{:?}", bytes);

    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:10000").expect("Failed to bind to port");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        thread::spawn(|| {
            handle_client(stream);
        });
    }

    Ok(())
}
