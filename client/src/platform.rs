use std::{process::exit, fs::File, io::Read};

use cfg_if::cfg_if;

use camino::Utf8PathBuf;
use paris::error;

use crate::{is_root, config::{Config, ClientServer}};

pub fn get_config_directory() -> Utf8PathBuf {
    match dirs::config_dir() {
        Some(mut dir) => {
            dir.push("mcsync");
            dir.push("config.json");

            Utf8PathBuf::from_path_buf(dir).unwrap()
        },
        None => {
            error!("Cannot obtain config directory. You need to specify the directory manually using --config-dir=PATH");
            exit(1);
        }
    }
}

pub fn get_wg_config() -> Utf8PathBuf {
    cfg_if! {
        if #[cfg(unix)] {
            Utf8PathBuf::from(format!("/tmp/.mcsync/{}/wg0.conf", users::get_current_uid()))
        }
    }
}

pub fn get_rclone_executable() -> Utf8PathBuf {
    cfg_if! {
        if #[cfg(unix)] {
            Utf8PathBuf::from(format!("{}/.local/bin/rclone", dirs::home_dir().unwrap().to_str().unwrap()))
        }
    }
}

pub fn does_wg_interface_exist() -> bool {
    cfg_if! {
        if #[cfg(unix)] {
            pnet_datalink::interfaces().into_iter().any(|x| x.name == "wg0")
        }
    }
}

pub fn is_connected(config: &Config) -> Option<ClientServer> {
    if !does_wg_interface_exist() {
        return None;
    }

    cfg_if! {
        if #[cfg(unix)] {
            match File::open(get_wg_config()) {
                Ok(mut file) => {
                    let mut buf: Vec<u8> = Vec::new();
                    let _ = file.read_to_end(&mut buf);

                    for line in std::str::from_utf8(&buf).unwrap().split('\n') {
                        if line.starts_with("Endpoint") {
                            let endpoint: Vec<&str> = line.split(" = ").clone().collect();
                            
                            match endpoint.to_owned().get(1) {
                                Some(address) => {
                                    return config.get_server_by_endpoint(address);
                                },
                                None => {
                                    error!("Couldn't parse endpoint from {}", get_wg_config());
                                    return None;
                                }
                            }
                        }
                    }

                    None
                }
                Err(_) => {
                    None
                }
            }
        }
    }
}

pub fn permission_check() {
    cfg_if! {
        if #[cfg(unix)] {
            if !is_root() {
                error!("No permission to create tunnel. Easiest way is to run with root privileges.\n");

                // Temp disabled because you actually need to raise the caps of wg-quick and not of mcsync. Needs more testing.
                //error!("If you don't want to use mcsync with root privileges, raise the capabilities of this executable.");
                //error!("Run: sudo setcap cap_net_admin=+ep {}", current_exe().unwrap().to_str().unwrap());
                exit(1);
            }

            /*match caps::has_cap(None, caps::CapSet::Effective, Capability::CAP_NET_ADMIN) {
                Ok(caps) => {
                    if !caps {
                        if !is_root() {
                            error!("No permission to create tunnel. Easiest way is to run with root privileges.\n");
                            error!("If you don't want to use mcsync with root privileges, raise the capabilities of this executable.");
                            error!("Run: sudo setcap cap_net_admin=+ep {}", current_exe().unwrap().to_str().unwrap());
                            //exit(1);
                        }
                    }
                }
                Err(error) => {
                    error!("Couldn't retrive capabilities of this executable: {}", error);
                    exit(1);
                }
            }*/
        }
    }
}
