mod routines;
mod prerequisites;
mod platform;
mod config;
mod sync;
mod utils;

use std::process::exit;

use camino::Utf8PathBuf;
use cfg_if::cfg_if;

use clap::{Parser, Args, Subcommand};
use config::Config;
use nix::unistd::Uid;
use paris::error;
use prerequisites::Prerequisites;
use platform::permission_check;
use routines::{client_info::ClientInfo, import::Import, connect::Connect, disconnect::Disconnect, init::Init};

#[derive(Parser, Debug)]
#[clap(author = "Nicolas Klier aka Mondei1", version, about = "Tunnel & share your Minecraft server with friends.", long_about = None)]
struct App {
    #[clap(default_value_t = platform::get_config_directory(), long, short, global = true)]
    config_file: Utf8PathBuf,
    
    #[clap(default_value_t = false, global = true, long, short)]
    /// mcsync will become very chatty.
    verbose: bool,

    #[clap(default_value_t = false, global = true, long, short)]
    // mcsync will directly connect to the backend without WireGuard.
    local: bool,

    #[clap(subcommand)]
    command: Action,
}

#[derive(Debug, Subcommand)]
enum Action {
    /// Execute this command first. It will guide though the post-install 
    Install,

    /// Connect to a already set up mcsync server.
    Connect {
        // Name of server to connect with.
        name: String
    },

    /// Disconnect from your current mcsync server.
    Disconnect,

    /// Print your client's public keys.
    ClientInfo,

    /// Import a server (usually using .mcss files)
    Import {
        /// Custom name for the new server.
        name: String,

        /// Location of .mcss file
        path: Utf8PathBuf
    },

    /// Print information about all game servers. Execute inside game server for more information.
    Status,

    /// Share game server with other members.
    Init {
        /// Name you want to give to your server.
        name: String,

        /// Path to a some script file (.sh/.bash) or to a .jar file.
        start_file: Utf8PathBuf
    }
}

#[tokio::main]
async fn main() {
    // Modules are installed and available. Next, we have to parse the command line.
    let args: App = App::parse();
    let mut conf = Config::new(args.config_file.clone());
    conf.verify_integrity();

    match args.command {
        Action::Install => {
            let setup = Prerequisites::new();
            setup.check().await;
        }
        Action::ClientInfo => {
            let ci = ClientInfo::new(conf);
            ci.print();
        },
        Action::Import { path, name } => {
            Import::execute(conf, name, path);
        },
        Action::Connect { name } => {
            if !args.local {
                permission_check();
            }
            
            let server = conf.get_server_by_name(&name);
            if server.is_none() {
                error!("Server {} doesn't exist.", name);
                exit(1);
            }

            Connect::execute(conf, server.unwrap(), !args.local).await;
        },
        Action::Disconnect => {
            permission_check();
            
            Disconnect::execute();
        },
        Action::Init { name, start_file } => {
            Init::execute(conf, name, start_file).await;
        }
        _ => {
            error!("This command is not yet supported. Sorry :c");
            exit(0);
        }
    }
}

fn is_root() -> bool {
    cfg_if! {
        if #[cfg(unix)] {
            Uid::effective().is_root()
        }
    }
}

