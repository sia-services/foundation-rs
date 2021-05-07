mod loaders;

use std::collections::HashMap;
use serde::Serialize;

use crate::server::{get_connection, SimpleResult};

pub struct MetaInfo {
    schemas: HashMap<String,Schema>,
}

pub struct Schema {
    // name of schema allready in metainfo Map
    entities: HashMap<String, Entity>,
}

pub struct Entity {
    // name of entity allready in schema Map
    pub entity_type: EntityType,
    pub num_rows: Option<u32>,
    pub columns: Vec<Column>,
    pub primary_key: Option<Vec<usize>>, // positions of pk columns
    pub indexes:     Vec<TableIndex>
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum EntityType {
    #[serde(alias="table")]
    Table, 
    #[serde(alias="view")]
    View,
    #[serde(alias="temp")]
    Temporary
}

#[derive(Clone)]
pub struct Column {
    pub name: String,
    pub col_type: ColumnType,
    pub sql_type: oracle::sql_type::OracleType,
    pub col_size: u16, // in bytes
    pub nullable: bool
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ColumnType {
    #[serde(alias="int")]
    Integer, 
    #[serde(alias="string")]
    String,
    #[serde(alias="number")]
    Number,
    #[serde(alias="datetime")]
    DateTime,
    #[serde(alias="unsupported")]
    Unsupported,
}

#[derive(Debug)]
pub struct TableIndex {
    pub unique:  bool,
    pub columns: Vec<IndexColumn>
}

#[derive(Debug)]
pub struct IndexColumn {
    pub column_index: usize,
    pub desc: bool
}

pub fn load(others: &Option<crate::server::config::OthersConfig>) -> SimpleResult<MetaInfo> {
    // sleep for sinchronize log output
    std::thread::sleep(std::time::Duration::from_millis(10));
    println!();
    println!("READING METAINFO FROM ORACLE...");

    let start = chrono::offset::Local::now();

    let conn = get_connection()?;

    let available_schemas = loaders::load_available_schemas(&conn, others)?;

    let mut schemas = loaders::load_entities(&conn, &available_schemas)?;
    loaders::load_columns(&conn, &available_schemas, &mut schemas)?;
    loaders::load_primary_keys(&conn, &available_schemas, &mut schemas)?;
    loaders::load_indexes(&conn, &available_schemas, &mut schemas)?;

    let mut schemas_count = 0;
    let mut tables_count = 0;
    let mut columns_count = 0;
    let mut pks_count = 0;
    let mut indexes_count = 0;

    for (_,schema) in schemas.iter() {
        for (_,entity) in schema.entities_iter() {
            tables_count += 1;
            columns_count += entity.columns.len();
            indexes_count += entity.indexes.len();
                        
            if entity.primary_key.is_some() {
                pks_count += 1;
            }
            
        }
        schemas_count += 1;
    }

    println!();
    println!("TOTAL:   {} schemas with {} tables & views and {} columns", schemas_count,  tables_count, columns_count);
    println!("         {} tables with primary keys", pks_count);
    println!("         {} indexes found", indexes_count);

    let end = chrono::offset::Local::now();
    let duration = end - start;

    let seconds = duration.num_seconds();
    let milliseconds = duration.num_milliseconds() - seconds * 1000;

    println!();
    println!("ELAPSED: {} seconds, {} milliseconds", seconds, milliseconds);
    println!();
        
    Ok(MetaInfo{schemas})
}

impl MetaInfo {
    pub fn find_schema<'a,'s>(&'s self, name: &'a str) -> Option<&'s Schema> {
        self.schemas.get(name)
    }

    pub fn schema_names(&self) -> std::collections::hash_map::Keys<'_, String, Schema> {
        self.schemas.keys()
    }
    /*
    pub fn schemas_iter(&self) -> std::collections::hash_map::Iter<'_, String, Schema> {
        self.schemas.iter()
    }
    */
}

impl Schema {
    pub fn find_entity<'a,'s>(&'s self, name: &'a str) -> Option<&'s Entity> {
        self.entities.get(name)
    }

    pub fn entities_iter(&self) -> std::collections::hash_map::Iter<'_, String, Entity> {
        self.entities.iter()
    }
}

impl Column {
    fn null_value_repr() -> String {
        "null".to_string()
    }

    pub fn try_to_string(&self, rs: &oracle::Row, colidx: usize) -> String {
        match self.col_type {
            ColumnType::String => {
                let v: String = rs.get(colidx).unwrap_or("".to_owned());
                format!("\"{}\"",v)
            },
            ColumnType::Integer | ColumnType::Number => {
                if self.nullable {
                    let v: Option<String> = rs.get(colidx).unwrap();
                    match v {
                        None => Column::null_value_repr(),
                        Some (v) => v
                    }
                } else {
                    let v: String = rs.get(colidx).unwrap();
                    v
                }
            },            
            ColumnType::DateTime => {
                if self.nullable {
                    let v: Option<chrono::DateTime<chrono::Local>> = rs.get(colidx).unwrap();
                    match v {
                        None => Column::null_value_repr(),
                        Some (v) => format!("\"{}\"", v.to_rfc3339())
                    }
                } else {
                    let v: chrono::DateTime<chrono::Local> = rs.get(colidx).unwrap();
                    format!("\"{}\"", v.to_rfc3339())
                }
            }
            _ => "\"not-implemented\"".to_owned()
        }
    }
}