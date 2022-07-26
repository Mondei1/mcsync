use std::{fs::File, io::{Read, Error, Write}, time::{SystemTime, UNIX_EPOCH}};

use log::warn;
use paris::{info, error, success};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Database {
    path: String,
    data: DatabaseFormat
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DatabaseFormat {
    version: u16,
    synced: Vec<DatabaseSynced>,
    client: Vec<DatabaseClient>
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DatabaseSynced {
    id: String,
    name: String,
    share: bool
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DatabaseClient {
    pub(crate) name: String,
    pub(crate) ipv4_address: String,
    pub(crate) last_seen: u64,
    pub(crate) wg_public_key: String,
    pub(crate) ssh_public_key: String
}

/// It's not a real database. We just dump everything into a .json file to remember various things.
/// A full-featured database is a bit overpowered. If things get too complicated I may migrate to SQLite.
/// 
/// This implementation shouldn't be very slow since we work in-memory and flush on exit.
impl Database {
    pub fn new() -> Self {
        let path = match std::env::var("DATABASE_PATH") {
            Ok(file_path) => {
                file_path
            },
            Err(_) => {
                info!(
                    "Fallback to /database.json because DATABASE_PATH has not been specified."
                );
                String::from("/database.json")
            }
        };

        let data = match File::open(&path) {
            Ok(mut data) => {
                let mut contents = String::new();
                let read_operation = data.read_to_string(&mut contents);

                if read_operation.is_err() {
                    error!("Couldn't read file: {}", read_operation.unwrap_err());
                    std::process::exit(1);
                }

                success!("Read database ({} bytes)", read_operation.unwrap());

                let parsed = match serde_json::from_str::<DatabaseFormat>(&contents) {
                    Ok(o) => o,
                    Err(error) => {
                        error!("Invalid json syntax: {}", error);
                        std::process::exit(1);
                    }
                };
                
                parsed
            },
            Err(error) => {
                error!("Couldn't open file: {}", error);
                std::process::exit(1);
            }
        };

        Self { path, data }
    }

    pub fn new_client(&mut self, client: DatabaseClient) {
        self.data.client.push(client);
    }

    pub fn remove_client(&mut self, name: &str) -> Option<DatabaseClient> {
        let position = self.data.client.iter().position(|c| c.name == name);
        let client = self.get_client(name);

        match position {
            Some(pos) => {
                let client_clone = client.unwrap().to_owned();
                self.data.client.remove(pos);

                Some(client_clone)
            },
            None => {
                None
            }
        }
    }

    /// Call once you have seen the client.
    pub fn seen_client(&mut self, ip: &str) -> () {
        match self.data.client.iter().find(|c| c.ipv4_address == ip).to_owned() {
            Some(mut client) => {
                client.to_owned().last_seen = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            },
            None => {
                warn!("Unknown client has been seen! IP: {}", ip);
            }
        }
    } 

    pub fn get_clients(&self) -> Vec<DatabaseClient> {
        self.data.client.clone()
    }

    pub fn get_client(&self, name: &str) -> Option<DatabaseClient> {
        match self.data.client.iter().find(|c| c.name == name).to_owned() {
            Some(value) => {
                Some(value.to_owned())
            },
            None => { None }
        }
    }

    /// Writes current state of database to disk.
    pub fn flush(&self) -> Option<()> {
        match File::create(&self.path) {
            Ok(mut file) => {
                match serde_json::to_string_pretty(&self.data) {
                    Ok(pretty) => {

                        match file.write_all(pretty.as_bytes()) {
                            Ok(_) => {
                                match file.flush() {
                                    Ok(_) => {
                                        success!("Database has been written to disk!");
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

                        Some(())
                    },
                    Err(error) => {
                        error!("Couldn't create JSON string: {}", error);
                        None
                    }
                }
            },
            Err(error) => {
                error!("Database file ({}) couldn't be opened: {}", self.path, error);
                None
            }
        }
    }
}