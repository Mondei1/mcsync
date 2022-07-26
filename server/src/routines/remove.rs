use paris::{error, success};

use crate::database::Database;

pub struct RemoveUser<'a> {
    database: &'a mut Database,
}

impl<'a> RemoveUser<'a> {
    pub fn new(database: &'a mut Database) -> Self {
        Self { database }
    }

    pub fn execute(&mut self) {
        let args: Vec<String> = std::env::args().collect();
        let client_name = match args.get(2) {
            Some(cn) => cn.to_owned(),
            None => {
                error!("No username specified!");
                return;
            }
        };

        match self.database.remove_client(&client_name) {
            Some(client) => {
                success!("Removed {} ({})", client.name, client.ipv4_address);
            },
            None => {
                error!("Cannot find user \"{}\"", client_name);
            }
        }
    }
}