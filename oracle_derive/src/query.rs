use proc_macro2::TokenStream;
use syn::{self, Index, Member, spanned::Spanned};
use quote::{quote, quote_spanned};

use crate::internals::Ctxt;
use crate::internals::ast::Container;

/// Expands #[derive(Query)] macro.
pub fn expand_derive_query(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctxt = Ctxt::new();

    let cont = match Container::from_ast(&ctxt, input) {
        Some(cont) => cont,
        None => return Err(ctxt.check().unwrap_err()),
    };

    ctxt.check()?;

    let name = cont.ident;
    let (impl_generics, ty_generics, where_clause) = cont.generics.split_for_impl();

    let doc_comment = format!("Provide metainfo for `{}`.", name);

    let from_rs_body = generate_from_values(&cont);

    Ok(quote! {
        impl #impl_generics oracle::RowValue for #name #ty_generics #where_clause {
            #[doc = #doc_comment]

            fn get(row: &oracle::Row) -> std::result::Result<#name, oracle::Error> {
                Ok(#from_rs_body)
            }
        }
    })
}

/// Generate body
/// Work only for structs and tuples.
/// Example:
///         OraTable { owner: row.get(0)?, table_name: row.get(1)?}
/// or for tuples:
///         OraTable ( row.get(0)?, row.get(1)? )
fn generate_from_values(cont: &Container) -> TokenStream {
    let expressions = cont.data.all_fields().enumerate().map(|(i,f)| {
        let index = Index::from(i);
        let body = quote_spanned! { f.original.span() => ( (row.get(#index)) )? };
        match &f.member {
            Member::Named(name) => quote_spanned! { f.original.span() => #name: #body },
            Member::Unnamed(_) => body
        }
    });
    let name = cont.ident;
    if cont.data.is_struct() {
        quote! {
        #name{ #(#expressions),* }
        }
    } else {
        quote! {
        #name( #(#expressions),* )
        }
    }
}
