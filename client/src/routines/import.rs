use std::{process::exit, fs::File};

use camino::Utf8PathBuf;
use paris::{error, success};
use serde::{Deserialize};
use uuid::Uuid;

use crate::config::{Config, ServerInfo};

pub struct Import {
}


impl Import {
    pub fn execute(mut config: Config, server_name: String, target_file: Utf8PathBuf) {
        if !target_file.exists() {
            error!("Selected server information file doesn't exist.");
            exit(1);
        }

        let new_uuid: String = match File::open(&target_file) {
            Ok(file) => {
                match serde_json::from_reader::<std::fs::File, ServerInfo>(file) {
                    Ok(si) => {
                        match config.add_server(server_name.clone(), si) {
                            Some(uuid) => uuid,
                            None => {
                                exit(1);
                            }
                        }
                    },
                    Err(error) => {
                        error!("Invalid syntax inside server info file: {}", error);
                        exit(1);
                    }
                }
            },
            Err(error) => {
                error!("Cannot open server info file: {}", error);
                exit(1);
            }
        };

        success!("Successfully added server {} ({}) to your configuration.", server_name, new_uuid);
        success!("You can now execute \"mcsync connect {}\" in order to play.", server_name);
    }

    pub fn write(&self) {
        
    }
}