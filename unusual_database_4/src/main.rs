use std::{collections::HashMap, net::UdpSocket};

fn main() {
    let udp_listener = UdpSocket::bind("0.0.0.0:10000").unwrap();
    let mut database: HashMap<String, String> = HashMap::new();
    let mut buf = [0; 1000];

    database.insert(String::from("version"), String::from("1.2.3"));

    loop {
        let (size, src) = udp_listener.recv_from(&mut buf).unwrap();
        println!("Received {size} bytes from {src}");
        let msg = String::from_utf8(buf[..size].to_vec()).unwrap();

        if msg.contains('=') {
            // Insert request
            let (key, value) = msg.split_once('=').unwrap();

            // Ignore version requests
            if key == "version" {
                continue;
            }

            database.insert(String::from(key), String::from(value));
        } else {
            // Retrieve request
            let value = match database.get(msg.as_str()) {
                Some(v) => v,
                None => "",
            };

            let response = msg + "=" + value;
            let _ = udp_listener.send_to(response.as_bytes(), src);
        }
    }
}
