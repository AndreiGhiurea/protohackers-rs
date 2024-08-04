use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

fn prompt_username(stream: TcpStream) -> Option<String> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream.try_clone().unwrap());

    let res = writer.write_all("Welcome to budgetchat! What shall I call you?\n".as_bytes());
    let _ = writer.flush();
    if res.is_err() {
        return None;
    }

    let mut username = String::new();
    let result = reader.read_line(&mut username);

    println!("{username}");

    match result {
        Ok(_) => {
            let username = username.trim();
            let re = regex::Regex::new(r"^[a-zA-Z0-9]+$");
            if !username.is_empty() && re.unwrap().is_match(username) {
                Some(String::from(username))
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

fn add_new_user(
    username: &String,
    stream: &mut TcpStream,
    clients: &Arc<Mutex<HashMap<String, TcpStream>>>,
) {
    // Announance current user
    clients
        .lock()
        .unwrap()
        .values_mut()
        .for_each(|other_client| {
            let _ =
                other_client.write_all(format!("* {} has entered the room\n", username).as_bytes());
            let _ = other_client.flush();
        });

    // List online users
    let _ = stream.write_all(
        format!(
            "* The room contains: {:?}\n",
            clients.lock().unwrap().keys()
        )
        .as_bytes(),
    );
    let _ = stream.flush();

    // Add client to the list
    clients
        .lock()
        .unwrap()
        .insert(username.clone(), stream.try_clone().unwrap());
}

fn read_message(reader: &mut BufReader<TcpStream>) -> Option<String> {
    let mut message = String::new();

    message.clear();
    let result = reader.read_line(&mut message);

    if result.is_err() || result.is_ok_and(|v| v == 0) {
        None
    } else {
        Some(message)
    }
}

fn disconnect_user(
    username: &String,
    stream: &mut TcpStream,
    clients: &Arc<Mutex<HashMap<String, TcpStream>>>,
) {
    // Assume the client has closed the connection
    // Remove the user from the list
    clients.lock().unwrap().remove(username);

    let _ = stream.shutdown(std::net::Shutdown::Both);

    // Announce user left
    clients
        .lock()
        .unwrap()
        .values_mut()
        .for_each(|other_client| {
            let _ =
                other_client.write_all(format!("* {} has left the room\n", username).as_bytes());
            let _ = other_client.flush();
        });
}

fn broadcast_message(
    current_user: &String,
    message: &String,
    clients: &Arc<Mutex<HashMap<String, TcpStream>>>,
) {
    // Sends meesage to all other clients.
    clients
        .lock()
        .unwrap()
        .iter_mut()
        .filter(|(k, _)| k.as_str() != current_user)
        .for_each(|(_, v)| {
            let _ = v.write_all(format!("[{current_user}] {message}\n").as_bytes());
            let _ = v.flush();
        });
}

fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<HashMap<String, TcpStream>>>) {
    let username = match prompt_username(stream.try_clone().unwrap()) {
        Some(name) => name,
        None => {
            println!("Invalid username, disconnecting");
            return;
        }
    };

    // Add the new user.
    add_new_user(&username, &mut stream, &clients);

    // Message loop
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    loop {
        let message = match read_message(&mut reader) {
            Some(msg) => String::from(msg.trim()),
            None => {
                disconnect_user(&username, &mut stream, &clients);
                return;
            }
        };

        broadcast_message(&username, &message, &clients);
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:10000").unwrap();
    let clients: Arc<Mutex<HashMap<String, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let clients = clients.clone();

        thread::spawn(|| {
            handle_client(stream, clients);
        });
    }
}
