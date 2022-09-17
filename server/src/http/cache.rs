use serde::{Serialize, Deserialize};

use crate::database::Database;

/// This struct keeps track who is allowed to push changes to which Minecraft server.
/// It also remembers the current state of each server.

pub struct Cache {
    server_sync: Vec<ServerSync>
}

// Which IP is allowed to sync which server?
#[derive(Clone)]
pub struct ServerSync {
    ip: String,
    sync_id: String
}

impl Cache {
    pub fn new() -> Self {
        Self { server_sync: vec![] }
    }

    pub fn add_sync(&mut self, ip: String, sync_id: String) {
        self.server_sync.push(ServerSync { ip, sync_id });
    }

    pub fn can_sync(&self, ip: String, target_sync: String) -> bool {
        self.server_sync.clone().into_iter().any(|x| x.ip == ip && x.sync_id == target_sync)
    }
}
