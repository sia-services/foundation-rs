use std::collections::HashMap;
use std::sync::Arc;
use actix_web::{get, web, Responder, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::dev::HttpServiceFactory;
use serde::Deserialize;

use crate::application::{ApplicationState, v1query};
use crate::metainfo;

// group of endpoints for api
pub fn v1_api_scope() -> impl HttpServiceFactory {
    web::scope("/api")
        .wrap(crate::security::Authorized::all())
        .service(table_query_by_pk)
        .service(table_query_by_params)
}

#[get("/v1/{schema}/{table}/{pk}")]
async fn table_query_by_pk(path: web::Path<(String,String,String)>, data: web::Data<Arc<ApplicationState>>) -> impl Responder {
    let (schema_name,table_name, pk_params) = path.into_inner();

    println!("table_query_by_pk: {}.{}; pk: {}", &schema_name, &table_name, &pk_params);

    let metainfo = data.metainfo.read().unwrap();

    if let Some(info) = metainfo.find_schema(&schema_name) {
        if let Some(info) = info.find_entity(&table_name) {
            let pk_params: Vec<String> = pk_params.split(",").map(|s|s.to_string()).collect();

            let info = unsafe { 
                // hack: transmute lifetime
                // because we load metainfo once in startup
                let info: &'static metainfo::Entity = std::mem::transmute(info);
                info
            };

            let query = v1query::DynamicQuery::create_from_pk(&schema_name, &table_name, info, pk_params);
            return match query {
                Ok(query) => {
                    let result = web::block(move || query.fetch_one()).await;
                    match result {
                        Ok(result) => HttpResponse::Ok().set(ContentType::json()).body(result),
                        Err(e) => {
                            eprintln!("{:?}",e);
                            HttpResponse::InternalServerError().finish()
                        }
                    }
                },
                Err(err) => HttpResponse::BadRequest().body(err)
            };
        }
    };

    HttpResponse::NotFound().finish()
}

// for limit, offset etc, see: https://oracletutorial.com/oracle-basics/oracle-fetch

#[derive(Deserialize)]
struct QueryParams {
    q:      String,
    limit:  Option<u32>,
    offset: Option<u32>,
    order:  Option<String>,
}

#[get("/v1/{schema}/{table}/")]
async fn table_query_by_params(path: web::Path<(String,String)>, req: web::Query<QueryParams>, data: web::Data<Arc<ApplicationState>>) -> impl Responder {
    let (schema_name,table_name) = path.into_inner();

    println!("table_query_by_params: {}.{}", &schema_name, &table_name);

    let metainfo = data.metainfo.read().unwrap();

    if let Some(info) = metainfo.find_schema(&schema_name) {
        if let Some(info) = info.find_entity(&table_name) {
            println!("table_query_by_params, q: {}", req.q);

            let q: serde_json::error::Result<HashMap<String,String>> = serde_json::from_str(&req.q);
            return match q {
                Ok(paremeters) => {
                    let order: Vec<String> = req.order.as_ref().map(|s|s.split(",").map(|s|s.to_string()).collect()).unwrap_or(vec![]);

                    let info = unsafe { 
                        // hack: transmute lifetime
                        // because we load metainfo once in startup
                        let info: &'static metainfo::Entity = std::mem::transmute(info);
                        info
                    };
        
                    let query = v1query::DynamicQuery::create_from_params(&schema_name, &table_name, info, paremeters, order, req.limit, req.offset);
                    return match query {
                        Ok(query) => {
                            let result = web::block(move || query.fetch_many()).await;
                            match result {
                                Ok(result) => HttpResponse::Ok().set(ContentType::json()).body(result),
                                Err(e) => {
                                    eprintln!("{:?}",e);
                                    HttpResponse::InternalServerError().finish()
                                }
                            }
                        },
                        Err(err) => HttpResponse::BadRequest().body(err)
                    };
                },
                Err(err) => HttpResponse::BadRequest().body(format!("Invalid query format: {}", err))
            };
        }
    };

    HttpResponse::NotFound().finish()
}
