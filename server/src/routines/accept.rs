use std::{io::Read};
use std::process::exit;

use domain::utils::base64;
use ipnet::Ipv4Net;
use paris::{error};
use rand::{RngCore, Rng};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::io::{self};

use crate::database::{Database, DatabaseClient};
use crate::docker::DockerManager;
use crate::env;
use crate::wireguard::Wireguard;

pub struct Accept<'a> {
    database: &'a mut Database,
    wireguard: &'a mut Wireguard,
    docker: &'a DockerManager
}

#[derive(Debug, Deserialize)]
struct ClientInfo {
    version: u16,
    wireguard_pub: String
}

#[derive(Debug, Serialize)]
struct ServerInfo {
    version: u16,
    endpoint: String,
    public_key: String,
    psk: String,
    tool_subnet: String,
    user_subnet: String,
    ipv4_address: String,
    dns: String
}

impl<'a> Accept<'a> {
    pub fn new(database: &'a mut Database, wireguard: &'a mut Wireguard, docker: &'a DockerManager) -> Self {
        Self { database, wireguard, docker }
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

    pub async fn execute(&mut self) {
        let endpoint: String = match std::env::var("ENDPOINT") {
            Ok(endpoint) => {
                // We just pretent it's a http URL.
                match Url::parse(&format!("http://{}", endpoint)) {
                    Ok(result) => {
                        if result.port().is_none() {
                            error!("You need to set a port in ENDPOINT. If you're not sure use 51820 as it's WireGuard's default port.");
                            exit(1);
                        }

                        endpoint
                    },
                    Err(error) => {
                        error!("Environment variable contains invalid address: {}", error);
                        exit(1);
                    }
                }
            },
            Err(_) => {
                error!("You need to set the environment variable ENDPOINT which is an address ( [Domain OR IPv4]:PORT ) under which WireGuard is reachable by others.");
                exit(1);
            }
        };

        let dns_ip = match self.docker.get_dns_container().await {
            Some(dns) => {
                self.docker
                    .get_container_ip(dns)
                    .await
                    .unwrap()
            },
            None => {
                error!("Cannot find DNS container. Did you rename your containers? The name has to contain \"dns\" somewhere e.g. \"mcsync-dns-1\".");
                exit(1);
            }
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

        let parsed = match serde_json::from_str::<ClientInfo>(&unparsed) {
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

        let net: Ipv4Net = env::get_user_subnet();
        let mut ipv4: Option<String> = None;

        let ip_space = net.hosts();

        // Loop over available IP space to find a free address.
        // This is like a mini DHCP, without DHCP protocol.

        // .skip(1): This skips the first usable address since that one is reserved for the VPN itself.
        for available_ip in ip_space.skip(1) {
            let mut available = true;
            for client in self.database.get_clients() {
                if client.ipv4_address == available_ip.to_string() {
                    available = false;
                    break;
                }
            }

            if available {
                ipv4 = Some(available_ip.to_string());
                break;
            }
        }

        if ipv4.is_none() {
            error!(
                "No more space in network {}! Cannot add any new client. Consider remove inactive clients using \"mcsync-server remove [CLIENT_NAME]\"",
                &net.to_string()
            );
            return;
        }

        // Generate random PSK
        let mut rng = rand::thread_rng();
        let mut psk: [u8; 32] = [0; 32];

        // Fill psk array with random bytes.
        rng.fill_bytes(&mut psk);

        let psk_base64 = base64::encode_string(&psk);
        let address = ipv4.unwrap();

        let client = DatabaseClient {
            name: client_name.clone(),
            ipv4_address: address.clone(),
            last_seen: 0,
            wg_public_key: parsed.wireguard_pub.clone(),
            wg_psk: psk_base64.clone()
        };

        self.database.new_client(client.clone());
        self.wireguard.add_peer(parsed.wireguard_pub, psk_base64, address.clone());

        let public_key = self.database.get_wireguard_private_key().pubkey().to_base64();

        let tool_subnet = match self.docker.get_network().await {
            Ok(info) => {
                // Like this is really dangerous. There are a lot of .unwrap's but you only live once, right?
                info.ipam.unwrap().config.unwrap().get(0).unwrap().subnet.clone().unwrap()
            },
            Err(error) => {
                error!("Cannot find Docker network \"mcsync\": {}", error);
                exit(1);
            }
        };

        let server_info = ServerInfo {
            version: 1,
            endpoint,
            public_key,
            psk: client.wg_psk,
            ipv4_address: address,
            user_subnet: net.to_string(),
            tool_subnet,
            dns: dns_ip
        };

        match self.docker.get_vpn_container().await {
            Some(vpn) => {
                let _ = self.docker.restart_container(vpn);
            },
            None => {
                error!("Cannot find WireGuard container. Is it stopped?");
                exit(1);
            }
        }

        println!("{}", serde_json::to_string_pretty(&server_info).unwrap());
    }
}
