use ring::digest::{Context, Digest, SHA256};
use std::io::{Read, self};

// Adapted from https://rust-lang-nursery.github.io/rust-cookbook/cryptography/hashing.html
pub fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest, io::Error> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        match reader.read(&mut buffer) {
            Ok(count) => {
                if count == 0 {
                    break;
                }
                context.update(&buffer[..count]);
            }
            Err(error) => {
                return Err(error);
            }
        }
    }

    Ok(context.finish())
}