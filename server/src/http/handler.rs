use std::sync::{Arc, Mutex};

use paris::{info, success};
use rocket::{fairing::AdHoc, State, response::{content::Json, self}};
use serde::{Serialize, Deserialize};

use crate::database::{Database, DatabaseSynced};

pub struct HttpHandler {
    database: Arc<Mutex<Database>>,
}

#[derive(Serialize, Deserialize)]
pub struct ReturnSync {
    sync: Vec<DatabaseSynced>
}

impl HttpHandler {
    pub async fn new(database: Database) -> Self {
        Self { database: Arc::new(Mutex::new(database)) }
    }

    pub async fn listen(&self) {
        /*let log = warp::log::custom(|req| {
            let mut db_clone = self.database;

            let ip = req.remote_addr().unwrap().ip().to_string();
            db_clone.seen_client(ip.as_str());
            db_clone.flush();

            let client = db_clone.get_client_by_ip(&ip);

            info!("{} ({}) -- {} {}",
                ip,
                if client.is_some() { client.unwrap().name.clone() } else { String::from("") },
                req.method(),
                req.path()
            );
        });*/

        start(self.database.clone());

        success!("Listening on port 80 ...");
    }
}

fn start(db: Arc<Mutex<Database>>) {
    let db_clone = db.lock().unwrap().clone();

    rocket::ignite()
    .attach(AdHoc::on_request("Last seen & logging", move |req, _| {
        let mut db_clone = db.lock().unwrap();

        let ip = req.client_ip().unwrap().to_string();
        db_clone.seen_client(ip.as_str());
        db_clone.flush();

        let client = db_clone.get_client_by_ip(&ip);

        info!(
            "{} ({}) -- {} {}",
            ip,
            if client.is_some() {
                client.unwrap().name.clone()
            } else {
                String::from("")
            },
            req.method(),
            req.uri().path()
        );
    }))
    .manage(db_clone)
    .mount("/", routes![hello])
    .launch();
}

#[get("/")]
fn hello() -> &'static str {
    "ok"
}

/*#[get("/sync")]
fn sync_list(db: State<Database>) -> Json<ReturnSync> {
    Json(ReturnSync { sync: db.get_syncs()} )
}*/
