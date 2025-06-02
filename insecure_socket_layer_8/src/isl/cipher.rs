use std::ops::BitXor;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

enum Operation {
    ReverseBits,
    Xor(u8),
    Xorpos,
    Add(u8),
    Addpos,
}

impl Operation {
    fn encode(&self, byte: u8, pos: usize) -> u8 {
        match self {
            Operation::ReverseBits => byte.reverse_bits(),
            Operation::Xor(n) => byte.bitxor(n),
            Operation::Xorpos => {
                let res = (byte as usize ^ pos) % 256;
                res as u8
            }
            Operation::Add(n) => byte.wrapping_add(*n),
            Operation::Addpos => {
                let res = ((byte as usize).wrapping_add(pos)) % 256;
                res as u8
            }
        }
    }

    fn decode(&self, byte: u8, pos: usize) -> u8 {
        match self {
            Operation::ReverseBits => byte.reverse_bits(),
            Operation::Xor(n) => byte.bitxor(n),
            Operation::Xorpos => {
                let res = (byte as usize ^ pos) % 256;
                res as u8
            }
            Operation::Add(n) => byte.wrapping_sub(*n),
            Operation::Addpos => {
                let res = ((byte as usize).wrapping_sub(pos)) % 256;
                res as u8
            }
        }
    }
}

pub struct Cipher {
    operations: Vec<Operation>,
}

async fn read_cipher_byte<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Result<u8, String> {
    reader
        .read_u8()
        .await
        .map_err(|_| String::from("Failed reading cipher byte"))
}

impl Cipher {
    pub async fn new<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Result<Cipher, String> {
        let mut operations = Vec::new();

        loop {
            let op = read_cipher_byte(reader).await?;

            if op == 0x00 {
                break;
            }

            let op = match op {
                0x01 => Operation::ReverseBits,
                0x02 => {
                    let n = read_cipher_byte(reader).await?;
                    Operation::Xor(n)
                }
                0x03 => Operation::Xorpos,
                0x04 => {
                    let n = read_cipher_byte(reader).await?;
                    Operation::Add(n)
                }
                0x05 => Operation::Addpos,
                _ => return Err(String::from("Invalid cipher operation")),
            };

            operations.push(op);
        }

        if operations.is_empty() {
            Err(String::from("Empty cipher"))
        } else {
            Ok(Cipher { operations })
        }
    }

    pub fn is_no_op(&self) -> bool {
        let h = 'h';
        self.encode(h as u8, 0) == h as u8
    }

    pub fn decode(&self, mut byte: u8, pos: usize) -> u8 {
        for op in self.operations.iter().rev() {
            byte = op.decode(byte, pos);
        }

        byte
    }

    pub fn encode(&self, mut byte: u8, pos: usize) -> u8 {
        for op in self.operations.iter() {
            byte = op.encode(byte, pos);
        }

        byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_cipher() {
        let ops = vec![Operation::Xor(1), Operation::ReverseBits];
        let cipher = Cipher { operations: ops };

        let input: Vec<u8> = vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]; // hello
        let expected_output: Vec<u8> = vec![0x96, 0x26, 0xb6, 0xb6, 0x76];
        let mut output: Vec<u8> = Vec::new();

        for (pos, byte) in input.iter().enumerate() {
            let encoded_byte = cipher.encode(*byte, pos);
            output.push(encoded_byte);
        }

        assert_eq!(output, expected_output);

        output.clear();
        for (pos, byte) in expected_output.iter().enumerate() {
            let decoded_byte = cipher.decode(*byte, pos);
            output.push(decoded_byte);
        }

        assert_eq!(output, input);
    }

    #[test]
    fn basic_cipher2() {
        let ops = vec![Operation::Addpos, Operation::Addpos];
        let cipher = Cipher { operations: ops };

        let input: Vec<u8> = vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]; // hello
        let expected_output: Vec<u8> = vec![0x68, 0x67, 0x70, 0x72, 0x77];
        let mut output: Vec<u8> = Vec::new();

        for (pos, byte) in input.iter().enumerate() {
            let encoded_byte = cipher.encode(*byte, pos);
            output.push(encoded_byte);
        }

        assert_eq!(output, expected_output);

        output.clear();
        for (pos, byte) in expected_output.iter().enumerate() {
            let decoded_byte = cipher.decode(*byte, pos);
            output.push(decoded_byte);
        }

        assert_eq!(output, input);
    }
}
