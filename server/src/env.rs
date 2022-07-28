use std::{process::exit, path::Path};

use ipnet::Ipv4Net;
use nickel::hyper::Url;
use paris::{info, error};

/*
 Collection of functions to retrive & parse env input.
*/

pub fn get_user_subnet() -> Ipv4Net {
    let user_subnet = match std::env::var("USER_SUBNET") {
        Ok(subnet) => subnet,
        Err(_) => {
            String::from("192.168.10.0/24")
        },
    };

    match user_subnet.parse() {
        Ok(s) => s,
        Err(error) => {
            error!("Your custom user subnet {} is invalid. Consider choosing another one: {}", user_subnet, error);
            exit(1);
        }
    }
}

pub fn get_endpoint() -> String {
    match std::env::var("ENDPOINT") {
        Ok(endpoint) => {
            // We just pretent it's a http URL.
            match Url::parse(&format!("http://{}", endpoint)) {
                Ok(result) => {
                    if result.port().is_none() {
                        error!("You need to set a port in ENDPOINT. If you're not sure use 51820 as it's WireGuard's default port.");
                        exit(1);
                    }

                    endpoint
                },
                Err(error) => {
                    error!("Environment variable contains invalid address: {}", error);
                    exit(1);
                }
            }
        },
        Err(_) => {
            error!("You need to set the environment variable ENDPOINT which is an address ( [Domain OR IPv4]:PORT ) under which WireGuard is reachable by others.");
            exit(1);
        }
    }
}

pub fn get_wg_config() -> String {
    match std::env::var("WG_CONFIG") {
        Ok(file_path) => {
            if !Path::new(&file_path).exists() {
                error!("File {} in WG_CONFIG doesn't exist.", &file_path);
                exit(1);
            }
            file_path
        }
        Err(_) => {
            info!("Fallback to /vpn/wg0.conf because WG_CONFIG has not been specified.");
            String::from("/vpn/wg0.conf")
        }
    }
}

pub fn get_docker_path() -> String {
    match std::env::var("DOCKER_SOCKET") {
        Ok(d) => format!("unix://{}", d),
        Err(_) => {
            String::from("unix:///var/run/docker.sock")
        }
    }
}

pub fn get_dns_zone_dir() -> String {
    match std::env::var("DNS_ZONE_DIR") {
        Ok(file_path) => {
            if !Path::new(&file_path).exists() {
                error!("Directory {} in DNS_ZONE_DIR doesn't exist.", &file_path);
                exit(1);
            }
            
            file_path
        },
        Err(_) => {
            info!("Fallback to /dns/mcsync.d/ because DNS_ZONE_DIR has not been specified.");
            String::from("/dns/mcsync.d/")
        }
    }
}

pub fn get_database_path() -> String {
    match std::env::var("DATABASE_PATH") {
        Ok(file_path) => {
            file_path
        },
        Err(_) => {
            info!(
                "Fallback to /database.json because DATABASE_PATH has not been specified."
            );
            String::from("/database.json")
        }
    }
}