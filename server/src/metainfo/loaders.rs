use std::collections::HashMap;
use itertools::Itertools;
use oracle::{self,StmtParam, sql_type::OracleType};

use oracle_derive::RowValue;

use super::{
    Column, 
    ColumnType, 
    Entity, 
    EntityType, 
    IndexColumn,
    TableIndex,
    Schema
};
use crate::server::{Connection, SimpleResult};

pub fn load_available_schemas(
    conn: &Connection,
    others: &Option<crate::server::config::OthersConfig>,
) -> SimpleResult<String> {
    let sql = "SELECT USERNAME FROM SYS.ALL_USERS WHERE ORACLE_MAINTAINED = 'N'";
    let rows = conn
        .query_as::<String>(sql, &[])
        .map_err(|err| format!("query available users err: {:?}", err))?;

    let mut owners = Vec::<String>::with_capacity(16);

    for row_result in rows {
        let owner = row_result.map_err(|err| format!("fetch available users err: {:?}", err))?;

        let mut to_push = true;

        if let Some(others) = others {
            to_push = others.excludes.iter().position(|s| s == &owner).is_none();
        }
        if to_push {
            owners.push(format!("'{}'", owner));
        }
    }

    Ok(owners.join(","))
}

#[derive(RowValue)]
struct OraTable {
    owner: String,
    table_name: String,
    table_type: String,
    num_rows: Option<u32>,
    temporary: String,
}

pub fn load_entities(
    conn: &Connection,
    available_schemas: &str,
) -> SimpleResult<HashMap<String, Schema>> {
    let sql = format!(
        "SELECT OWNER, TABLE_NAME, TABLE_TYPE, NUM_ROWS, TEMPORARY FROM (
        SELECT OWNER, TABLE_NAME, 'TABLE' AS TABLE_TYPE, NUM_ROWS, TEMPORARY
        FROM SYS.ALL_TABLES
        UNION
        SELECT OWNER, VIEW_NAME, 'VIEW' AS TABLE_TYPE, 0, 'N'
        FROM SYS.ALL_VIEWS
        ) WHERE OWNER IN ( {} )
        ORDER BY OWNER, TABLE_TYPE, TABLE_NAME",
        available_schemas
    );
    let mut stmt = conn.prepare(&sql, &[StmtParam::FetchArraySize(1000)])
        .map_err(|err| format!("prepare stmt for tables and viws err: {:?}", err))?;
    let rows = stmt
        .query_as::<OraTable>(&[])
        .map_err(|err| format!("query tables and viws err: {:?}", err))?;

    let mut schemas = HashMap::with_capacity(64);

    let grouped_entities = rows.filter_map(|r| r.ok()).group_by(|t| t.owner.clone());
    for (owner, row_result) in grouped_entities.into_iter() {
        let mut entities = HashMap::with_capacity(64);

        for t in row_result.into_iter() {
            let table_name = t.table_name.to_lowercase();

            let entity_type = match &t.table_type as &str {
                "TABLE" => {
                    if t.temporary == "Y" {
                        EntityType::Temporary
                    } else {
                        EntityType::Table
                    }
                }
                "VIEW" => EntityType::View,
                _ => panic!("unspecified entity type"),
            };
    
            let columns = Vec::new();
            let primary_key = Option::None;
            let indexes = Vec::new();
            entities.insert(
                table_name,
                Entity {
                    entity_type,
                    columns,
                    num_rows: t.num_rows,
                    primary_key,
                    indexes
                },
            );
        }

        schemas.insert(owner.to_lowercase(), Schema { entities });
    }

    Ok(schemas)
}

#[derive(RowValue)]
struct OraColumn {
    owner: String,
    table_name: String,
    column_name: String,
    data_type: String,
    data_length: u16,
    data_precision: Option<u8>,
    data_scale: Option<i8>,
    nullable: String
}

pub fn load_columns(
    conn: &Connection,
    available_schemas: &str,
    metainfo: &mut HashMap<String, Schema>,
) -> SimpleResult<()> {
    let sql = format!(
        "SELECT OWNER, TABLE_NAME, COLUMN_NAME, DATA_TYPE, DATA_LENGTH, DATA_PRECISION, DATA_SCALE, NULLABLE \
        FROM SYS.ALL_TAB_COLUMNS WHERE OWNER IN ( {} ) ORDER BY OWNER, TABLE_NAME, COLUMN_ID"
        ,available_schemas
    );

    let mut stmt = conn.prepare(&sql, &[StmtParam::FetchArraySize(10000)])
        .map_err(|err| format!("prepare stmt for columns err: {:?}", err))?;

    let rows = stmt
        .query_as::<OraColumn>(&[])
        .map_err(|err| format!("query columns err: {:?}", err))?;

    // group columns by schema
    let grouped_columns = rows.filter_map(|r| r.ok()).group_by(|t| t.owner.clone());

    for (owner, row_result) in grouped_columns.into_iter() {
        let schema_name = owner.to_lowercase();
        let schema = metainfo.get_mut(&schema_name);
        if let Some(schema) = schema {
            // group by table_name
            let grouped_columns = row_result.group_by(|t| t.table_name.clone());

            for (table_name, columns) in grouped_columns.into_iter() {
                let table_name = table_name.to_lowercase();
                let entity = schema.entities.get_mut(&table_name);
                if let Some(entity) = entity {
                    for c in columns
                    {
                        let name = c.column_name.to_lowercase();
                        let nullable = c.nullable == "Y";

                        let (col_type, col_size, sql_type) = match &c.data_type as &str {
                            "CHAR" | "VARCHAR2" => (
                                ColumnType::String, 
                                c.data_length, 
                                OracleType::Varchar2(c.data_length.into())
                            ),
                            "LONG" => (
                                ColumnType::String, 
                                4096,
                                OracleType::Long
                            ),
                            "DATE" => (
                                ColumnType::DateTime,
                                8,
                                OracleType::Date
                            ),
                            "NUMBER" => {
                                let p = c.data_precision.unwrap_or_default();
                                let s = c.data_scale.unwrap_or_default();
                                let ora_type = OracleType::Number(p,s);
                                if s == 0 {
                                    if p == 0 || p > 7 {
                                        (ColumnType::Integer, 8, ora_type) // int 64
                                    } else if p > 4 {
                                        (ColumnType::Integer, 4, ora_type) // int 32
                                    } else {
                                        (ColumnType::Integer, 2, ora_type) // int 16
                                    }
                                } else {
                                    (ColumnType::Number, 8, ora_type) // float 64
                                }
                            }
                            _ => {
                                // Unsupported
                                (
                                    ColumnType::Unsupported, 
                                    0,
                                    OracleType::UInt64 // fictive type
                                )
                            }
                        };

                        entity.columns.push(Column {
                            name,
                            col_type,
                            sql_type,
                            col_size,
                            nullable,
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(RowValue)]
struct OraPrimaryKey {
    owner: String,
    table_name: String,
    constraint_name: String,
    column_name: String,
}

pub fn load_primary_keys(
    conn: &Connection,
    available_schemas: &str,
    metainfo: &mut HashMap<String, Schema>,
) -> SimpleResult<()> {
    let sql = format!(
        "SELECT C.OWNER, C.TABLE_NAME, C.CONSTRAINT_NAME, CC.COLUMN_NAME \
        FROM SYS.ALL_CONSTRAINTS C \
        JOIN SYS.ALL_CONS_COLUMNS CC ON C.OWNER = CC.OWNER AND C.TABLE_NAME = CC.TABLE_NAME AND C.CONSTRAINT_NAME = CC.CONSTRAINT_NAME
        WHERE C.OWNER IN ( {} ) AND C.CONSTRAINT_TYPE = 'P' AND C.STATUS = 'ENABLED'
        ORDER BY C.OWNER, C.TABLE_NAME, C.CONSTRAINT_NAME, CC.POSITION"
        ,available_schemas
    );

    let mut stmt = conn.prepare(&sql, &[StmtParam::FetchArraySize(1000)])
        .map_err(|err| format!("prepare stmt for primary keys err: {:?}", err))?;

    let rows = stmt
        .query_as::<OraPrimaryKey>(&[])
        .map_err(|err| format!("query primary keys err: {:?}", err))?;

    // group primary keys by schema
    let grouped_keys = rows.filter_map(|r| r.ok()).group_by(|t| t.owner.clone());

    for (owner, row_result) in grouped_keys.into_iter() {
        let schema_name = owner.to_lowercase();
        let schema = metainfo.get_mut(&schema_name);
        if let Some(schema) = schema {
            // group by table_name and constraint name
            let grouped_keys = row_result.group_by(|t| (t.table_name.clone(), t.constraint_name.clone()));
            for ((table_name, _), key_columns) in grouped_keys.into_iter() {
                let table_name = table_name.to_lowercase();
                let entity = schema.entities.get_mut(&table_name);
                if let Some(entity) = entity {
                    let column_indices: Vec<usize> = key_columns
                        .map(|c| {
                            let column_name = c.column_name.to_lowercase();
                            entity.columns.iter().position(|c| c.name == column_name)
                        })
                        .filter_map(|p| p)
                        .collect();
                    entity.primary_key.replace(column_indices);
                }
            }
        };
    }

    Ok(())
}

#[derive(RowValue)]
struct OraIndex {
    owner: String,
    table_name: String,
    index_name: String,
    uniqueness: String,
    column_name: String,
    descend: String,
}

pub fn load_indexes(
    conn: &Connection,
    available_schemas: &str,
    metainfo: &mut HashMap<String, Schema>,
) -> SimpleResult<()> {
    let sql = format!(
        "SELECT C.TABLE_OWNER, C.TABLE_NAME, C.INDEX_NAME, C.UNIQUENESS, CC.COLUMN_NAME, CC.DESCEND \
        FROM SYS.ALL_INDEXES C \
        JOIN SYS.ALL_IND_COLUMNS CC ON C.TABLE_OWNER = CC.INDEX_OWNER AND C.INDEX_NAME = CC.INDEX_NAME
        WHERE C.OWNER IN ( {} ) AND C.STATUS = 'VALID'
        ORDER BY C.TABLE_OWNER, C.TABLE_NAME, C.INDEX_NAME, CC.COLUMN_POSITION"
        ,available_schemas
    );

    let mut stmt = conn.prepare(&sql, &[StmtParam::FetchArraySize(1000)])
        .map_err(|err| format!("prepare stmt for indexes err: {:?}", err))?;

    let rows = stmt
        .query_as::<OraIndex>(&[])
        .map_err(|err| format!("query indexes err: {:?}", err))?;

    // group indexes by schema
    let grouped_indexes = rows.filter_map(|r| r.ok()).group_by(|t| t.owner.clone());

    for (owner, row_result) in grouped_indexes.into_iter() {
        let schema_name = owner.to_lowercase();
        let schema = metainfo.get_mut(&schema_name);
        if let Some(schema) = schema {
            // group by table_name, index name and uniques
            let grouped_indexes = row_result.group_by(|t| (t.table_name.clone(), t.index_name.clone(), t.uniqueness.clone()));
            for ((table_name, _,uniqueness), key_columns) in grouped_indexes.into_iter() {
                let table_name = table_name.to_lowercase();
                let entity = schema.entities.get_mut(&table_name);
                if let Some(entity) = entity {

                    let columns: Vec<IndexColumn> = key_columns.map(|c| {
                        let column_name = c.column_name.to_lowercase();
                        entity
                            .columns
                            .iter()
                            .position(|c|c.name == column_name)
                            .map(|column_index| IndexColumn{column_index, desc: c.descend != "ACC"} )
                    })
                        .filter_map(|p|p)
                        .collect();

                    if columns.len() > 0 {
                        let index = TableIndex {unique: uniqueness == "UNIQUE", columns};
                        entity.indexes.push(index);
                    }
                }
            }
        };
    }

    Ok(())
}