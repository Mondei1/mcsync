use std::path::Path;
use std::process::exit;
use std::str::FromStr;
use std::{fs::File, io::Write, fs::remove_file, time::Duration};

use domain::base::iana::Class;
use domain::base::{Dname, Rtype};
use domain::base::octets::Octets512;
use domain::rdata::A;
use domain::resolv::{StubResolver, stub::conf::{ResolvConf, ResolvOptions, ServerConf}};
use log::{warn};
use paris::{error, info, success, log};

use crate::env;
use crate::{docker::DockerManager};

pub struct DNSManager {
    docker_instance: DockerManager,
    zone_dir: String,
    dns_server: String
}

impl DNSManager {
    pub fn new(docker_instance: DockerManager) -> Self {
        let zone_dir = env::get_dns_zone_dir();

        Self {
            docker_instance,
            zone_dir,
            dns_server: String::new()
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

    pub async fn query(&self, target: &str) -> Option<String> {
        if self.dns_server == "" {
            return None;
        }

        let dns_address = format!("{}:53", self.dns_server).parse().unwrap();

        let resolver = StubResolver::from_conf(ResolvConf {
            options: ResolvOptions {
                ..Default::default()
            },
            servers: vec![
                ServerConf {
                    addr: dns_address,
                    transport: domain::resolv::stub::conf::Transport::Udp,
                    request_timeout: Duration::from_millis(500),
                    recv_size: 1232,
                    udp_payload_size: 512
                }
            ]
        });

        let dname: Dname<Octets512> = Dname::from_str(&format!("{}.mc", target)).unwrap();
        let resolution = resolver.query((dname.clone(), Rtype::A, Class::In)).await;

        match resolution {
            Ok(lookup) => {
                
                match lookup.answer() {
                    Ok(answer) => {
                        for record in answer.limit_to::<A>() {
                            let ip = record.unwrap().data().to_owned();

                            return Some(ip.addr().to_string());
                        }

                        None
                    },
                    Err(_) => {
                        None
                    }
                }
            },
            Err(_) => {
                println!("Error on DNS resolution");
                None
            }
        }
    }

    // Setting `service_domain` to true means you explicitly state that you want to set reserved domains.
    pub async fn set_or_update_record(&self, name: &str, new_ip: &str, service_domain: bool) -> Option<()> {
        if !service_domain {
            if name == "backend" {
                warn!("Cannot create game server domain for reserved names like \"backend.mc\".");
                return None;
            }
        }

        // Check if DNS already resolves with the desired IP.
        self.query(name).await;

        let record = format!(
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

    // Once called, domains "backend.mc" will be set.
    // Those names are reservered and cannot be created by users.
    pub async fn setup_service_domains(&mut self) {
        match self.docker_instance.get_dns_container().await {
            Some(dns) => {
                let dns_ip = self.docker_instance
                    .get_container_ip(dns)
                    .await
                    .unwrap();
    
                self.set_or_update_record("dns", &dns_ip, true).await;
                self.dns_server = dns_ip;
            },
            None => {
                error!("Cannot find DNS container. Did you rename your containers? The name has to contain \"dns\" and \"mcsync\" somewhere e.g. \"mcsync-dns-1\".");
                return;
            }
        };

        match self.docker_instance.get_self().await {
            Some(server) => {
                let own_ip = self.docker_instance
                    .get_container_ip(server)
                    .await
                    .unwrap();
    
                    self.set_or_update_record("backend", &own_ip, true).await;
                },
            None => {
                error!("Cannot find own container. Did you rename your containers? The name has to contain \"backend\" and \"mcsync\" somewhere e.g. \"mcsync-backend-1\".");
                return;
            }
        }

        self.restart_dns().await;
    }
}
