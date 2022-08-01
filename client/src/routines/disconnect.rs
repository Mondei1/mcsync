use std::{process::{exit, Command}, fs::{remove_file}};

use paris::{error, warn, success};

use crate::platform::{get_wg_config, does_wg_interface_exist};

pub struct Disconnect {

}

impl Disconnect {
    pub fn execute() {
        let path = get_wg_config();

        if !does_wg_interface_exist() {
            error!("You're not connected with any server.");
            exit(1);
        }

        // Instruct WireGuard to disconnect using wireguard-tools
        match Command::new("wg-quick").args(["down", &path.to_string()]).spawn() {
            Ok(mut child) => {
                if !child.wait().unwrap().success() {
                    error!("WireGuard failed to destroy your tunnel. See possible errors above.");
                    exit(1);
                }
            },
            Err(error) => {
                error!("Connection failed. Cannot spawn wg-quick process. Is it installed? Error: {}", error);
                exit(1);
            }
        }

        if path.exists() {
            match remove_file(&path) {
                Ok(_) => {},
                Err(error) => {
                    error!("Cannot delete config file at {}: {}", path, error);
                }
            }
        } else {
            warn!("There is no WireGuard config at {} to delete but you're still connected. Something is off...", path);
            exit(1);
        }

        success!("Successfully disconnected. You can no longer access their Minecraft servers.");
    }
}
