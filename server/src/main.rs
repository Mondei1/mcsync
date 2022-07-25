#![feature(decl_macro)]

#[macro_use]
extern crate nickel;

mod http;
mod routines;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

use bollard::container::{ListContainersOptions, InspectContainerOptions, StopContainerOptions};
use lazy_static::lazy_static;
use log::{info, warn};
use routines::accept::Accept;

use paris::{error, Logger};
use serde::Deserialize;

use bollard::Docker;

lazy_static! {
    pub static ref FILE_DNS: String = String::new();
    pub static ref FILE_VPN: String = String::new();
    pub static ref DOCKER_PATH: String = match std::env::var("DOCKER_SOCKET") {
        Ok(d) => format!("unix://{}", d),
        Err(_) => {
            warn!(
                "Fallback to /var/run/docker.socket because DOCKER_SOCKET has not been specified."
            );
            String::from("unix:///var/run/docker.sock")
        }
    };
}

#[derive(Debug, Deserialize)]
struct DockerVersion {
    ServerVersion: String,
}

struct PingHandshake {
    version: usize,
    server_address: String,
    server_port: u8,
    next_state: usize,
}

#[tokio::main]
async fn main() {
    let mut current_host: String = String::new();
    let mut log = Logger::new();

    // No one is hosting right now.
    if current_host == "" {}

    let docker = match Docker::connect_with_socket_defaults() {
        Ok(d) => d,
        Err(error) => {
            error!("Unable to connect with Docker: {}", error);
            std::process::exit(1);
        }
    };

    let mut list_container_filters = HashMap::new();
    list_container_filters.insert("status", vec!["running"]);

    let containers = &docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters: list_container_filters,
            ..Default::default()
        }))
        .await
        .unwrap();

    for container in containers {
        let detail = docker
            .inspect_container(
                container.id.as_ref().unwrap(),
                None::<InspectContainerOptions>,
            )
            .await
            .unwrap();
        
        let ip_address = match detail.network_settings {
            None => {
                warn!("Couldn't retrive information about {}", container.id.as_ref().unwrap());
                continue;
            },
            Some(settings) => {
                match settings.networks {
                    None => {
                        warn!("Couldn't retrive Docker networks from {}", container.id.as_ref().unwrap());
                        continue;
                    },
                    Some(networks) => {
                        match networks.get("mcsync") {
                            None => {
                                warn!("Couldn't find network \"mcsync\" for container {}", container.id.as_ref().unwrap());
                                continue;
                            },
                            Some(network_detail) => {
                                network_detail.ip_address.clone().unwrap()
                            }
                        }
                    }
                }
            }
        };

        if detail.name.as_ref().unwrap() == "/mcsync-dns" {
            let stop = docker.stop_container(detail.id.as_ref().unwrap(), Some(StopContainerOptions {
                ..Default::default()
            })).await;

            match stop {
                Ok(_) => {
                    info!("DNS server stopped.");
                },
                Err(error) => {
                    error!("Couldn't stop DNS server: {}", error);
                }
            }
        }

        println!("{}: {}", detail.name.unwrap(), ip_address);
    }

    // success!("Connected with Docker (version {})", serde_json::from_str::<DockerVersion>(docker_client.get_version_info().unwrap().as_str()).unwrap().ServerVersion);

    let args: Vec<String> = std::env::args().collect();
    let subroutine = args.get(1);

    if subroutine.is_some() {
        match subroutine.unwrap().as_str() {
            "accept" => {
                Accept::execute();
            }
            _ => {
                log.error("Unknown argument");
            }
        }
    } else {
        log.log("Run mcsync' server version 0.1.0-DEV");

        let http_server = http::handler::HttpHandler::new();
        http_server.listen();

        fake_status_server();
    }
}

fn fake_status_server() {
    thread::spawn(move || {
        let listener = TcpListener::bind("0.0.0.0:25565").unwrap();
        // accept connections and process them, spawning a new thread for each one
        println!("Server listening on port 3333");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    thread::spawn(move || {
                        // connection succeeded
                        handle_client(stream)
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);
                    /* connection failed */
                }
            }
        }
        // close the socket server
        drop(listener);
    })
    .join()
    .unwrap();
}

fn handle_client(mut stream: TcpStream) {
    let mut data = [0 as u8; 50]; // using 50 byte buffer
    let mut count = 0;
    while match stream.read(&mut data) {
        Ok(size) => {
            // echo everything!
            println!("Received: {:?}", &data[0..size]);
            stream.write(&data[0..size]).unwrap();
            true
        }
        Err(_) => {
            println!(
                "An error occurred, terminating connection with {}",
                stream.peer_addr().unwrap()
            );
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {
        if count > 5 {
            break;
        }
        count += 1;
    }
}
