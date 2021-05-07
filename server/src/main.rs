mod application;
mod metainfo;
mod security;
mod server;

use std::io::{Error, ErrorKind};

use actix_web::{middleware, App, HttpServer};
use actix_web::http::ContentEncoding;
use actix_slog::StructuredLogger;

use slog::info;

// TODO: threadlocal: https://doc.rust-lang.org/std/macro.thread_local.html
// TODO: authorization: roles and privileges
// TODO: v1 query api - one request / one table
// TODO: v2 query api - one request / one select
// TODO: v3 query api - one request / scriptlet

// TODO: web interface for /mgmt

// rest api structure:
//   /mgmt            management
//       /health      health checking
//       /schemas     metadata-catalog
//   /api             web applications api
//       /v1/schemas  tables / views / procedures

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log = server::setup_logging();
    info!(log, "Starting Foundation Server");

    // configure server
    let config = server::load_config()
        .expect("Can not load config file");

    let db_config = &config.connection;
    server::create_datasource(db_config)
        .map_err(|e|Error::new(ErrorKind::Other, e))?;

    let application = application::ApplicationState::load(&config)
        .map_err(|e|Error::new(ErrorKind::Other, e))?;

    let http = &config.http;
    let builder = server::setup_tls(&http);    
    let identity_service = server::setup_identity(&config.jwt);

    let listen = &http.listen;
    let listen = format!("{}:{}", &listen.domain, &listen.port);
    info!(log, "Server Started on https://{}", &listen);

    HttpServer::new(move || {
        App::new()
            .data(application.clone())
            .wrap(StructuredLogger::new(log.clone()))
            .wrap(middleware::Compress::new(ContentEncoding::Br))
            .wrap(identity_service.clone())
            .service(application::base_scope())  
            .service(application::v1_api_scope())
    })
        .keep_alive(75)
        .bind_openssl(&listen, builder)?
        .run()
        .await
}
