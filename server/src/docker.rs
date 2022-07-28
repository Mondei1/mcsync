use std::{collections::HashMap, process::exit};

use bollard::{Docker, API_DEFAULT_VERSION, container::{ListContainersOptions, RestartContainerOptions}, models::ContainerSummary, errors::Error};
use paris::{error, warn, success, info};

use crate::SILENT;

#[derive(Clone)]
pub struct DockerManager {
    docker_daemon: Docker,
}

impl DockerManager {
    pub async fn new() -> DockerManager {
        let docker_path = match std::env::var("DOCKER_SOCKET") {
            Ok(d) => format!("unix://{}", d),
            Err(_) => {
                if !*SILENT.lock().unwrap() {
                    info!(
                        "Fallback to /var/run/docker.sock because DOCKER_SOCKET has not been specified."
                    );
                }
                String::from("unix:///var/run/docker.sock")
            }
        };

        let docker_daemon = match Docker::connect_with_socket(&docker_path, 30, API_DEFAULT_VERSION)
        {
            Ok(d) => {
                match d.version().await {
                    Ok(version_data) => {
                        if !*SILENT.lock().unwrap() {
                            success!("Connected with Docker (version {})", version_data.version.unwrap());
                        }

                        d
                    },
                    Err(error) => {
                        error!("Unable to connect with Docker: {}", error);
                        warn!("Access to the Docker daemon is required in order to restart services on changes.");
                        exit(1);
                    }
                }
            },
            // This crate never returns an Error object.
            Err(_) => { exit(1) }
        };

        Self { docker_daemon }
    }

    pub async fn find_container_by_name(&self, name: &str, running_only: bool) -> Option<ContainerSummary> {
        let mut list_container_filters = HashMap::new();

        if running_only {
            list_container_filters.insert("status", vec!["running"]);
        }

        let containers = &self.docker_daemon
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: list_container_filters,
                ..Default::default()
            }))
            .await
            .unwrap();

        for container in containers {
            match &container.names {
                None => { continue; },
                Some(names) => {
                    for container_name in names {
                        if container_name.contains(&name.to_string()) && container_name.contains("mcsync") {
                            return Some(container.clone());
                        }
                    }
                }
            }
        }

        return None;
    }

    /** This will only work for containers inside the "mcsync" network. */
    pub async fn get_container_ip(&self, container: ContainerSummary) -> Option<String> {
        match container.network_settings {
            None => {
                warn!("Couldn't retrive information about {}", container.id.as_ref().unwrap());
                None
            },
            Some(settings) => {
                match settings.networks {
                    None => {
                        warn!("Couldn't retrive Docker networks from {}", container.id.as_ref().unwrap());
                        None
                    },
                    Some(networks) => {
                        match networks.get("mcsync") {
                            None => {
                                warn!("Couldn't find network \"mcsync\" for container {}", container.id.as_ref().unwrap());
                                None
                            },
                            Some(network_detail) => {
                                Some(network_detail.ip_address.clone().unwrap())
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn restart_container(&self, container: ContainerSummary) -> Result<(), Error> {
        self.docker_daemon.restart_container(&container.id.unwrap(), Some(RestartContainerOptions {
            ..Default::default()
        })).await
    }

    pub async fn get_self(&self) -> Option<ContainerSummary> {
        self.find_container_by_name("mcsync", true).await
    }

    pub async fn get_dns_container(&self) -> Option<ContainerSummary> {
        self.find_container_by_name("dns", true).await
    }

    pub async fn get_ssh_container(&self) -> Option<ContainerSummary> {
        self.find_container_by_name("ssh", true).await
    }

    pub async fn get_vpn_container(&self) -> Option<ContainerSummary> {
        self.find_container_by_name("wireguard", true).await
    }
    
}
