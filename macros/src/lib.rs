use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(GrammarName)]
pub fn grammar_name_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let generated = quote! {
        impl GrammarNode for #name {
            fn name() -> String {
                "mel".to_string()
            }
        }
    };
    generated.into()
}
