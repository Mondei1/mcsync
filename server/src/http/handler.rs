use paris::info;

use ssh_key::{
    private::{Ed25519Keypair, KeypairData, PrivateKey},
    rand_core::OsRng,
    PublicKey,
};

use nickel::Nickel;

use crate::database::Database;

pub struct HttpHandler {
    server: Nickel,
    database: Database,
}

impl HttpHandler {
    pub fn new(database: Database) -> Self {
        let mut server = Nickel::new();
        let db = database.clone();

        server.utilize(middleware! { |request|
            let mut db_clone = db.clone();

            let ip = request.origin.remote_addr.ip().to_string();
            db_clone.seen_client(ip.as_str());
            db_clone.flush();

            let client = db_clone.get_client_by_ip(&ip);

            info!("{} ({}) -- {} {}", 
                ip,
                if client.is_some() { client.unwrap().name.clone() } else { String::from("") },
                request.origin.method,
                request.origin.uri
            );
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
        self.server.listen("0.0.0.0:8080").unwrap();
    }
}
