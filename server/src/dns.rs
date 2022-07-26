use std::{fs::File, io::Write, fs::remove_file};

use log::warn;
use paris::{error, info, success, log};

use crate::{docker::DockerManager, routines::remove};

pub struct DNSManager {
    docker_instance: DockerManager,
    zone_dir: String,
}

impl DNSManager {
    pub fn new(docker_instance: DockerManager) -> Self {
        let zone_dir = match std::env::var("DNS_ZONE_DIR") {
            Ok(file_path) => file_path,
            Err(_) => {
                info!("Fallback to /dns/mcsync.d/ because DNS_ZONE_DIR has not been specified.");
                String::from("/dns/mcsync.d/")
            }
        };

        Self {
            docker_instance,
            zone_dir,
        }
    }

    pub fn remove_record(&self, name: &str) -> Option<()> {
        match remove_file(format!("{}/{}.conf", self.zone_dir, name)) {
            Ok(_) => {
                info!("Removed record for {}.mc", name);

                Some(())
            },
            Err(error) => {
                error!("Unable to delete record {}/{}.conf: {}: ", self.zone_dir, name, error);
                None
            }
        }
    }

    pub fn set_or_update_record(&self, name: &str, new_ip: &str) -> Option<()> {
        let mut record = format!(
            "server:
\tlocal-data: \"{}.mc.	IN A {}\"\n
\tlocal-data-ptr: \"{}	{}.mc\"",
                name, new_ip, new_ip, name
        );

        match File::create(format!("{}/{}.conf", self.zone_dir, name)) {
            Ok(mut file) => {
                match file.write_all(record.as_bytes()) {
                    Ok(_) => {
                        match file.flush() {
                            Ok(_) => {
                                log!("DNS record for {}.mc now points to {}", name, new_ip);
                            },
                            Err(error) => {
                                error!("Error on flusing new DNS configuration: {}", error);
                                return None;
                            }
                        }
                        
                    },
                    Err(error) => {
                        error!("Error on writing new DNS configuration: {}", error);
                        return None;
                    }
                }

                Some(())
            },
            Err(error) => {
                error!("DNS record file ({}.conf) couldn't be opened: {}", name, error);
                None
            }
        }
    }

    pub async fn restart_dns(&self) {
        match self.docker_instance.get_dns_container().await {
            Some(container) => {
                info!(
                    "Applied changes to DNS. Restart container {} ...",
                    container.id.as_ref().unwrap()
                );

                match self.docker_instance.restart_container(container).await {
                    Ok(_) => {
                        success!("DNS container restarted successfully.");
                    }
                    Err(error) => {
                        error!("Couldn't restart DNS container: {}", error);
                    }
                }
            }
            None => {
                warn!("Applied changes to DNS but its container cannot be found. You have to restart it manually. Did you change names?");
            }
        }
    }
}
