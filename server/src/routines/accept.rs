use std::{fs::File, io::Read};

use log::info;
use paris::{error, success};
use serde::Deserialize;

pub struct Accept {
}

#[derive(Debug, Deserialize)]
struct InvitationRequest {
    version: u16,
    wireguard_pub: String,
    ssh_pub: String
}

impl Accept {
    pub fn execute() {
        let args: Vec<String> = std::env::args().collect();
        let filepath = args.get(2);

        if filepath.is_none() {
            error!("No file path provided!");
            std::process::exit(1);
        }

        let path = filepath.unwrap();
        match File::open(path) {
            Ok(mut data) => {
                let mut contents = String::new();
                let read_operation = data.read_to_string(&mut contents);

                if read_operation.is_err() {
                    error!("Couldn't read file: {}", read_operation.unwrap_err());
                    std::process::exit(1);
                }

                success!("Read invitation ({} bytes)", read_operation.unwrap());

                let parsed = match serde_json::from_str::<InvitationRequest>(&contents) {
                    Ok(o) => o,
                    Err(error) => {
                        error!("Invalid json syntax: {}", error);
                        std::process::exit(1);
                    }
                };

                println!("Data: {:?}", parsed);
            },
            Err(error) => {
                error!("Couldn't open file: {}", error);
                std::process::exit(1);
            }
        }
    }
}