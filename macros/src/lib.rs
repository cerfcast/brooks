use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{DeriveInput, Field, Ident, Meta, PathSegment, TypePath, spanned::Spanned, token};

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

#[proc_macro_attribute]
pub fn grammar_location(_: TokenStream, item: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(item).unwrap();

    let items = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct.fields.clone(),
        _ => todo!(),
    };

    let mut items = match items {
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

    let name = &ast.ident;

    let mut replacement_attrs = proc_macro2::TokenStream::new();
    for a in ast.attrs {
        a.to_tokens(&mut replacement_attrs);
    }

    let generated = quote! {
        #replacement_attrs
        pub struct #name
            #items
    };

    return generated.into();
}
