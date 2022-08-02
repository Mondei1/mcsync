use std::process::exit;

use camino::Utf8PathBuf;
use nix::unistd::getcwd;
use paris::error;

use crate::{config::Config, sync::Sync};

pub struct Init {
}

impl Init {
    pub async fn execute(config: Config, sync_name: String) {
        match config.get_sync_by_name(sync_name.as_str()) {
            Some(_) => {
                error!("There is already a sync with this name.");
                exit(1);
            },
            None => { }
        }

        let cwd = match getcwd() {
            Ok(c) => c,
            Err(error) => {
                error!("Unable to obtain your current working directory: {}", error);
                exit(1);
            }
        };

        if config.get_current_server().is_none() {
            error!("You need to connect with a server first!");
            exit(1);
        }

        let sync = match Sync::new(&config, Utf8PathBuf::from_path_buf(cwd).unwrap(), config.get_current_server()) {
            Some(s) => s,
            None => {
                error!("Failed to initialize Minecraft server. See previous erros.");
                exit(1);
            }
        };

        sync.negotiate_delta().await;
    }
}