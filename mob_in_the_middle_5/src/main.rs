use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};

const TONY_ADDRESS: &str = "7YWHMfk9JZe0LM0g1ZauHuiSxhI";
const BUDGET_CHAT_ADDR: &str = "chat.protohackers.com:16963";

async fn handle_incoming<'a>(
    addr: &SocketAddr,
    mut server_reader: BufReader<ReadHalf<'a>>,
    mut client: WriteHalf<'a>,
) {
    let mut srv_message = String::new();
    let re_address = fancy_regex::Regex::new(r"(?<!\S)7[a-zA-Z0-9]{25,34}(?!\S)").unwrap();

    loop {
        srv_message.clear();

        let res = server_reader.read_line(&mut srv_message).await;
        if res.is_err() {
            return;
        }

        if res.is_ok_and(|x| x != 0) {
            let srv_message = re_address.replace_all(srv_message.as_str(), TONY_ADDRESS);
            println!("[server {addr}]: {srv_message}");
            let _ = client.write_all(srv_message.as_bytes()).await;
            let _ = client.flush().await;
        }
    }
}

async fn handle_outgoing<'a>(
    addr: &SocketAddr,
    mut client_reader: BufReader<ReadHalf<'a>>,
    mut server: WriteHalf<'a>,
) {
    let mut client_message = String::new();
    let re_address = fancy_regex::Regex::new(r"(?<!\S)7[a-zA-Z0-9]{25,34}(?!\S)").unwrap();

    loop {
        client_message.clear();

        let res = client_reader.read_line(&mut client_message).await;
        if res.is_err() {
            return;
        }

        if res.is_ok_and(|x| x != 0) {
            let client_message = re_address.replace_all(client_message.as_str(), TONY_ADDRESS);
            println!("[client {addr}]: {client_message}");
            let _ = server.write_all(client_message.as_bytes()).await;
            let _ = server.flush().await;
        } else {
            return;
        }
    }
}

async fn handle_client(mut client: TcpStream) {
    // Initialize a connection to budget chat.
    let addr = client.peer_addr().unwrap();
    println!("Client connection from: {}", addr);
    let mut server = TcpStream::connect(BUDGET_CHAT_ADDR).await.unwrap();

    let (srv_read, srv_write) = server.split();
    let (client_read, client_write) = client.split();
    let server_reader = BufReader::new(srv_read);
    let client_reader = BufReader::new(client_read);

    let incoming = handle_incoming(&addr, server_reader, client_write);
    let outgoing = handle_outgoing(&addr, client_reader, srv_write);

    tokio::select! {
        () = incoming => (),
        () = outgoing => (),
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:10000").await.unwrap();

    loop {
        let stream = listener.accept().await.unwrap();
        tokio::spawn(async move {
            handle_client(stream.0).await;
        });
    }
}
