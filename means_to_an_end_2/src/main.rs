use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn handle_insert(
    assets: &mut BTreeMap<i32, i32>,
    timestamp: i32,
    value: i32,
) -> Result<Option<i32>, String> {
    let result = assets.insert(timestamp, value);

    if result.is_some() {
        return Err(String::from("Timestamp is already populated"));
    }

    Ok(result)
}

fn handle_query(
    assets: &mut BTreeMap<i32, i32>,
    mintime: i32,
    maxtime: i32,
) -> Result<Option<i32>, String> {
    if maxtime < mintime {
        return Ok(Some(0));
    }

    let range = assets.range(mintime..=maxtime);
    let count: isize = range.clone().count() as isize;

    if count == 0 {
        return Ok(Some(0));
    }

    let mut sum: isize = 0;
    range.for_each(|(_, v)| {
        sum += *v as isize;
    });

    Ok(Some((sum / count) as i32))
}

fn handle_client(mut stream: TcpStream) {
    let mut assets = BTreeMap::new();
    let mut msg: [u8; 9] = [0; 9];

    loop {
        let _ = stream.read_exact(&mut msg);
        let msg_type: char = msg.first().unwrap().to_owned() as char;
        let param1 = i32::from_be_bytes(msg.get(1..5).unwrap().try_into().unwrap());
        let param2 = i32::from_be_bytes(msg.get(5..9).unwrap().try_into().unwrap());

        // println!("type: {}\nparam1: {}\nparam2: {}", msg_type, param1, param2);

        let result = match msg_type {
            'I' => handle_insert(&mut assets, param1, param2),
            'Q' => handle_query(&mut assets, param1, param2),
            _ => {
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return;
            }
        };

        let response = match result {
            Ok(opt) => opt,
            Err(e) => {
                println!("{}", e);
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return;
            }
        };

        if let Some(response) = response {
            let _ = stream.write_all(&response.to_be_bytes());
        }
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
