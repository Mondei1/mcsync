use std::{process::exit};

use cfg_if::cfg_if;

use camino::Utf8PathBuf;
use paris::error;

use crate::is_root;

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
            return Utf8PathBuf::from(format!("/tmp/.mcsync/{}/wg0.conf", users::get_current_uid()));
        }
    }
}

pub fn does_wg_interface_exist() -> bool {
    cfg_if! {
        if #[cfg(unix)] {
            pnet_datalink::interfaces().into_iter().find(|x| x.name == "wg0").is_some()
        }
    }
}

pub fn permission_check() {
    cfg_if! {
        if #[cfg(unix)] {
            if !is_root() {
                error!("No permission to create tunnel. Easiest way is to run with root privileges.\n");
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