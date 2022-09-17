use std::{process::exit, fs::File, io::Write};

use camino::Utf8PathBuf;
use humansize::{format_size, DECIMAL};
use nix::unistd::getcwd;
use paris::{error, info};

use crate::{config::Config, sync::{Sync, FileHash, SyncFile}, platform::is_connected};

pub const MAX_PATH_LENGTH: usize = 80;

pub struct Init {
}

impl Init {
    pub async fn execute(mut config: Config, sync_name: String, start_file: Utf8PathBuf) {
        if config.get_sync_by_name(sync_name.as_str()).is_some() {
            error!("There is already a sync with this name.");
            exit(1);
        }

        let cwd: Utf8PathBuf = match getcwd() {
            Ok(c) => Utf8PathBuf::from_path_buf(c).unwrap(),
            Err(error) => {
                error!("Unable to obtain your current working directory: {}", error);
                exit(1);
            }
        };

        let current_server = match is_connected(&config) {
            Some(s) => s,
            None => {
                error!("You need to connect with a server first!");
                exit(1);
            }
        };

        let mut sync_file = cwd.clone();
        sync_file.push(".sync");

        if sync_file.exists() {
            info!("This server is already synced.");
            return;
        }

        match File::create(&sync_file) {
            Ok(mut file) => {
                let default = SyncFile {
                    version: 1,
                    first_sync: 0,
                    last_sync: 0,
                    server: is_connected(&config).unwrap().id
                };

                let json = match serde_json::to_string_pretty(&default) {
                    Ok(j) => j,
                    Err(error) => {
                        error!("Failed to create JSON object for .sync file: {}", error);
                        return;
                    }
                };

                match file.write_all(json.as_bytes()) {
                    Ok(_) => { },
                    Err(error) => {
                        error!("Failed to write to .sync: {}", error);
                        return;
                    }
                }

                config.add_sync(&sync_name, current_server.id, start_file);
            }
            Err(error) => {
                error!("Cannot create new .sync file within your Minecraft directory: {}", error);
                return;
            }
        }

        let sync = match Sync::new(&config, cwd) {
            Some(s) => s,
            None => {
                error!("Failed to initialize Minecraft server. See previous erros.");
                exit(1);
            }
        };

        let mut delta = match sync.negotiate_delta().await {
            Some(d) => d,
            None => {
                error!("Couldn't negotiate delta with remote. See previous errors.");
                exit(1);
            }
        };

        let mut final_send: Vec<FileHash> = Vec::new();
        final_send.append(&mut delta.new);
        final_send.append(&mut delta.modified);

        let file_amount = final_send.len();

        for (sent, sync_file) in final_send.into_iter().enumerate() {
            let progress = ((sent / file_amount * 100) as f64).round() as u32;
            let mut print_progress = String::new();

            // Format progress
            if progress.to_string().len() < 3 {
                for _ in 0..(3 - progress.to_string().len()) {
                    print_progress += " ";
                }

                print_progress += &progress.to_string();
            }

            let mut print_path = sync_file.clone().path;

            // Cut of path at the start to save space
            if print_path.len() > MAX_PATH_LENGTH {
                let begin = print_path.len() - MAX_PATH_LENGTH;
                let end = print_path.len();

                print_path = format!("...{}", print_path.get(begin..end).unwrap());
            } else {
                print_path = format!("{} ", print_progress);
            }

            println!("({} %) ↑ {} ({})", print_progress, print_path, format_size(sync_file.size, DECIMAL));

            let _ = sync.transfer(&sync_file).await;
        }
    }
}
