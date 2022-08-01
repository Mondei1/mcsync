use std::process::exit;

use paris::error;

use crate::config::Config;

pub struct Init {

}

impl Init {
    pub fn execute(config: Config, sync_name: String) {
        match config.get_sync_by_name(sync_name.as_str()) {
            Some(_) => {
                error!("There is already a sync with this name.");
                exit(1);
            },
            None => { }
        }

        
    }
}