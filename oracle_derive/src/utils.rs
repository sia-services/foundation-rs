use quote::quote;

pub fn extract_path(ty: &syn::Type) -> Option<&syn::TypePath> {
    if let syn::Type::Path(x) = ty {
        Some(x)
    } else {
        None
    }
}

pub fn extract_reference(ty: &syn::Type) -> Option<&syn::TypeReference> {
    if let syn::Type::Reference(x) = ty {
        Some(x)
    } else {
        None
    }
}

pub fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}