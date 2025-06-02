mod cipher;

use cipher::Cipher;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

pub struct InsecureSocketLayer<S> {
    reader: BufReader<S>,
    cipher: Cipher,
    client_pos: usize,
    server_pos: usize,
}

impl<S> InsecureSocketLayer<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn new(stream: S) -> Result<InsecureSocketLayer<S>, String> {
        let mut reader = BufReader::new(stream);
        let cipher = Cipher::new(&mut reader).await?;

        if cipher.is_no_op() {
            return Err(String::from("Cipher is no op"));
        }

        Ok(InsecureSocketLayer {
            reader,
            cipher,
            client_pos: 0,
            server_pos: 0,
        })
    }

    pub async fn read(&mut self) -> Result<String, String> {
        let mut decoded_request = String::new();

        loop {
            let byte = self
                .reader
                .read_u8()
                .await
                .map_err(|_| "Failed to read request byte")?;

            let decoded_byte = self.cipher.decode(byte, self.client_pos);
            decoded_request.push(decoded_byte as char);

            self.client_pos += 1;

            if decoded_byte as char == '\n' {
                break;
            }
        }

        Ok(decoded_request)
    }

    pub async fn write(&mut self, response: String) -> Result<(), String> {
        let mut writer = BufWriter::new(self.reader.get_mut());

        for byte in response.as_bytes() {
            let encoded_byte = self.cipher.encode(*byte, self.server_pos);

            writer
                .write_u8(encoded_byte)
                .await
                .map_err(|_| "Failed to write encoded byte")?;

            self.server_pos += 1;
        }

        writer.flush().await.map_err(|_| "Failed to flush writer")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::get_most_toys;

    use super::*;
    use tokio::io::{AsyncWriteExt, DuplexStream};
    use tokio::runtime::Runtime;

    // Helper to write cipher spec and message to a duplex stream
    async fn write_cipher_and_message(stream: &mut DuplexStream, cipher: &[u8], msg: &[u8]) {
        stream.write_all(cipher).await.unwrap();
        stream.write_all(msg).await.unwrap();
        stream.flush().await.unwrap();
    }

    #[test]
    fn test_xor1_reversebits_hello() {
        let cipher = [0x02, 0x01, 0x01, 0x00];
        let msg = [0x96, 0x26, 0xb6, 0xb6, 0x76, 0xd0];
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let (mut client, server) = tokio::io::duplex(64);
            write_cipher_and_message(&mut client, &cipher, &msg).await;
            let mut isl = InsecureSocketLayer::new(server).await.unwrap();
            let decoded = isl.read().await.unwrap();
            assert_eq!(decoded, "hello\n");
        });
    }

    #[test]
    fn test_addpos_addpos_hello() {
        let cipher = [0x05, 0x05, 0x00];
        let msg = [0x68, 0x67, 0x70, 0x72, 0x77, 0x14];
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let (mut client, server) = tokio::io::duplex(64);
            write_cipher_and_message(&mut client, &cipher, &msg).await;
            let mut isl = InsecureSocketLayer::new(server).await.unwrap();
            let decoded = isl.read().await.unwrap();
            assert_eq!(decoded, "hello\n");
        });
    }

    #[test]
    fn test_example_session_xor123_addpos_reversebits() {
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

            // Write cipher and first request
            client.write_all(&cipher).await.unwrap();
            client.write_all(&req1).await.unwrap();
            client.flush().await.unwrap();

            // Server: read first request
            let mut isl = InsecureSocketLayer::new(server).await.unwrap();
            let decoded1 = isl.read().await.unwrap();
            assert_eq!(decoded1, "4x dog,5x car\n");

            // Server: write first response
            let response1 = get_most_toys(decoded1).unwrap();
            assert_eq!(response1, "5x car\n");
            isl.write(response1).await.unwrap();

            // Client: read first response
            let mut buf = [0u8; 7];
            client.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, &resp1);

            // Write second request
            client.write_all(&req2).await.unwrap();
            client.flush().await.unwrap();

            // Server: read second request
            let decoded2 = isl.read().await.unwrap();
            assert_eq!(decoded2, "3x rat,2x cat\n");

            // Server: write second response
            let response2 = get_most_toys(decoded2).unwrap();
            assert_eq!(response2, "3x rat\n");
            isl.write(response2).await.unwrap();

            // Client: read second response
            let mut buf2 = [0u8; 7];
            client.read_exact(&mut buf2).await.unwrap();
            assert_eq!(&buf2, &resp2);
        });
    }
}
