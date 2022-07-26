use log::info;
use ssh_key::{
    private::{KeypairData, PrivateKey, Ed25519Keypair},
    rand_core::OsRng, PublicKey,
};

use nickel::Nickel;

use crate::database::{Database};

pub struct HttpHandler<'a> {
    server: Nickel,
    database: &'a Database
}

impl<'a> HttpHandler<'a> {
    pub fn new(database: &'a mut Database) -> Self {
        let mut server = Nickel::new();
        let mut db = database.to_owned();
        
        server.utilize(middleware! { |request|
            let ip = request.origin.remote_addr.ip().to_string();
            // let result = (&db).seen_client(ip.as_str());
            info!("{} -- {} {}", ip, request.origin.method, request.origin.uri);
        });

        server.utilize(router! {
            get "/" => | req, res | {
                "ok"
            }

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
                "404 not found"
            }
        });

        Self { server, database }
    }

    pub fn listen(self) {
        self.server.listen("127.0.0.1:8080").unwrap();
    }
}