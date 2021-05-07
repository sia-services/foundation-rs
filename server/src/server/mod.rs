pub mod config;
mod datasource;
mod setup;

pub use setup::setup_logging;
pub use setup::setup_tls;
pub use setup::setup_identity;
pub use self::config::load_config;

pub use datasource::{
    create_datasource,
    get_connection,
    Connection
};

pub type SimpleResult<T> = Result<T, String>;