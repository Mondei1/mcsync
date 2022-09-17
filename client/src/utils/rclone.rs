use std::{io::{Cursor, copy}, fs::{File}, process::Command};

use crate::{platform::get_rclone_executable, utils::child::spawn_child};

use humansize::{DECIMAL};
use paris::{error, info, success};
use cfg_if::cfg_if;

/// Will return the current installed version if it's installed and `None` if it's not.
pub fn get_installed_version() -> Option<String> {
    if !get_rclone_executable().exists() {
        return None;
    }

    let version_output = Command::new(get_rclone_executable())
        .arg("--version")
        .output()
        .unwrap()
        .stdout;

    // This looks very dangerous because it is. Just pray rclone doesn't change it's
    // output format or everything will panic.
    let version_string = std::str::from_utf8(&version_output).unwrap()
        .split("\n")
        .collect::<Vec<&str>>()
        .get(0)
        .unwrap()
        .split(" ")
        .collect::<Vec<&str>>()
        .get(1)
        .unwrap()
        .replace("v", "");

    Some(version_string)
}

pub async fn get_latest_version() -> Option<String> {
    const RCLONE_VERSION_FILE: &str = "https://downloads.rclone.org/version.txt";

    match reqwest::get(RCLONE_VERSION_FILE).await {
        Ok(res) => {
            let raw = res.text().await.unwrap();
            let version_parsed = raw.split(" ");

            match version_parsed.collect::<Vec<&str>>().get(1) {
                Some(v) => {
                    Some(v.to_string()
                        .replace("v", "")
                        .replace("\n", ""))
                },
                None => {
                    error!("Format of version.txt by rclone seems to have changed. Please upgrade mcsync or open an issue.");
                    error!("Expected: 'rclone v[VERSION]' Got: {}", raw);
    
                    return None;
                }
            }
        },
        Err(error) => {
            error!("Failed to retrive the latest rclone version file. URL: {}, Error: {}", RCLONE_VERSION_FILE, error);
            return None;
        }
    }
}

pub async fn check_for_update() -> bool {
    let latest_version = match get_latest_version().await {
        Some(v) => v,
        None => {
            error!("Coudln't acquire latest version info. See above errors.");
            return false;
        }
    };

    let installed_version = get_installed_version();
    if installed_version.is_some() {
        let iv = installed_version.unwrap();

        // Abort if the latest version is already installed.
        if iv == latest_version {
            return false;
        }

        info!("Latest release of rclone is {} but an older version ({}) is installed", latest_version, iv);
    }

    return true;
}

pub async fn install_latest_version() -> Option<()> {
    let latest_version = get_latest_version().await.unwrap();

    // Get current architecture
    let arch = {
        match std::env::consts::ARCH {
            "x86" => {
                String::from("386")
            },
            "x86_64" => {
                String::from("amd64")
            }
            "arm" => {
                String::from("arm-v7")
            }
            "aarch64" => {
                String::from("arm64")
            }
            _ => {
                error!("Couldn't determine current architecture. Default to x86_64 (amd64).");
                String::from("amd64")
            }
        }
    };

    cfg_if! {
        if #[cfg(unix)] {
            // Before anyone jumps at me and says "You know you can just download from https://downloads.rclone.org/rclone-current-linux-ARCH.zip"
            //   -> I want to be able to compare the current install and the latest version.
            let download_url = format!("https://downloads.rclone.org/v{version}/rclone-v{version}-linux-{arch}.zip", version = latest_version, arch = arch);
            info!("Download file {} ...", download_url);

            // We download the latest version into memory and deflate it later.
            let archive = match reqwest::get(&download_url).await {
                Ok(res) => {
                    if !res.status().is_success() {
                        error!("Server responded with an non-success code: {}", res.status());
                        error!("Maybe rclone is not available for your architecture. Check out their download page: https://rclone.org/downloads/");
                        return None;
                    }

                    let data = res.bytes().await.unwrap();
                    success!("Downloaded rclone v{} ({})", latest_version, humansize::format_size(data.len(), DECIMAL));

                    data
                },
                Err(error) => {
                    error!("Failed to download rclone archive from {}: {}", download_url, error);
                    return None;
                }
            };

            //let mut tmp = File::create("/tmp/rclone.zip").unwrap();

            // Convert Vec<u8> into [u8]
            let mut cursor: Cursor<Vec<u8>> = Cursor::new(archive.to_vec());
            cursor.set_position(0);

            let mut zip_file = match zip::ZipArchive::new(cursor) {
                Ok(z) => z,
                Err(error) => {
                    error!("Downloaded file appears not to be a zip-file. Very strange... Error: {}", error);
                    return None;
                }
            };

            let file_name = format!("rclone-v{}-linux-{}/rclone", latest_version, arch);
            match zip_file.by_name(&file_name) {
                Ok(mut f) => {
                    let mut dest = File::create(get_rclone_executable()).unwrap();
                    
                    match copy(&mut f, &mut dest) {
                        Ok(_) => {
                            // Make file executable. I don't know if there is a "rusty" way to do this but here we are.
                            spawn_child("chmod", ["+x", get_rclone_executable().as_str()]);
                            success!("Successfully extracted the newest rclone executable to {}", get_rclone_executable());
                        },
                        Err(error) => {
                            error!("Extraction of the newest rclone executable failed: {}", error);
                            return None;
                        }
                    }
                },
                Err(_) => {
                    error!("Downloaded zip doens't contain a file called 'rclone'. You may need to update mcsync.");
                    return None;
                }
            }

            Some(())
        }
    }
}