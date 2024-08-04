use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

#[derive(Deserialize)]
struct Request {
    method: String,
    number: f64,
}

#[derive(Serialize, Debug)]
struct Response {
    method: String,
    prime: bool,
}

fn handle_client(mut stream: TcpStream) {
    let buf = BufReader::new(stream.try_clone().unwrap());
    for line in buf.lines() {
        let string = line.unwrap();

        let mut malformed = false;
        println!("Request {}", &string.trim());

        let req: Request = match serde_json::from_str(&string) {
            Ok(req) => req,
            Err(_) => {
                println!("Error deserializing");
                malformed = true;
                Request {
                    method: String::from("isPrime"),
                    number: 1.0,
                }
            }
        };

        if req.method != "isPrime" {
            malformed = true;
        }

        let response: Response;
        if malformed {
            println!("Malformed!");
            response = Response {
                method: String::from("malformed"),
                prime: false,
            };
        } else {
            println!("Valid!");
            response = Response {
                method: String::from("isPrime"),
                prime: match req.number as u64 {
                    ..=1 => false,
                    2.. => primes::is_prime(req.number as u64),
                },
            }
        }

        println!("{:?}", response);
        let mut response = serde_json::to_string(&response).unwrap();
        response.push('\n');
        let _ = stream.write(Vec::from(response).as_ref());
        let _ = stream.flush();
        if malformed {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return;
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:10000").expect("Failed to bind to port");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("Connection: {}", stream.peer_addr().unwrap());
        thread::spawn(|| {
            handle_client(stream);
        });
    }

    Ok(())
}
