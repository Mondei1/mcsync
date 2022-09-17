use std::{fs::File, time::Duration};

use camino::Utf8PathBuf;
use data_encoding::HEXLOWER;
use paris::{error, warn, info};
use reqwest::{Client, Body};
use serde::{Serialize, Deserialize};
use tokio_util::codec::{BytesCodec, FramedRead};
use walkdir::WalkDir;

use crate::{utils::hash::sha256_digest, config::{Config, ClientServer}};

pub const SYNC_VERSION: u16 = 1;

// === [ BEGIN HTTP JSON TYPES ] ===

#[derive(Serialize, Deserialize)]
pub struct CreateServer {
    server_name: String
}

#[derive(Serialize, Deserialize)]
pub struct CreateServerResponse {
    server_uuid: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileHash {
    pub(crate) id: u32,
    pub(crate) size: u64,
    pub(crate) path: String,
    pub(crate) hash: String
}

#[derive(Serialize, Deserialize)]
pub struct DeltaClient {
    files: Vec<FileHash>,
    last_sync: u64
}

#[derive(Serialize, Deserialize)]
pub struct DeltaServer {
    pub(crate) new: Vec<FileHash>,
    pub(crate) modified: Vec<FileHash>,
    pub(crate) removed: Vec<FileHash>
}

// === [ END HTTP JSON TYPES ] ===

#[derive(Serialize, Deserialize)]
pub struct SyncFile {
    pub(crate) version: u16,
    pub(crate) server: String,
    pub(crate) first_sync: u64,
    pub(crate) last_sync: u64
}

pub struct Sync {
    sync: SyncFile,
    server: ClientServer,
    minecraft_server_path: Utf8PathBuf,
    http_client: Client
}

/// Takes care of parsing the .sync file inside each Minecraft folder and to to.
/// This is a custom sync implementation. It is limited as it is designed to sync one way (client -> server)
/// It cannot handle files that where deleted afterwards
impl Sync {
    pub fn new(config: &Config, path: Utf8PathBuf) -> Option<Self> {
        if !path.exists() {
            error!("Sync cannot be created as it points to an non-existing path: {}", path);
            return None;
        }

        if !path.is_dir() {
            error!("Sync cannot be created as it doesn't point to a directory: {}", path);
            return None;
        }

        // Sync file should be in the root of the Minecraft server.
        let mut sync_file = path.clone();
        sync_file.push(".sync");

        let sync: SyncFile = if sync_file.exists() {
            match File::open(sync_file) {
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
            }
        } else {
            error!("This Minecraft server is not yet synced.");
            return None;
        };

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

    pub async fn create_on_remote(&self) -> Option<String> {
        let server = CreateServer {
            server_name: self.sync.server.clone()
        };

        let req = self.http_client.post("http://backend.mc/server")
            .json(&server)
            .send()
            .await;

        match req {
            Ok(res) => {
                if res.status().is_success() {
                    let json: CreateServerResponse =  match res.json().await {
                        Ok(j) => j,
                        Err(error) => {
                            error!("Server sent faulty response: {}", error);
                            return None;
                        }
                    };

                    return Some(json.server_uuid);
                }

                error!("Failed to create server on remote: {}", res.status());
                
                None
            }
            Err(error) => {
                error!("Failed to send server creation request: {}", error);
                None
            }
        }
    }

    pub async fn negotiate_delta(&self) -> Option<DeltaServer> {
        // Looks like this: "[HASH] [PATH]"
        let mut files: Vec<FileHash> = Vec::new();

        info!("Compute local hashes ...");
        let mut id: u32 = 0;

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
                    error!("Unable to compute SHA-256 hash of {}: {} (skip file)", path, error);
                    continue;
                }
            };

            let final_hash = HEXLOWER.encode(hash.as_ref());

            let file = FileHash {
                id,
                size: file.metadata().unwrap().len(),
                path: path.strip_prefix(&self.minecraft_server_path).unwrap().to_string(),
                hash: final_hash
            };

            println!("{} {}", file.hash, file.path);
            files.push(file);

            id += 1;
        }

        info!("Done. Processed {} files.", files.len());

        let server_request_body = DeltaClient {
            files,
            last_sync: self.sync.last_sync
        };

        let req = self.http_client.post(format!("http://backend.mc/server/{}/delta", self.server.id))
            .json(&server_request_body)
            .send()
            .await;

        let delta: DeltaServer = match req {
            Ok(res) => {
                if !res.status().is_success() {
                    error!("Couldn't retrive delta from server: {}", res.status());
                    return None;
                } else if res.status().as_u16() == 409 {
                    error!("There is a conflict! The server already has a newer version. Unfortunately there is now way to resolve conflicts right now. As for now you cannot sync.");
                    return None;
                }

                match res.json().await {
                    Ok(d) => d,
                    Err(error) => {
                        error!("Server sent a faulty response: {}", error);
                        return None;
                    }
                }
            }
            Err(error) => {
                error!("Server doesn't seem reachable: {}", error);
                return None;
            }
        };

        info!("Delta summary: {} new, {} modified and {} files were deleted since last sync.", delta.new.len(), delta.modified.len(), delta.removed.len());

        Some(delta)
    }

    // At this point, the server grants our IP to send over the new files. No need for authentication.
    pub async fn transfer(&self, sync_file: &FileHash) -> bool {
        let file = match tokio::fs::File::open(&sync_file.path).await {
            Ok(f) => f,
            Err(error) => {
                error!("File {} got deleted/moved while sync is in process: {}", &sync_file.path, error);
                return false;
            }
        };

        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);

        let req = self.http_client
            .post(format!("http://backend.mc/server/{}/transfer/{}", self.sync.server, sync_file.id))
            .body(body)
            .send()
            .await;

        match req {
            Ok(res) => {
                if res.status().is_success() {
                    return true;
                }

                error!("Server respond with error code {}", res.status());
                false
            }
            Err(error) => {
                error!("Request failed: {}", error);
                false
            }
        }
    }
}
