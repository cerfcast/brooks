use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    DeriveInput, Field, Ident, PathSegment, TypePath, punctuated::Punctuated, spanned::Spanned,
    token,
};

use syn::Token;
use syn::parse::Parser;

#[proc_macro_attribute]
pub fn grammar_name(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
    let path = parser.parse(attr).unwrap();

    let ast: DeriveInput = syn::parse(item).unwrap();
    let name = &ast.ident;

    let (x, y, z) = ast.generics.split_for_impl();

    let names = path
        .iter()
        .fold(proc_macro2::TokenStream::new(), |mut ts, n| {
            ts.extend(quote! { stringify!(#n).to_string(), });
            return ts;
        });

    let generated = quote! {
        #ast
        impl #x GrammarNode for #name #y #z {
            fn name() -> Vec<String> {
                vec![#names]
            }
        }
    };

    generated.into()
}

#[proc_macro_attribute]
pub fn grammar_location(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast: DeriveInput = syn::parse(item).unwrap();

    let mut strct = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct.clone(),
        _ => todo!(),
    };

    let mut items = match strct.fields {
        syn::Fields::Named(fields_named) => fields_named,
        _ => todo!(),
    };

    items.named.push(Field {
        attrs: vec![],
        vis: syn::Visibility::Public(token::Pub { span: ast.span() }),
        mutability: syn::FieldMutability::None,
        ident: Some(Ident::new("location", ast.span())),
        colon_token: None,
        ty: syn::Type::Path(TypePath {
            qself: None,
            path: PathSegment {
                ident: Ident::new("GrammarLocation", ast.span().clone()),
                arguments: syn::PathArguments::None,
            }
            .into(),
        }),
    });

    strct.fields = syn::Fields::Named(items);
    ast.data = syn::Data::Struct(strct);
    let ts = ast.to_token_stream();

    let generated = quote! {
        #ts
    };

    return generated.into();
}
