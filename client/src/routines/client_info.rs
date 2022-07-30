use std::process::exit;

use paris::error;
use serde::{Serialize, Deserialize};

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    version: u16,
    wireguard_pub: String
}

impl ClientInfo {
    pub fn new(config: Config) -> Self {
        Self {
            version: 1,
            wireguard_pub: config.get_public_wireguard_key().to_base64()
        }
    }

    pub fn print(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                println!("{}", json);
            },
            Err(error) => {
                error!("Cannot create json string for your client info: {}", error);
                exit(1);
            }
        }
    }
}