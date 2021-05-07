extern crate quote;

#[macro_use]
extern crate syn;

extern crate proc_macro;
use proc_macro::TokenStream;

use syn::{parse_macro_input, DeriveInput};

mod internals;
mod query;
mod utils;

/// Generate RowValue implementation in form of #[derive(RowValue)]
/// example:
/// #[derive(RowValue)]
// pub struct OraTable {
//     owner: String,
//     table_name: String
// }
//
/// impl oracle::RowValue for OraTable {
//     fn get(row: &oracle::Row) -> std::result::Result<OraTable, Error> {
//           Ok(OraTable { 
//               owner: row.get(0)?, 
//               table_name: row.get(1)?
//          })
//     }
// }
///
#[proc_macro_derive(RowValue)]
pub fn derive_query(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    query::expand_derive_query(&input)
        .unwrap_or_else(utils::to_compile_errors)
        .into()    
}