mod routines;
mod prerequisites;
mod platform;
mod config;

use std::{process::{Command, exit}};

use camino::Utf8PathBuf;
use cfg_if::cfg_if;

use clap::{Parser, Args, Subcommand};
use config::Config;
use nix::unistd::Uid;
use paris::{error, success, info};
use prerequisites::Prerequisites;
use routines::client_info::ClientInfo;

#[derive(Parser, Debug)]
#[clap(author = "Nicolas Klier aka Mondei1", version, about = "Tunnel & share your Minecraft server with friends.", long_about = None)]
struct App {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[clap(subcommand)]
    command: Action,
}

#[derive(Debug, Subcommand)]
enum Action {
    /// Connect to a already set up mcsync server.
    Connect,

    /// Disconnect from your current mcsync server.
    Disconnect,

    /// Print your client's public keys.
    ClientInfo,

    /// Print information about all game servers. Execute inside game server for more information.
    Status,

    /// Share game server with other members.
    Init
}

#[derive(Debug, Args)]
struct GlobalOpts {

    #[clap(default_value_t = platform::get_config_directory(), long, short, global = true)]
    config_file: Utf8PathBuf,
    
    #[clap(parse(from_occurrences), global = true, long, short)]
    /// Declare how much mcsync should talk.
    verbose: usize
}

fn main() {
    // Modules are installed and available. Next, we have to parse the command line.
    let args: App = App::parse();
    let conf = Config::new(args.global_opts.config_file.clone());
    conf.verify_integrity();

    if !is_root() {
        error!("mcsync requires root permissions to create a tunnel for you.");
        exit(0);
    }

    // Try to install WireGuard module.
    let setup = Prerequisites::new();
    setup.check();

    match args.command {
        Action::ClientInfo => {
            ClientInfo::new(conf);
        },
        _ => {
            error!("This command is not yet supported. Sorry :c");
            exit(0);
        }
    }
}

fn is_root() -> bool {
    cfg_if! {
        if #[cfg(unix)] {
            return Uid::effective().is_root();
        }
    }
}

