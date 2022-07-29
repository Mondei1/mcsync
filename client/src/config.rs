use std::{
    fs::{create_dir_all, File, self},
    process::exit, io::Write, os::unix::prelude::OpenOptionsExt,
};

use camino::Utf8PathBuf;
use paris::{error, warn};

use serde::{Deserialize, Serialize};
use ssh_key::{private::Ed25519Keypair, rand_core::OsRng, PrivateKey};

pub struct Config {
    config_path: Utf8PathBuf,
    data: ClientConfig,
    fingerprint: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub(crate) version: u16,
    pub(crate) keys: ClientKeys,
    pub(crate) server: Vec<ClientServer>,
    pub(crate) sync: Vec<ClientSync>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientKeys {
    pub(crate) wg: String,
    pub(crate) ssh: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientServer {
    id: String,
    name: String,
    endpoint: String,
    public_key: String,
    user_subnet: String,
    tool_subnet: String,
    ipv4_address: String,
    dns: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientSync {
    id: String,
    server: String,
    name: String,
    location: String,
    share: bool
}

impl Config {
    pub fn new(config_path: Utf8PathBuf) -> Self {
        let mut data: ClientConfig;

        if config_path.exists() {
            data = match File::open(&config_path) {
                Ok(file) => {
                    match serde_json::from_reader(file) {
                        Ok(json) => {
                            json
                        },
                        Err(error) => {
                            error!("Config file has syntax errors: {}", error);
                            exit(1);
                        }
                    }
                },
                Err(error) => {
                    error!("Couldn't open config file at {}: {}", &config_path, error);
                    exit(1);
                }
            }
        } else {
            // Remove file from path.
            let mut path_clone = config_path.clone();
            path_clone.pop();

            // If parent directory doesn't exist, try to create the path.
            if !path_clone.exists() {
                match create_dir_all(&path_clone) {
                    Ok(_) => {}
                    Err(error) => {
                        error!(
                            "Failed to recursivly create all directories to {}: {}",
                            path_clone.to_string(),
                            error
                        );
                        exit(1);
                    }
                }
            }

            // Create default config
            let file_options = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .mode(0o660)
                .open(&config_path);
            data = match file_options {
                Ok(mut file) => {
                    // We now need to generate both WireGuard & SSH keys.
                    let wg = wireguard_keys::Privkey::generate();
                    let ssh_keypair = Ed25519Keypair::random(&mut OsRng);
                    let ssh = PrivateKey::try_from(ssh_keypair).unwrap();

                    let new_config = ClientConfig {
                        version: 1,
                        keys: ClientKeys {
                            ssh: ssh.to_openssh(ssh_key::LineEnding::LF).unwrap().to_string(),
                            wg: wg.to_base64(),
                        },
                        server: vec![],
                        sync: vec![]
                    };

                    match serde_json::to_string_pretty(&new_config) {
                        Ok(json) => {
                            match file.write_all(json.as_bytes()) {
                                Ok(_) => { }
                                Err(error) => {
                                    error!("Couldn't write new config at {}: {}", config_path, error);
                                    exit(1);
                                }
                            }
                        },
                        Err(error) => {
                            error!("Couldn't create JSON string: {}", error);
                            exit(1);
                        }
                    }

                    new_config
                }
                Err(error) => {
                    error!("Couldn't create config file at {}: {}", &config_path, error);
                    exit(1);
                }
            };
        }

        if !config_path.is_file() {
            error!("Selected config file is actually not a file.");
            exit(1);
        }

        Self { config_path, data, fingerprint: String::new() }
    }

    pub fn verify_integrity(&self) {
        // 1. Check if all synced server point to a valid server.
        // 2. Make sure those endpoints are reachable.
        for sync in &self.data.sync {
            if self.data.server.iter().find(|x| x.id == sync.server).is_none() {
                warn!("Sync with ID {} points to an undefined server.", sync.server);
            }
        }
    }

    pub fn flush(&mut self) -> Option<()> {
        match serde_json::to_string_pretty(&self.data) {
            Ok(pretty) => {
                let new_fingerprint = sha256::digest(&pretty);
                if new_fingerprint == self.fingerprint {
                    return None;
                }

                self.fingerprint = new_fingerprint;

                match File::create(&self.config_path) {
                    Ok(mut file) => {
                        match file.write_all(pretty.as_bytes()) {
                            Ok(_) => {
                                match file.flush() {
                                    Ok(_) => {
                                        Some(())
                                    },
                                    Err(error) => {
                                        error!("Error on flushing buffer to file: {}", error);
                                        return None;
                                    }
                                }
                                
                            },
                            Err(error) => {
                                error!("Error on writing buffer to file: {}", error);
                                return None;
                            }
                        }
                    }

                    Err(error) => {
                        error!("Config file ({}) couldn't be opened: {}", &self.config_path, error);
                        None
                    }
                }
            },
            Err(error) => {
                error!("Couldn't create JSON string: {}", error);
                None

            }
        };

        None
    }
}
