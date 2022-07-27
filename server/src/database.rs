use std::{fs::File, io::{Read, Error, Write}, time::{SystemTime, UNIX_EPOCH}, borrow::BorrowMut, vec};

use paris::{info, error, success, warn};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Database {
    path: String,

    /// Stores the fingerprint of the latest written database to prevent flushing
    /// the same contents all the time.
    fingerprint: String,
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

                let size = read_operation.unwrap();
                success!("Read database ({} bytes)", size);

                // Empty file
                if size == 0 {
                    DatabaseFormat {
                        version: 1,
                        client: vec![],
                        synced: vec![]
                    }
                } else {
                    let parsed = match serde_json::from_str::<DatabaseFormat>(&contents) {
                        Ok(o) => o,
                        Err(error) => {
                            error!("Invalid json syntax: {}", error);
                            std::process::exit(1);
                        }
                    };
                    
                    parsed
                }
                
            },
            Err(error) => {
                error!("Couldn't open file: {}", error);
                std::process::exit(1);
            }
        };

        Self { path, data, fingerprint: String::new() }
    }

    pub fn new_client(&mut self, client: DatabaseClient) {
        self.data.client.push(client);
    }

    pub fn remove_client(&mut self, name: &str) -> Option<DatabaseClient> {
        let position = self.data.client.iter().position(|c| c.name == name);
        let client = self.get_client_by_name(name);

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
        match self.data.client.iter().position(|c| c.ipv4_address == ip) {
            Some(pos) => {
                let mut changed_user = self.get_clients().get(pos).unwrap().clone();
                changed_user.last_seen = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

                let _ = std::mem::replace(&mut self.data.client[pos], changed_user);
            },
            None => {
                warn!("Unknown client has been seen! IP: {}", ip);
            }
        }
    } 

    pub fn get_clients(&self) -> Vec<DatabaseClient> {
        self.data.client.clone()
    }

    pub fn get_client_by_name(&self, name: &str) -> Option<&DatabaseClient> {
        match self.data.client.iter().find(|c| c.name == name).to_owned() {
            Some(value) => {
                Some(value)
            },
            None => { None }
        }
    }

    pub fn get_client_by_ip(&self, ip: &str) -> Option<&DatabaseClient> {
        match self.data.client.iter().find(|c| c.ipv4_address == ip).to_owned() {
            Some(value) => {
                Some(value)
            },
            None => { None }
        }
    }

    pub fn get_data(&self) -> &DatabaseFormat {
        &self.data
    }

    /// Writes current state of database to disk. Only if config changed since last write.
    pub fn flush(&mut self) -> Option<()> {
        match serde_json::to_string_pretty(&self.data) {
            Ok(pretty) => {
                let new_fingerprint = sha256::digest(&pretty);
                if new_fingerprint == self.fingerprint {
                    return None;
                }

                self.fingerprint = new_fingerprint;

                match File::create(&self.path) {
                    Ok(mut file) => {
                        match file.write_all(pretty.as_bytes()) {
                            Ok(_) => {
                                match file.flush() {
                                    Ok(_) => {
                                        info!("Database has been written to disk!");
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
                        error!("Database file ({}) couldn't be opened: {}", self.path, error);
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