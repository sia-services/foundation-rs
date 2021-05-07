use std::path::Path;

use openssl::ssl::{SslAcceptorBuilder, SslAcceptor, SslFiletype, SslMethod};

use slog_async;
use slog_term;
use slog::{Drain,o};

use super::config;

/// setup logging
// TODO: use logger everywhere
// TODO: async logger:  https://github.com/zupzup/rust-web-example/blob/main/src/logging/mod.rs
// TODO: actix example: https://www.zupzup.org/rust-webapp/index.html
// TODO: see also:      https://github.com/zupzup/rust-web-example/blob/main/src/handlers/mod.rs
// TODO: see also:      https://rust.graystorm.com/2019/07/20/better-logging-for-the-web-application/
pub fn setup_logging() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!())
}

/// load ssl keys
// to create a self-signed temporary cert for testing:
// `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
pub fn setup_tls(settings: &config::HttpListener) -> SslAcceptorBuilder {
    let certfilepath = Path::new(&settings.tls_cert);
    let keyfilepath = Path::new(&settings.tls_key);

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    builder
        .set_private_key_file(keyfilepath, SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file(certfilepath).unwrap();
    builder
}


pub fn setup_identity(settings: &config::JwtConfig) -> crate::security::IdentityService {
    crate::security::IdentityService::new(
        settings.issuer.to_string(), 
        Path::new(&settings.public_key).to_path_buf()
    )
}