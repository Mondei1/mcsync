use std::path::Path;
use std::process::exit;
use std::sync::{Arc, Mutex};

use actix_web::web::{Data, resource};
use dav_server::{DavHandler, DavConfig};
use dav_server::actix::{DavRequest, DavResponse};
use dav_server::fakels::FakeLs;
use dav_server::localfs::LocalFs;
use paris::{ error};
use serde::{Serialize, Deserialize};

use actix_web::{HttpServer, get, App, Responder, HttpRequest};
use crate::database::{Database, DatabaseSynced};
use crate::env::get_minecraft_save_path;

use super::cache::Cache;
use super::middleware::ClientSeenFactory;

pub struct HttpHandler {
    database: Database,
}

#[derive(Serialize, Deserialize)]
pub struct ReturnSync {
    sync: Vec<DatabaseSynced>
}

impl HttpHandler {
    pub async fn new(database: Database) -> Self {
        let saves_path = get_minecraft_save_path();
        let saves_dir = Path::new(&saves_path);
        if !saves_dir.exists() {
            error!("Minecraft saves directory doesn't exist. Please create directory at {}", saves_path);
            exit(1);
        }

        if !saves_dir.is_dir() {
            error!("MINECRAFT_SAVES points to a non-directory but is has to be a directory.");
            exit(1);
        }

        Self { database }
    }

    pub async fn listen(&self) {
        start(self.database.clone()).await;
    }
}

pub async fn dav_handler(req: DavRequest, davhandler: Data<DavHandler>) -> DavResponse {
    if let Some(prefix) = req.prefix() {
        let config = DavConfig::new().strip_prefix(prefix);
        davhandler.handle_with(config, req.request).await.into()
    } else {
        davhandler.handle(req.request).await.into()
    }
}

async fn start(db: Database) {
    let _ = HttpServer::new(move || {
        let db_clone = db.clone();
        let data = Arc::new(Mutex::new(Cache::new()));

        let dav_server = DavHandler::builder()
            .filesystem(LocalFs::new("/tmp", false, false, false))
            .locksystem(FakeLs::new())
            .build_handler();

        App::new()
            //.service(get_root)
            //.service(get_status)
            //.service(set_status)
            .service(resource("/dav/{tail:.*}").to(dav_handler))
            .app_data(Data::new(db_clone.clone()))
            .app_data(Data::new(dav_server.clone()))
            .wrap(ClientSeenFactory::new(db_clone))
    })
    .bind(("0.0.0.0", 8080))
    .unwrap()
    .run()
    .await;
}

#[get("/")]
async fn get_root(req: HttpRequest, db: Data<Database>) -> impl Responder {
    String::from("mcsync server v0.1")
}

#[get("/<sync>")]
async fn get_status(req: HttpRequest, cache: Data<Cache>, sync: String) -> impl Responder {
    format!("{}", cache.can_sync(req.connection_info().peer_addr().unwrap().to_string(), sync))
}

#[get("/<sync>/allow")]
async fn set_status(req: HttpRequest, cache: Data<Cache>, sync: String) -> impl Responder {
    // cache.borrow_mut().add_sync(req.connection_info().peer_addr().unwrap().to_string(), sync);

    String::new()
}


/*#[get("/sync")]
fn sync_list(db: State<Database>) -> Json<ReturnSync> {
    Json(ReturnSync { sync: db.get_syncs()} )
}*/
