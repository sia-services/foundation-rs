use std::collections::HashSet;
use std::iter::FromIterator;
use std::sync::Arc;

use actix_web::dev::HttpServiceFactory;
use actix_web::{get, web, HttpResponse, Responder};

use serde::Serialize;

use crate::metainfo::{ColumnType,EntityType};
use super::ApplicationState;

// https://github.com/foundation-rs/backend/blob/master/server/src/application/mgmt_scope.rs

// group of endpoints for metainfo
pub fn metainfo_scope() -> impl HttpServiceFactory {
    web::scope("/schemas")
        .wrap(crate::security::Authorized::developers())
        .service(schemas_metainfo)
        .service(tables_metainfo)
        .service(table_metainfo)
}

#[derive(Serialize)]
struct DatabaseMetainfo<'a> {
    schemas: Vec<&'a str>
}

#[derive(Serialize)]
struct SchemaMetainfo<'a> {
    tables: Vec<TableMetaInfoBrief<'a>>
}

#[derive(Serialize)]
struct TableMetaInfoBrief<'a> {
    name:      &'a str,
    #[serde(alias="type")]
    entity_type: EntityType,
    has_pk:    bool
}

#[derive(Serialize)]
struct TableMetaInfo<'a> {
    name:      &'a str,
    #[serde(alias="type")]
    entity_type: EntityType,
    has_pk:    bool,
    columns:   Vec<ColumnMetaInfo<'a>>
}

#[derive(Serialize)]
pub struct ColumnMetaInfo<'a> {
    pub name:     &'a str,
    #[serde(alias="type")]
    pub col_type: ColumnType,
    pub is_pk:    bool,
    pub nullable: bool
}

#[get("/")]
async fn schemas_metainfo(data: web::Data<Arc<ApplicationState>>) -> impl Responder {
    let metainfo = data.metainfo.read().unwrap();
    let mut schemas: Vec<&str> = metainfo.schema_names().map(|s|s.as_str()).collect();
    schemas.sort();
    let response = DatabaseMetainfo { schemas };
    HttpResponse::Ok().json(response)
}

#[get("/{schema}")]
async fn tables_metainfo(path: web::Path<(String,)>, data: web::Data<Arc<ApplicationState>>) -> impl Responder {
    let schema_name = path.into_inner().0;
    let metainfo = data.metainfo.read().unwrap();

    // match metainfo.schemas.get(schema_name.as_str()) {
    match metainfo.find_schema(&schema_name) {
        Some(info) => {
            let mut tables: Vec<TableMetaInfoBrief> = info.entities_iter().map(|(name, info)|
                TableMetaInfoBrief {
                    name,
                    entity_type: info.entity_type,
                    has_pk: info.primary_key.is_some()
                }).collect();
            tables.sort_by(|a,b|a.name.cmp(b.name));

            HttpResponse::Ok().json(SchemaMetainfo { tables })
        },
        None => HttpResponse::NotFound().finish()
    }
}

#[get("/{schema}/{table}")]
async fn table_metainfo(path: web::Path<(String,String)>, data: web::Data<Arc<ApplicationState>>) -> impl Responder {
    let (schema_name,table_name) = path.into_inner();
    let metainfo = data.metainfo.read().unwrap();

    if let Some(info) = metainfo.find_schema(&schema_name) {
        if let Some(info) = info.find_entity(&table_name) {
            let pk_indices = match &info.primary_key {
                Some(pk) => {
                    HashSet::from_iter(pk)
                }, None => {
                      HashSet::new()
                  }
              };

            let columns = info
                .columns
                .iter()
                .enumerate()
                .map(|(ref i, c)| {
                    let is_pk = pk_indices.contains(i);
                    ColumnMetaInfo { 
                        name: c.name.as_str(), 
                        col_type: c.col_type, 
                        is_pk, 
                        nullable: c.nullable}
                }).collect();

            let response = TableMetaInfo {
                name: &table_name,
                entity_type: info.entity_type,
                has_pk: pk_indices.len() > 0,
                columns
            };
            return HttpResponse::Ok().json(response)
        }
    };

    HttpResponse::NotFound().finish()
}