use std::collections::HashMap;
use itertools::Itertools;

use oracle;

use crate::metainfo;
use crate::server::get_connection;

pub struct DynamicQuery {
    sql: String,
    fetch_array_size: u32,
    columns: &'static Vec<metainfo::Column>, // hack because we load metainfo once in startup
    params: Vec<Parameter>,
}

struct Parameter {
    pub column: &'static metainfo::Column, // hack because we load metainfo once in startup
    pub value:  ParameterValue
}
enum ParameterValue {
    Int16 (i16), Int32(i32), Int64(i64), Varchar(String)
}

impl DynamicQuery {
    pub fn create_from_pk(schema_name: &str, 
                          entity_name: &str, 
                          entity:      &'static metainfo::Entity, 
                          pk_params:   Vec<String>) -> Result<DynamicQuery, String> {
        match &entity.primary_key {
            None => Err("Primary key not exists".to_string()),
            Some(ref pk_indices) => {
                let param_columns_len = pk_params.len();

                if param_columns_len != pk_indices.len() {
                    return Err("Count of columns in primary key does not match with count of parameters in query".to_string())
                }

                let mut params = Vec::with_capacity(param_columns_len);

                for (pk_column_index, p) in pk_indices.iter().zip(pk_params) {
                    let pk_column = unsafe { entity.columns.get_unchecked(*pk_column_index) };

                    let parsed = Parameter::parse(pk_column, p.to_string());
                    match parsed {
                        Err(err) => return Err(format!("Can not parse parameter value {} for column {}: {}", p, pk_column.name, err)),
                        Ok(parsed) => {
                            params.push(parsed);
                        }
                    }
                };

                let sql = generate_sql(schema_name, entity_name, &entity.columns, &params, vec![], 1, Option::None);

                Ok( DynamicQuery { sql, fetch_array_size: 1, columns: &entity.columns, params } )
            }
        }
    }

    pub fn create_from_params(schema_name: &str,
                              entity_name: &str,
                              entity:      &'static metainfo::Entity, 
                              parameters:  HashMap<String,String>,
                              order:       Vec<String>,
                              limit:       Option<u32>,
                              offset:      Option<u32>
    ) -> Result<DynamicQuery, String> {
        let param_columns_len = parameters.len();

        let mut params = Vec::with_capacity(param_columns_len);

        for (ref col_name,ref p) in parameters {
            let column = entity.columns.iter().find(|c|&c.name == col_name);

            match column {
                None => return Err(format!("Not found column {}", col_name)),
                Some(column) => {
                    let parsed = Parameter::parse(&column as &'static metainfo::Column, p.to_string());
                    match parsed {
                        Err(err) => return Err(format!("Can not parse parameter value {} for column {}: {}", p, col_name, err)),
                        Ok(parsed) => {
                            params.push(parsed);
                        }
                    }
                }
            }
        }

        for col_name in &order {
            let column = entity.columns.iter().find(|c|&c.name == col_name);
            if column.is_none() {
                return Err(format!("Order column {} nof found in table {}", col_name, &entity_name))
            }
        };

        let limit = limit.unwrap_or(25);

        if limit > 100  {
            return Err("limit rows must be <= 100".to_string());
        }

        if let Some(offset) = offset {
            if offset < limit {
                return Err("offset must be >= limit".to_string());
            }
            if offset % limit > 0 {
                return Err("offset must be a multiple of the limit (remainder must be zero)".to_string());
            }
        }

        let sql = generate_sql(schema_name, entity_name, &entity.columns, &params, order, limit, offset);

        Ok( DynamicQuery { sql, fetch_array_size: limit, columns: &entity.columns, params } )
    }

    /// execute a query and generate JSON result
    pub fn fetch_one(self) -> Result<String,String> {
        let conn = get_connection()
            .map_err(|err|format!("Can not connect to oracle: {}", err))?;

        let mut stmt = conn.prepare(&self.sql, &[oracle::StmtParam::FetchArraySize(self.fetch_array_size)])
            .map_err(|err| format!("can not prepare statement: {}", err))?;

        let params_view: Vec<&dyn oracle::sql_type::ToSql> = 
            self.params
                .iter()
                .map(|p| p as &dyn oracle::sql_type::ToSql)
                .collect();

        let row = stmt
            .query_row(&params_view[..])
            .map_err(|err| format!("can not dynamic query from statement: {:?}", err))?;

        let result = self.gen_result(row);

        Ok(result)
    }

    /// execute a query and generate JSON result
    pub fn fetch_many(self) -> Result<String,String> {
        let conn = get_connection()
            .map_err(|err|format!("Can not connect to oracle: {}", err))?;

        let mut stmt = conn.prepare(&self.sql, &[oracle::StmtParam::FetchArraySize(self.fetch_array_size)])
            .map_err(|err| format!("can not prepare statement: {}", err))?;

        let params_view: Vec<&dyn oracle::sql_type::ToSql> = 
            self.params
                .iter()
                .map(|p| p as &dyn oracle::sql_type::ToSql)
                .collect();

        let rows = stmt
            .query(&params_view[..])
            .map_err(|err| format!("can not dynamic query from statement: {:?}", err))?;

        let mut result = Vec::new();

        for row in rows {
            match row {
                Ok(row) => {
                    let r = self.gen_result(row);
                    result.push(r);        
                },
                Err(err) => {
                    return Err(format!("can not fetch query result: {:?}", err))
                }
            }
        }

        Ok( format!("[{}]", result.join(",")) )
    }

    fn gen_result(&self, rs: oracle::Row) -> String {
        let results: Vec<String> = self.columns
            .iter()
            .enumerate()
            .map(|(idx, col)|{
                let result = col.try_to_string(&rs, idx);
                format!("\"{}\":{}", col.name, result)
            }).collect();

        format!("{{ {} }}", results.join(","))
    }

}

fn generate_sql(schema_name: &str, 
    entity_name: &str, 
    columns:     &Vec<metainfo::Column>, 
    params:      &Vec<Parameter>,
    order:       Vec<String>,
    limit:       u32,
    offset:      Option<u32>
) -> String {
    let joined_result_columns = columns.iter().map(|c|&c.name).join(",");

    let enumerated_param_columns: Vec<String> =
    params.iter().enumerate().map(|(idx,p)|format!("{} = :{}", p.column.name, idx+1)).collect();
    let joined_param_columns = enumerated_param_columns.join(" AND ");

    let mut sql = format!("SELECT {} FROM {}.{} WHERE {}", joined_result_columns, schema_name, entity_name, joined_param_columns);

    if order.len() > 0 {
        let joined_order_columns = order.join(",");
        let order_clause = format!(" ORDER BY {}", joined_order_columns);
        sql.push_str(&order_clause);
    }

    if limit > 1 {
        if let Some(offset) = offset {
            let offset_clause = format!(" OFFSET {} ROWS", offset);
            sql.push_str(&offset_clause);
        }
        let fetch_clause = format!(" FETCH NEXT {} ROWS ONLY", limit);
        sql.push_str(&fetch_clause);
    }
    sql        
}

impl Parameter {
    fn parse(column: &'static metainfo::Column, value: String) -> Result<Self, &'static str> {
        let value = match column.col_type {
            metainfo::ColumnType::Integer => {
                match column.col_size {
                    2 => {
                        let val: i16 = value.parse().unwrap();
                        Ok(ParameterValue::Int16(val))
                    },
                    4 => {
                        let val: i32 = value.parse().unwrap();
                        Ok(ParameterValue::Int32(val))
                    },
                    8 => {
                        let val: i64 = value.parse().unwrap();
                        Ok(ParameterValue::Int64(val))
                    },
                    _ => Err("Not supported size for Number")
                }
            },
            metainfo::ColumnType::String => {
                Ok(ParameterValue::Varchar(value))
            },
            _ => Err("Not supported type for Primary key")
        };
        value.map(|v| Parameter{ column, value: v})
    }
}

impl oracle::sql_type::ToSql for Parameter {
    fn oratype(&self, _conn: &oracle::Connection) -> oracle::Result<oracle::sql_type::OracleType> {
        Ok(self.column.sql_type.clone())
    }

    fn to_sql(&self, p: &mut oracle::SqlValue) -> oracle::Result<()> {
        match &self.value {
            ParameterValue::Int16(val) => {
                val.to_sql(p)
            },
            ParameterValue::Int32(val) => {
                val.to_sql(p)
            },
            ParameterValue::Int64(val) => {
                val.to_sql(p)
            },
            ParameterValue::Varchar(val) => {
                val.to_sql(p)
            },
        }
    }
}
