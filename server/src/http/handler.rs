use ssh_key::{
    private::{KeypairData, PrivateKey, RsaKeypair, Ed25519Keypair},
    rand_core::OsRng, PublicKey,
};

use std::env;
use nickel::Nickel;
use paris::Logger;

pub struct HttpHandler {
    server: Nickel
}

impl HttpHandler {
    pub fn new() -> Self {
        let mut server = Nickel::new();
        
        server.utilize(middleware! { |request|
            let mut logger = Logger::new();
            logger.info(format!("{} -- {} {}", request.origin.remote_addr.ip().to_string(), request.origin.method, request.origin.uri));
        });

        server.utilize(router! {
            get "/ssh-access" => | req, res | {
                let key = Ed25519Keypair::random(&mut OsRng);
                let key_clone = key.clone();
                let keypair = KeypairData::from(key);

                let private_key = PrivateKey::try_from(keypair).unwrap();
                let public_key = PublicKey::try_from(key_clone.public).unwrap();

                let result = private_key.to_openssh(ssh_key::LineEnding::LF).unwrap().to_string();

                println!("{}", public_key.to_openssh().unwrap().to_string());

                result
            }

            get "**" => |req, res| {
                req.origin.remote_addr.ip().to_string()
            }
        });

        Self { server }
    }

    pub fn listen(self) {
        self.server.listen("127.0.0.1:8080").unwrap();
    }
}