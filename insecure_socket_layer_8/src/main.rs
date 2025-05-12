use std::{
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    str::FromStr,
    thread,
};

fn log_with_thread_id(message: &str) {
    let thread_id = thread::current().id();
    println!("[Thread {:?}] {}", thread_id, message);
}

#[derive(Clone)]
enum CipherOp {
    EndOfCipherSpec,
    ReverseBits,
    Xor(u8),
    Xorpos,
    Add(u8),
    Addpos,
}

fn read_cipher(stream: &mut TcpStream) -> Vec<CipherOp> {
    let mut cipher: Vec<CipherOp> = Vec::new();

    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).expect("Failed to read byte");

        log_with_thread_id(&format!("Read cipher byte: {:#x}", byte[0]));

        let cipher_op = match byte[0] {
            0 => CipherOp::EndOfCipherSpec,
            1 => CipherOp::ReverseBits,
            2 => {
                let mut xor_value = [0u8; 1];
                stream
                    .read_exact(&mut xor_value)
                    .expect("Failed to read xor value");
                log_with_thread_id(&format!("Xor value: {:#x}", xor_value[0]));
                CipherOp::Xor(xor_value[0])
            }
            3 => CipherOp::Xorpos,
            4 => {
                let mut add_value = [0u8; 1];
                stream
                    .read_exact(&mut add_value)
                    .expect("Failed to read add value");
                log_with_thread_id(&format!("Add value: {:#x}", add_value[0]));
                CipherOp::Add(add_value[0])
            }
            5 => CipherOp::Addpos,
            _ => {
                let _ = stream.shutdown(std::net::Shutdown::Both);
                panic!("Invalid cipher operation");
            }
        };

        if let CipherOp::EndOfCipherSpec = cipher_op {
            break;
        }

        cipher.push(cipher_op);
    }

    cipher
}

fn check_cipher_validity(cipher: Vec<CipherOp>) -> bool {
    let sample_input = "Hello, World!";
    let mut stream_pos = 0usize;
    let mut valid = true;

    let encoded = encode(sample_input.as_bytes(), cipher.clone(), &mut stream_pos);

    if encoded == sample_input.as_bytes() {
        valid = false;
    }

    stream_pos = 0;
    let decoded = decode(&encoded, cipher.clone(), &mut stream_pos);

    if decoded != sample_input {
        panic!("Cipher decoding failed");
    }

    valid
}

fn encode(input: &[u8], cipher: Vec<CipherOp>, stream_pos: &mut usize) -> Vec<u8> {
    let mut result = Vec::new();

    for byte in input.bytes() {
        let mut modified_byte = byte.expect("Failed to get byte");

        for op in &cipher {
            match op {
                CipherOp::ReverseBits => {
                    modified_byte = modified_byte.reverse_bits();
                }
                CipherOp::Xor(x) => {
                    modified_byte ^= *x;
                }
                CipherOp::Xorpos => {
                    modified_byte ^= (*stream_pos % 256) as u8;
                }
                CipherOp::Add(x) => {
                    modified_byte = modified_byte.wrapping_add(*x);
                }
                CipherOp::Addpos => {
                    modified_byte = modified_byte.wrapping_add((*stream_pos % 256) as u8);
                }
                _ => {}
            }
        }

        *stream_pos += 1;
        result.push(modified_byte);
    }

    result
}

fn decode(input: &[u8], cipher: Vec<CipherOp>, stream_pos: &mut usize) -> String {
    let mut result = String::new();

    for byte in input {
        let mut modified_byte = *byte;

        for op in cipher.iter().rev() {
            match op {
                CipherOp::ReverseBits => {
                    modified_byte = modified_byte.reverse_bits();
                }
                CipherOp::Xor(x) => {
                    modified_byte ^= *x;
                }
                CipherOp::Xorpos => {
                    modified_byte ^= (*stream_pos % 256) as u8;
                }
                CipherOp::Add(x) => {
                    modified_byte = modified_byte.wrapping_sub(*x);
                }
                CipherOp::Addpos => {
                    modified_byte = modified_byte.wrapping_sub((*stream_pos % 256) as u8);
                }
                _ => {}
            }
        }

        *stream_pos += 1;
        result.push(modified_byte as char);
    }

    result
}

fn read_request_and_decode(
    stream: &mut TcpStream,
    cipher: Vec<CipherOp>,
    client_stream_pos: &mut usize,
) -> String {
    let mut byte = vec![0u8; 1];
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut decoded_request = String::new();

    loop {
        let res = reader.read_exact(&mut byte);

        if res.is_err() {
            return decoded_request;
        }

        let decoded_byte = decode(&byte, cipher.clone(), client_stream_pos);
        decoded_request.push_str(decoded_byte.as_str());

        if decoded_byte == "\n" {
            break;
        }
    }

    decoded_request
}

fn handle_client(mut stream: TcpStream) {
    let mut client_stream_pos = 0usize;
    let mut server_stream_pos = 0usize;

    let cipher = read_cipher(&mut stream);

    if check_cipher_validity(cipher.clone()) {
        log_with_thread_id("Cipher is valid");
    } else {
        log_with_thread_id("Cipher is invalid");
        let _ = stream.shutdown(std::net::Shutdown::Both);
        return;
    }

    loop {
        log_with_thread_id("Ready to receive request");
        let decoded_request =
            read_request_and_decode(&mut stream, cipher.clone(), &mut client_stream_pos);

        if decoded_request.is_empty() {
            log_with_thread_id("No request received, closing connection");
            break;
        }

        if !decoded_request.is_ascii() {
            log_with_thread_id(format!("Client stream pos: {}", client_stream_pos).as_str());
            log_with_thread_id(format!("Received non-ASCII request: {}", decoded_request).as_str());
            continue;
        }

        log_with_thread_id(&format!("Decoded request: {}", decoded_request));

        let response = decoded_request
            .split(',')
            .max_by(|a, b| {
                let a_nr = a
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<u32>()
                    .expect("Failed to parse number");
                let b_nr = b
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<u32>()
                    .expect("Failed to parse number");

                a_nr.cmp(&b_nr)
            })
            .expect("No valid response found")
            .trim();

        let mut response = String::from_str(response).unwrap();
        response.push('\n');

        let encoded_response = encode(response.as_bytes(), cipher.clone(), &mut server_stream_pos);

        stream
            .write_all(&encoded_response)
            .expect("Failed to write response");
        log_with_thread_id(&format!("Sent response: {}", response));
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:10000").unwrap();

    for stream in listener.incoming() {
        thread::spawn(|| {
            handle_client(stream.unwrap());
        });
    }
}
