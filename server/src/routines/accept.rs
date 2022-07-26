use std::io::Read;
use std::process::exit;

use ipnet::Ipv4Net;
use log::{warn, info};
use paris::{error, success};
use rand::Rng;
use serde::Deserialize;
use ssh_key::PublicKey;
use std::io::{self, BufRead};

use crate::database::{Database, DatabaseClient};

pub struct Accept<'a> {
    database: &'a mut Database,
}

#[derive(Debug, Deserialize)]
struct InvitationRequest {
    version: u16,
    wireguard_pub: String,
    ssh_pub: String,
}

impl<'a> Accept<'a> {
    pub fn new(database: &'a mut Database) -> Self {
        Self { database }
    }

    /** Used when user does not specify any client name. Returns a random name from a fictional character.
       Maybe you get one or more references :winky_face:
    */
    fn random_name(&self) -> String {
        let names: [&str; 35] = [
            "Gary Goodspeed",
            "Quinn Ergon",
            "Little Cato",
            "Lord Commander",
            "Avocato",
            "HUE",
            "Elliot Alderson",
            "Tyrell Wellick",
            "Phillip Price",
            "Dominique DiPierro",
            "Angela Moss",
            "Mr. Robot",
            "Whiterose",
            "Mobley",
            "Cisco",
            "Gideon Goddard",
            "Deon Wilson",
            "Vincent Moore",
            "Chappie",
            "Rick Sanchez",
            "Summer Smith",
            "Morty Smith",
            "Jerry Smith",
            "Beth Smith",
            "Mr. Goldenfold",
            "Squanchy",
            "Gear Head",
            "Birdperson",
            "Jessica",
            "Ben Tennyson",
            "Gwen Tennyson",
            "Geralt of Rivia",
            "Yennefer",
            "Ciri",
            "Ori",
        ];

        names[rand::thread_rng().gen_range(0..names.len() - 1)].to_string()
    }

    pub fn execute(&mut self) {
        let user_subnet = match std::env::var("USER_SUBNET") {
            Ok(subnet) => subnet,
            Err(_) => {
                info!("No USER_SUBNET provided. Fallback to 192.168.10.0/24");
                String::from("192.168.10.0/24")
            },
        };

        let args: Vec<String> = std::env::args().collect();
        let client_name = match args.get(2) {
            Some(cn) => cn.to_owned(),
            None => self.random_name(),
        };

        let mut contents: Vec<u8> = Vec::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        match handle.read_to_end(&mut contents) {
            Ok(_) => {}
            Err(error) => {
                error!("Couldn't read from stdin: {}", error);
                exit(1);
            }
        }

        let unparsed = std::str::from_utf8(&contents).unwrap();

        let parsed = match serde_json::from_str::<InvitationRequest>(&unparsed) {
            Ok(o) => o,
            Err(error) => {
                println!("JSON: {}", &unparsed);
                error!("Invalid json syntax: {}", error);
                std::process::exit(1);
            }
        };

        if parsed.version > 1 {
            error!("User info has been created using a newer format.");
            exit(1);
        }

        let net: Ipv4Net = match user_subnet.parse() {
            Ok(s) => s,
            Err(error) => {
                error!("Your custom user subnet {} is invalid. Consider choosing another one: {}", user_subnet, error);
                return;
            }
        };
        let mut ipv4_address: Option<String> = None;

        let ip_space = net.hosts().rev();

        for available_ip in ip_space {
            let mut available = true;
            for client in self.database.get_clients() {
                if client.ipv4_address == available_ip.to_string() {
                    available = false;
                    break;
                }
            }

            if available {
                ipv4_address = Some(available_ip.to_string());
            }
        }

        if ipv4_address.is_none() {
            error!(
                "No more space in network {}! Cannot add any new client. Consider remove inactive clients using \"mcsync-server remove [CLIENT_NAME]\"",
                &user_subnet
            );
            return;
        }

        // Validate public key
        let pub_key = match PublicKey::from_openssh(format!("ssh-ed25519 {}", parsed.ssh_pub).as_str()) {
            Ok(pk) => pk,
            Err(error) => {
                error!("Client info contains an invalid ED25519 key: {}", error);
                return;
            }
        };

        success!("Valid ED25519 key {}: {}", pub_key.comment(), pub_key.fingerprint(Default::default()).to_string());

        let client = DatabaseClient {
            name: client_name.clone(),
            ipv4_address: ipv4_address.unwrap(),
            last_seen: 0,
            wg_public_key: parsed.wireguard_pub,
            ssh_public_key: parsed.ssh_pub,
        };

        self.database.new_client(client.clone());

        success!(
            "Added new client {} ({}).",
            client_name,
            client.ipv4_address
        );
    }
}
