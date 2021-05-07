use std::sync::RwLock;

use lazy_static::lazy_static;
use r2d2::{Pool,PooledConnection};
use r2d2_oracle::OracleConnectionManager;

pub type Datasource = Pool<OracleConnectionManager>;
pub type Connection = PooledConnection<OracleConnectionManager>;

type DatasourceHandler = RwLock<Option<Datasource>>;

lazy_static! {
  static ref DATASOURCE: DatasourceHandler = RwLock::new(None);
}

fn new_datasource(config: &super::config::DbConnection) -> Result<Datasource, String> {
    let user = &config.credentials.user;
    let pw = &config.credentials.pw;
    let manager = OracleConnectionManager::new(user, pw, &config.url);
    let pool = r2d2::Pool::builder()
            .max_size(15)
            .build(manager)
            .map_err(|err|format!("Build db {:?} connection pool err: {:?}", &config.url, err))?;

    Ok(pool)
}

pub fn create_datasource(config:&super::config::DbConnection) -> Result<(), String> {
    let mut ds = (*DATASOURCE).write()
        .map_err(|_err| format!("Can not get lock for datasource creation"))?;

    if let None = *ds {
        let datasource = new_datasource(config)?;
        *ds = Some(datasource);
    };

    Ok(())
}

pub fn get_connection() -> Result<Connection, String> {
    let ds = (*DATASOURCE).read().unwrap();
    let cc = ds.as_ref().unwrap();
    // oracle::connect(&cc.url, &cc.user, &cc.pw)
    cc.get().map_err(|err|format!("Connect to db err: {:?}", err))
}
