mod isl;

use isl::InsecureSocketLayer;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;

fn get_most_toys(toys: String) -> Result<String, String> {
    let parsed: Result<Vec<(u32, String)>, String> = toys
        .split(',')
        .map(|x| {
            let mut parts = x.splitn(2, 'x');
            let count = parts
                .next()
                .ok_or("Invalid input string")?
                .trim()
                .parse::<u32>()
                .map_err(|_| "Invalid input string")?;
            let name = parts
                .next()
                .ok_or("Invalid input string")?
                .trim()
                .to_string();
            Ok((count, name))
        })
        .collect();

    let parsed = parsed?;
    let most_toys = parsed
        .into_iter()
        .max_by_key(|(count, _)| *count)
        .ok_or("Invalid input string")?;

    Ok(format!("{}x {}\n", most_toys.0, most_toys.1))
}

async fn handle_client<S>(socket: S) -> Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    //
    // Create the socket layer.
    //
    let mut isl = InsecureSocketLayer::new(socket).await?;

    //
    // Process requests.
    //
    loop {
        let request = isl.read().await?;

        let response = get_most_toys(request)?;

        isl.write(response).await?;
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:10000").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            match handle_client(socket).await {
                Ok(_) => (),
                Err(str) => println!("{}", str),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::runtime::Runtime;

    #[test]
    fn basic_toys() {
        let s = "1x dog,3x car,2x rat\n";
        assert_eq!(get_most_toys(String::from(s)).unwrap(), "3x car\n");
    }

    #[test]
    fn invalid_toys() {
        let s = "1x dog,3 car,2x rat\n";
        assert_eq!(
            get_most_toys(String::from(s)),
            Err(String::from("Invalid input string"))
        );

        let s = "\n";
        assert_eq!(
            get_most_toys(String::from(s)),
            Err(String::from("Invalid input string"))
        );
    }

    #[test]
    fn test_handle_client_xor123_addpos_reversebits() {
        // Cipher: xor(123), addpos, reversebits
        let cipher = [0x02, 0x7b, 0x05, 0x01, 0x00];
        // First client request (obfuscated): 4x dog,5x car\n
        let req1 = [
            0xf2, 0x20, 0xba, 0x44, 0x18, 0x84, 0xba, 0xaa, 0xd0, 0x26, 0x44, 0xa4, 0xa8, 0x7e,
        ];
        // First server response (obfuscated): 5x car\n
        let resp1 = [0x72, 0x20, 0xba, 0xd8, 0x78, 0x70, 0xee];
        // Second client request (obfuscated): 3x rat,2x cat\n
        let req2 = [
            0x6a, 0x48, 0xd6, 0x58, 0x34, 0x44, 0xd6, 0x7a, 0x98, 0x4e, 0x0c, 0xcc, 0x94, 0x31,
        ];
        // Second server response (obfuscated): 3x rat\n
        let resp2 = [0xf2, 0xd0, 0x26, 0xc8, 0xa4, 0xd8, 0x7e];

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let (mut client, server) = tokio::io::duplex(128);

            tokio::spawn(async move {
                match handle_client(server).await {
                    Ok(_) => (),
                    Err(str) => println!("{}", str),
                }
            });

            // Write cipher and first request
            client.write_all(&cipher).await.unwrap();
            client.write_all(&req1).await.unwrap();
            client.flush().await.unwrap();

            // Write second request
            client.write_all(&req2).await.unwrap();
            client.flush().await.unwrap();

            // Client: read first response
            let mut buf = [0u8; 7];
            client.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, &resp1);

            // Client: read second response
            let mut buf2 = [0u8; 7];
            client.read_exact(&mut buf2).await.unwrap();
            assert_eq!(&buf2, &resp2);
        });
    }
}
