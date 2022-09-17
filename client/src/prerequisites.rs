use std::{
    ffi::OsStr,
    process::{exit, Command},
};

use cfg_if::cfg_if;

use paris::{error, info};

use crate::utils::{child::spawn_child, rclone};

pub struct Prerequisites {}

#[derive(Debug)]
pub enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Apk,
    Emerge,
}

impl Prerequisites {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn check(&self) {
        // 1. Check for WireGuard
        if !is_wireguard_module_available() {
            cfg_if! {
                if #[cfg(unix)] {
                    let package_manager = match determine_package_manager() {
                        Some(pm) => pm,
                        None => {
                            error!("Cannot determine your package manager and therefore cannot install WireGuard kernel modules.");
                            exit(1);
                        }
                    };

                    info!("Determined {:?} as your package manager.", package_manager);
                    info!("Run installation (this might take a few seconds) ...");

                    let install_status = install_wireguard_module(package_manager);
                    if !install_status {
                        error!("Automatic installation failed (see previous error). You're on your own, sorry. Come back once you installed the kernel module.");
                        error!("You can execute \"modinfo wireguard\" to check if you installed everything correctly.");
                        exit(1);
                    }

                    info!("Installation successful! Please reboot your computer so the new modules get loaded.");
                    exit(0);
                }
            }
        }

        // 2. Check for rclone
        if rclone::check_for_update().await {
            rclone::install_latest_version().await;
        }
    }
}

pub fn is_wireguard_module_available() -> bool {
    cfg_if! {
        if #[cfg(unix)] {
            match Command::new("modinfo").arg("wireguard").output() {
                Ok(o) => {
                    if std::str::from_utf8(&o.stderr).unwrap().contains("ERROR:") {
                        error!("WireGuard kernel module is unavailable.");
                    } else {
                        // success!("WireGuard module is available.");
                        return true;
                    }
                },
                Err(error) => {
                    error!("Cannot run modinfo: {}", error);
                    error!("Expect the WireGuard kernel module not to be installed.");
                }
            }

            false
        }
    }
}

pub fn install_wireguard_module(package_manager: PackageManager) -> bool {
    cfg_if! {
        if #[cfg(unix)] {
            match package_manager {
                PackageManager::Apt => {
                    return run_package_manager("apt", ["-qq", "-y", "install", "wireguard"]);
                },
                PackageManager::Dnf => {
                    return run_package_manager("dnf", ["-y", "-q", "install", "wireguard-tools"]);
                },
                PackageManager::Pacman => {
                    return run_package_manager("pacman", ["--noconfirm", "-qq", "-S", "wireguard-toolss"]);
                },
                PackageManager::Apk => {
                    return run_package_manager("apk", ["-q", "--no-progress", "install", "wireguard"]);
                }
                PackageManager::Emerge => {
                    return run_package_manager("emerge", ["--noconfmem", "-q", "wireguard-tools"]);
                }
            }
        }
    }
}

pub fn run_package_manager<I, S>(command: &str, args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let result = spawn_child(command, args);

    if result {
        info!("Package installation finished successfully.");
    } else {
        error!("Failed to install the modules. See previous errors.");
    }
    
    result
}

pub fn determine_package_manager() -> Option<PackageManager> {
    // Apt
    match Command::new("apt").output() {
        Ok(_) => {
            return Some(PackageManager::Apt);
        }
        Err(_) => {}
    }

    // Dnf
    match Command::new("dnf").output() {
        Ok(_) => {
            return Some(PackageManager::Dnf);
        }
        Err(_) => {}
    }

    // Pacman
    match Command::new("pacman").output() {
        Ok(_) => {
            return Some(PackageManager::Pacman);
        }
        Err(_) => {}
    }

    // Apk
    match Command::new("apk").output() {
        Ok(_) => {
            return Some(PackageManager::Apk);
        }
        Err(_) => {}
    }

    // Emerge
    match Command::new("emerge").output() {
        Ok(_) => {
            return Some(PackageManager::Emerge);
        }
        Err(_) => {}
    }

    None
}
