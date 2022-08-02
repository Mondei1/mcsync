use std::{fs::File, io::{Write}, time::Duration};

use camino::Utf8PathBuf;
use data_encoding::{HEXLOWER};
use paris::{error, warn, info};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;

use crate::{utils::{hash::sha256_digest}, config::{Config, ClientServer}};

pub const SYNC_VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
pub struct CreateServer {
    server_name: String
}

#[derive(Serialize, Deserialize)]
pub struct SyncFile {
    version: u16,
    server: String,
    first_sync: u64,
    last_sync: u64
}

pub struct Sync {
    sync: SyncFile,
    server: ClientServer,
    minecraft_server_path: Utf8PathBuf,
    http_client: Client
}

/// Takes care of parsing the .sync file inside each Minecraft folder and to to
impl Sync {
    pub fn new(config: &Config, path: Utf8PathBuf, server: Option<ClientServer>) -> Option<Self> {
        if !path.exists() {
            error!("Sync cannot be created as it points to an non-existing path: {}", path);
            return None;
        }

        if !path.is_dir() {
            error!("Sync cannot be created as it doesn't point to a directory: {}", path);
            return None;
        }

        let mut sync_file = path.clone();
        sync_file.push(".sync");

        let sync: SyncFile;

        if sync_file.exists() {
            sync = match File::open(sync_file) {
                Ok(file) => {
                    match serde_json::from_reader(file) {
                        Ok(json) => json,
                        Err(error) => {
                            error!("Error on parsing .sync within your Minecraft directory: {}", error);
                            return None;
                        }
                    }
                }
                Err(error) => {
                    error!("Cannot open .sync file within your Minecraft directory: {}", error);
                    return None;
                }
            };
        } else {
            if server.is_none() {
                error!("This Minecraft server is not yet synced.");
                return None;
            }

            sync = match File::create(sync_file) {
                Ok(mut file) => {
                    let default = SyncFile {
                        version: 1,
                        first_sync: 0,
                        last_sync: 0,
                        server: server.unwrap().id
                    };

                    let json = match serde_json::to_string_pretty(&default) {
                        Ok(j) => j,
                        Err(error) => {
                            error!("Failed to create JSON object for .sync file: {}", error);
                            return None;
                        }
                    };

                    match file.write_all(json.as_bytes()) {
                        Ok(_) => { },
                        Err(error) => {
                            error!("Failed to write to .sync: {}", error);
                            return None;
                        }
                    }

                    default
                }
                Err(error) => {
                    error!("Cannot create new .sync file within your Minecraft directory: {}", error);
                    return None;
                }
            }
        }

        if sync.version > SYNC_VERSION {
            error!("This sync has been made with a newer version of mcsync. Please upgrade.");
            return None;
        }

        let server = match config.get_server_by_id(&sync.server) {
            Some(s) => s,
            None => {
                error!("The .sync file points to a server that doesn't exist. Do you removed the server?");
                return None;
            }
        };

        let client = reqwest::ClientBuilder::new()
                    .connect_timeout(Duration::from_millis(200))
                    .user_agent("mcsync client")
                    .build().unwrap();

        Some(Self {
            minecraft_server_path: path,
            http_client: client,
            server,
            sync
        })
    }

    pub async fn create_on_remote(&self) {
        let req = self.http_client.post(format!("http://backend.mc/server"))
            .body(CreateServer {
                server_name: self.sync.
            });
    }

    pub async fn negotiate_delta(&self) {
        // Looks like this: "[HASH] [PATH]"
        let mut list: String = String::new();
        let mut amount: u32 = 0;

        info!("Compute local hashes ...");
        for entry in WalkDir::new(&self.minecraft_server_path) {
            let entry = match entry {
                Ok(e) => e,
                Err(error) => {
                    warn!("Couldn't access {} (skip file)", error.into_io_error().unwrap());
                    continue;
                }
            };

            let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf()).unwrap();

            if path.is_dir() {
                continue;
            }

            let mut file = match File::open(entry.path()) {
                Ok(f) => f,
                Err(error) => {
                    warn!("Couldn't open {} (skip file)", error);
                    continue;
                }
            };

            let hash = match sha256_digest(&mut file) {
                Ok(s) => s,
                Err(error) => {
                    error!("Unable to digest SHA-256 hash of {}: {} (skip file)", path, error);
                    continue;
                }
            };

            let final_hash = HEXLOWER.encode(hash.as_ref());

            println!("{} {}", final_hash, path.strip_prefix(&self.minecraft_server_path).unwrap());
            list += format!("{} {}\n", final_hash, path.to_string()).as_str();
            amount += 1;
        }

        info!("Done. Processed {} files.", amount);        

        let req = self.http_client.post(format!("http://backend.mc/server/{}/delta", self.server.id))
            .body(list)
            .send()
            .await;

        match req {
            Ok(res) => {
                if !res.status().is_success() {
                    error!("Couldn't retrive delta from server: {}", res.status());
                    return;
                }

                let delta = match res.text().await {
                    Ok(d) => d,
                    Err(error) => {
                        error!("Server sent a faulty response: {}", error);
                        return;
                    }
                };

                println!("{}", delta);
            }
            Err(error) => {
                error!("Server doesn't seem reachable: {}", error);
            }
        }

    }
}