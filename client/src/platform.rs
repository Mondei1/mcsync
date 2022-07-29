use std::{process::exit};

use camino::Utf8PathBuf;
use paris::error;

pub fn get_config_directory() -> Utf8PathBuf {
    match dirs::config_dir() {
        Some(mut dir) => {
            dir.push("mcsync");
            dir.push("config.json");

            Utf8PathBuf::from_path_buf(dir).unwrap()
        },
        None => {
            error!("Cannot obtain config directory. You need to specify the directory manual using --config-dir=PATH");
            exit(1);
        }
    }
}