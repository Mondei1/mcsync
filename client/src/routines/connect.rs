use std::{
    fs,
    io::Write,
    os::unix::prelude::OpenOptionsExt,
    process::{exit, Command}, time::Duration,
};

use camino::Utf8PathBuf;
use paris::{error, success};

use crate::{config::{ClientServer, Config}, platform::{get_wg_config, does_wg_interface_exist}};

pub struct Connect {}

impl Connect {
    pub async fn execute(config: Config, server: ClientServer, use_vpn: bool) {
        let template = format!(
            r"
###
# GENERATED BY mcsync
# DO NOT TOUCH. WILL BE DELETED AS SOON AS YOU DISCONNECT FROM YOUR REMOTE SERVER.
###

# This is you
[Interface]
PrivateKey = {private_key}
Address = {ipv4_address}/32
DNS = {dns}

# This is {server_name} - your remote server
[Peer]
PublicKey = {server_public_key}
PresharedKey = {psk}
AllowedIPs = {user_subnet}, {tool_subnet}
Endpoint = {endpoint}
PersistentKeepalive = 25
        ",
            private_key = &config.get_data().keys.wg,
            ipv4_address = &server.ipv4_address,
            dns = &server.dns,
            server_name = &server.name,
            server_public_key = &server.public_key,
            psk = &server.psk,
            user_subnet = &server.user_subnet,
            tool_subnet = &server.tool_subnet,
            endpoint = &server.endpoint
        );

        let path: Utf8PathBuf;

        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                if use_vpn {
                    path = get_wg_config();

                    let mut dir = path.clone();
                    dir.pop();

                    if !dir.exists() {
                        match fs::create_dir_all(&dir) {
                            Ok(_) => {},
                            Err(error) => {
                                error!("Cannot create directory at {}: {}", &dir, error);
                                exit(1);
                            }
                        }
                    }

                    if path.exists() {
                        if does_wg_interface_exist() {
                            error!("You are still connected with a server. You first need to disconnect before you can join another one.");
                            error!("Run \"mcsync disconnect\" to disconnect.");

                            exit(1);
                        }

                        error!("There is already a WireGuard config file. However you're no longer connected with a server. You should be fine if you just delete {}", &path);
                        exit(1);
                    }

                    let file_options = fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .mode(0o660)
                        .open(&path);

                    match file_options {
                        Ok(mut file) => {
                            match file.write_all(template.as_bytes()) {
                                Ok(_) => { },
                                Err(error) => {
                                    error!("I/O error for file {}: {}", &path, error);
                                    exit(1);
                                }
                            }
                        },
                        Err(error) => {
                            error!("Couldn't create WireGuard config file at {}: {}", &path, error);
                            exit(1);
                        }
                    }

                    // Instruct WireGuard to connect using wireguard-tools
                    match Command::new("wg-quick").args(["up", path.as_ref()]).spawn() {
                        Ok(mut child) => {
                            if !child.wait().unwrap().success() {
                                error!("WireGuard failed to setup your tunnel. See possible errors above.");
                                exit(1);
                            }
                        },
                        Err(error) => {
                            error!("Connection failed. Cannot spawn wg-quick process. Is it installed? Error: {}", error);
                            exit(1);
                        }
                    }
                }

                // Check if backend is reachable.
                let client = reqwest::ClientBuilder::new()
                    .connect_timeout(Duration::from_millis(200))
                    .user_agent("mcsync client")
                    .build().unwrap();

                match client.get("http://backend.mc:8080").send().await {
                    Ok(res) => {
                        if res.status() == 200 {
                            success!("Connected with {}. Have fun playing!", &server.name);
                        } else {
                            error!("Connection succeeded but server backend returned with {}", res.status());
                        }
                    }
                    Err(error) => {
                        error!("Connection failed. The server backend is unreachable, so WireGuard may not have been able to connect.\nError: {}\n", error);
                        error!("If you don't want to troubleshoot the problem, type \"mcsync disconnect\"");

                        exit(1);
                    }
                }
            }
        }
    }
}
