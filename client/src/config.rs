use std::{
    fs::{self, create_dir_all, File},
    io::{Write, BufReader},
    os::unix::prelude::OpenOptionsExt,
    process::exit,
};

use camino::Utf8PathBuf;
use data_encoding::HEXUPPER;
use paris::{error, warn};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::hash::sha256_digest;

pub const CONFIG_VERSION: u16 = 1;
pub const SERVERINFO_VERSION: u16 = 1;

#[derive(Clone)]
pub struct Config {
    config_path: Utf8PathBuf,
    data: ClientConfig,
    fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub(crate) version: u16,
    pub(crate) keys: ClientKeys,
    pub(crate) server: Vec<ClientServer>,
    pub(crate) sync: Vec<ClientSync>,
    pub(crate) current: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientKeys {
    pub(crate) wg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientServer {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) endpoint: String,
    pub(crate) public_key: String,
    pub(crate) psk: String,
    pub(crate) user_subnet: String,
    pub(crate) tool_subnet: String,
    pub(crate) ipv4_address: String,
    pub(crate) dns: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSync {
    id: String,
    server: String,
    name: String,
    location: String,
    start: String,
    share: bool,
}

// Copied over from server. Maybe I use a shared project or something.
#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub(crate) version: u16,
    pub(crate) endpoint: String,
    pub(crate) public_key: String,
    pub(crate) psk: String,
    pub(crate) tool_subnet: String,
    pub(crate) user_subnet: String,
    pub(crate) ipv4_address: String,
    pub(crate) dns: String,
}

impl Config {
    pub fn new(config_path: Utf8PathBuf) -> Self {
        let data: ClientConfig;

        if config_path.exists() {
            data = match File::open(&config_path) {
                Ok(file) => match serde_json::from_reader(file) {
                    Ok(json) => json,
                    Err(error) => {
                        error!("Config file has syntax errors: {}", error);
                        exit(1);
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
                    // We now need to generate both WireGuard keys.
                    let wg = wireguard_keys::Privkey::generate();

                    let new_config = ClientConfig {
                        version: 1,
                        keys: ClientKeys { wg: wg.to_base64() },
                        server: vec![],
                        sync: vec![],
                        current: None
                    };

                    match serde_json::to_string_pretty(&new_config) {
                        Ok(json) => match file.write_all(json.as_bytes()) {
                            Ok(_) => {}
                            Err(error) => {
                                error!("Couldn't write new config at {}: {}", config_path, error);
                                exit(1);
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

        Self {
            config_path,
            data,
            fingerprint: String::new(),
        }
    }

    pub fn verify_integrity(&mut self) {
        let mut faulty = false;

        if self.data.version > CONFIG_VERSION {
            warn!("Config file has been created in a newer version of mcsync. Current version = {}, config version = {}", CONFIG_VERSION, self.data.version);
            faulty = true;
        }

        // 1. Check if all synced server point to a valid server.
        // 2. Make sure those endpoints are reachable.
        for sync in &self.data.sync {
            if self
                .data
                .server
                .iter()
                .find(|x| x.id == sync.server)
                .is_none()
            {
                faulty = true;
                warn!(
                    "Sync with ID {} points to an undefined server.",
                    sync.server
                );
            }
        }

        if self.data.current.is_some() {
            if self.get_server_by_id(&self.data.current.as_ref().unwrap()).is_none() {
                warn!("Field \"current\" in config points to an non-existing server. Remove flag.");
                self.data.current = None;
            }
        }

        if faulty {
            error!("There are some logical errors within your config file ({}), see previous warnings. That might be due to manual changes made by you. Please resolve them and try again.", self.config_path);
            exit(1);
        }
    }

    pub fn get_public_wireguard_key(&self) -> wireguard_keys::Pubkey {
        match wireguard_keys::Privkey::from_base64(&self.data.keys.wg) {
            Ok(private_key) => private_key.pubkey(),
            Err(_) => {
                error!("Private WireGuard key is corrupt. Have you touched it? You may have locked yourself out by touching it.");
                exit(1);
            }
        }
    }

    pub fn get_data(&self) -> &ClientConfig {
        &self.data
    }

    pub fn set_current_server(&mut self, server_id: &str) {
        self.data.current = Some(server_id.to_string());

        self.flush();
    }

    pub fn get_current_server(&self) -> Option<ClientServer> {
        if self.data.current.is_none() {
            return None;
        }

        self.get_server_by_id(&self.data.current.as_ref().unwrap())
    }

    pub fn add_server(&mut self, server_name: String, server: ServerInfo) -> Option<String> {
        if server.version > SERVERINFO_VERSION {
            error!("This server info file has been created with a newer version of mcsync. Refuse to load.");
            exit(1);
        }

        let uuid = Uuid::new_v4().to_string();

        let duplicate_name = self.data.server.iter().find(|x| x.name == server_name);
        if duplicate_name.is_some() {
            error!("A server with the name \"{}\" already exists. Please choose another name.", server_name);
            return None;
        }

        let duplicate_pk = self.data.server.iter().find(|x| x.public_key == server.public_key);
        if duplicate_pk.is_some() {
            warn!("Already existing server \"{}\" has the same public key as this server.", duplicate_pk.unwrap().name);
        }

        self.data.server.push(ClientServer {
            id: uuid.clone(),
            name: server_name,
            endpoint: server.endpoint,
            public_key: server.public_key,
            psk: server.psk,
            user_subnet: server.user_subnet,
            tool_subnet: server.tool_subnet,
            ipv4_address: server.ipv4_address,
            dns: server.dns,
        });

        match self.flush() {
            Some(_) => {
                Some(uuid)
            }
            None => {
                error!("Couldn't add new server (see previous errors)");
                None
            }
        }
    }

    pub fn get_server_by_name(&self, server_name: &str) -> Option<ClientServer> {
        self.data.clone().server.into_iter()
            .find(|x| x.name == server_name)
    }

    pub fn get_server_by_id(&self, server_id: &str) -> Option<ClientServer> {
        self.data.clone().server.into_iter()
            .find(|x| x.id == server_id)
    }

    pub fn add_sync(&self, sync_name: &str, mut start: Utf8PathBuf) -> Option<String> {
        if self.get_sync_by_name(sync_name).is_some() {
            return None;
        }

        if !start.exists() {
            error!("Start file doesn't exists. It has to be a script (.sh and .bat) or a JAR-file.");
            return None;
        }

        if !start.is_file() {
            error!("Selected path points to a non-file. Maybe a folder or link?");
            exit(1);
        }

        let final_path = match start.extension() {
            Some(ex) => {
                if ex != "jar" {
                    // Try again but without extension
                    return self.add_sync(sync_name, start.with_extension(""));
                }

                // File points to a JAR-file.
                start
            }
            // File has no extension. Try to figure out if both versions exist.
            None => {
                let filename = start.clone().file_name().unwrap().to_string();

                let mut unix = false;
                let mut windows = false;

                start.pop();
                start.push(format!("{}.sh", filename));
                if start.exists() {
                    unix = true;
                }

                start.pop();
                start.push(format!("{}.bat", filename));

                if start.exists() {
                    windows = true;
                }

                if !unix || !windows {
                    error!("There has to be a {}.sh (for Unix) and {}.bat (for Windows) file. This is required because some users might host using Windows or Linux/macOS.", filename, filename);
                    return None;
                }

                start.with_extension("")
            }
        };

        if !final_path.exists() {
            error!("Start file doesn't exist.");
            return None;
        }

        let new_id = Uuid::new_v4();

        return None;
    }
    
    pub fn get_sync_by_name(&self, sync_name: &str) -> Option<ClientSync> {
        self.data.clone().sync.into_iter()
            .find(|x| x.name == sync_name)
    }

    pub fn flush(&mut self) -> Option<()> {
        match serde_json::to_string_pretty(&self.data) {
            Ok(pretty) => {
                let hasher = match sha256_digest(BufReader::new(pretty.as_bytes())) {
                    Ok(s) => s,
                    Err(error) => {
                        error!("Error: Cannot determine if config has changed: {}", error);
                        exit(1);
                    }
                };

                let new_fingerprint = HEXUPPER.encode(hasher.as_ref());
                if new_fingerprint == self.fingerprint {
                    warn!("No change was made.");
                    return None;
                }

                self.fingerprint = new_fingerprint;

                match File::create(&self.config_path) {
                    Ok(mut file) => match file.write_all(pretty.as_bytes()) {
                        Ok(_) => match file.flush() {
                            Ok(_) => { return Some(()); }
                            Err(error) => {
                                error!("Error on flushing buffer to file: {}", error);
                                return None;
                            }
                        },
                        Err(error) => {
                            error!("Error on writing buffer to file: {}", error);
                            return None;
                        }
                    },

                    Err(error) => {
                        error!(
                            "Config file ({}) couldn't be opened: {}",
                            &self.config_path, error
                        );
                        return None;
                    }
                }
            }
            Err(error) => {
                error!("Couldn't create JSON string: {}", error);
                return None;
            }
        };
    }
}
