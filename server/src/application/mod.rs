mod metaapi;
mod v1api;
mod v1query;

use std::sync::{Arc, RwLock};

use actix_web::{get, web, Responder, Scope};

use crate::metainfo::{self, MetaInfo};
use crate::server::{self, config};

pub use v1api::v1_api_scope;

// This struct represents state
pub struct ApplicationState {
    metainfo: RwLock<MetaInfo>
}

impl ApplicationState {
    pub fn load(config: &config::ServerConfig) -> server::SimpleResult<Arc<ApplicationState>> {
        let metainfo = metainfo::load(&config.others)?;
        let metainfo = RwLock::new(metainfo);
        Ok( Arc::new(ApplicationState{metainfo}) )
    }
}

// group of base endpoints
pub fn base_scope() -> Scope {
    web::scope("/mgmt")
        .service(health)
        .service(metaapi::metainfo_scope())
        /*
        .service(fs::Files::new("/", "./www")
            .show_files_listing()
            .use_last_modified(true))
        .default_service(web::resource("").route(web::get().to(index)))
         */
}
/*
async fn index() -> Result<NamedFile> {
    let path: PathBuf = "./www/index.html".parse().unwrap();
    Ok(NamedFile::open(path)?)
}
*/
#[get("/health")]
async fn health() -> impl Responder {
    "OK".to_string()
}
