#![feature(decl_macro)]
#![feature(iter_advance_by)]

#[macro_use]
extern crate nickel;

mod database;
mod dns;
mod docker;
mod http;
mod routines;

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::exit;
use std::thread;

use database::Database;
use dns::DNSManager;
use docker::DockerManager;

use lazy_static::lazy_static;
use routines::accept::Accept;
use routines::remove::RemoveUser;

use paris::{error, info};
use shadow_rs::{shadow, Format};

lazy_static! {
    pub static ref FILE_VPN: String = String::new();
}

shadow!(build);

#[tokio::main]
async fn main() {
    info!(
        "Run mcsync server version {} ({})",
        build::PKG_VERSION,
        if shadow_rs::is_debug() {
            "DEBUG"
        } else {
            "PROD"
        }
    );
    info!(
        "Compiled {} using branch {} ({})",
        shadow_rs::DateTime::now().human_format(),
        shadow_rs::branch(),
        build::SHORT_COMMIT
    );

    //let signals = Signals::new(&[SIGTERM, SIGINT]);

    let mut database = Database::new();
    database.flush();

    let docker_manager = DockerManager::new().await;
    let dns_manager = DNSManager::new(docker_manager.clone());

    /*if signals.is_ok() {
        thread::spawn(move || async move {
            for sig in signals.unwrap().forever() {
                info!("Goodbye!");

                std::process::exit(sig);
            }
        });
    } else {
        error!("Couldn't create signal catcher: {}", signals.unwrap_err());
    }*/

    let own_ip = docker_manager
        .get_container_ip(docker_manager.get_dns_container().await.unwrap())
        .await
        .unwrap();

    println!("DNS IP is {}", own_ip);

    let args: Vec<String> = std::env::args().collect();
    let subroutine = args.get(1);

    dns_manager.set_or_update_record("testworld", "192.168.10.24");
    dns_manager.restart_dns().await;

    if subroutine.is_some() {
        match subroutine.unwrap().to_lowercase().as_str() {
            "accept" => {
                Accept::new(&mut database).execute();
            }
            "remove" => {
                RemoveUser::new(&mut database).execute();
            }
            _ => {
                error!("Unknown argument");
            }
        }

        database.flush();

        exit(0);
    }

    let http_server = http::handler::HttpHandler::new(&mut database);
    http_server.listen();

    // fake_status_server();
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