use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Meta};

#[proc_macro_attribute]
pub fn grammar_name(attr: TokenStream, item: TokenStream) -> TokenStream {
    let meta: Meta = syn::parse(attr).unwrap();
    let ast: DeriveInput = syn::parse(item).unwrap();
    let name = &ast.ident;
    let mp = meta.path().get_ident().unwrap();
    let generated = quote! {
        #ast
        impl GrammarNode for #name {
            fn name() -> String {
                stringify!(#mp).to_string()
            }
        }
    };

    generated.into()
}
