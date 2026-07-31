use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    DeriveInput, Field, Ident, PathSegment, TypePath, punctuated::Punctuated, spanned::Spanned,
    token,
};

use syn::parse::Parser;
use syn::{Path, Token};

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

/*

impl BuiltinFunction for PathElementBuiltin {
    fn name(&self) -> String {
        "path_element".to_string()
    }

    fn parameters(&self) -> tvs::Params {
        tvs::Params {
            args: vec![Type::String, Type::Integer],
        }
    }

    fn return_type(&self) -> Type {
        Type::String
    }

    fn interpw(&self, args: Value) -> BuiltinInterpResult {
        let args = match args {
            Value::ArgumentList(args) => args,
            _ => return Err(BuiltinInterpError::ArgumentsInvalid),
        };

        if args.len() != 2 {
            return Err(BuiltinInterpError::ArgumentMiscount(2, args.len()));
        }

        let path = match &args[0] {
            TypedValue {
                value: Value::String(s),
                tipe: _,
            } => s,
            TypedValue { value: _, tipe: t } => {
                return Err(BuiltinInterpError::ArgumentMismatch(
                    0,
                    Type::String,
                    t.clone(),
                ));
            }
        };

        let element = match &args[1] {
            TypedValue {
                value: Value::Integer(i),
                tipe: _,
            } => i,
            TypedValue { value: _, tipe: t } => {
                return Err(BuiltinInterpError::ArgumentMismatch(
                    1,
                    Type::Integer,
                    t.clone(),
                ));
            }
        };

        self.interp(path, element)
    }
}

*/

#[proc_macro_attribute]
pub fn builtin_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parser = Punctuated::<Path, Token![,]>::parse_terminated;
    let path = parser.parse(attr).unwrap();

    let ast: DeriveInput = syn::parse(item).unwrap();
    let name = &ast.ident;

    let builtin_name = name.to_string().replace("Builtin", "").to_lowercase();

    let mut path = path.iter();

    let return_type = path.nth(0).expect("Missing return type");

    let parameter_types_values: Vec<_> = path
        .map(|p| {
            let mut vp = p.clone();
            vp.segments[0] = PathSegment {
                ident: Ident::new("Value", p.span()),
                arguments: syn::PathArguments::None,
            };
            (p, vp)
        })
        .collect();

    let parameter_types: Vec<_> = parameter_types_values
        .clone()
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    let parameter_types_len = parameter_types_values.len();

    let type_check = parameter_types_values
        .iter()
        .enumerate()
        .map(|(i, (t, v))| {
            let arg_name = format_ident!("arg{i}");
            quote! {
                let #arg_name = match &args[#i] {
                        TypedValue {
                            value: #v(s),
                            tipe: _,
                        } => s,
                        TypedValue { value: _, tipe: t } => {
                            return Err(BuiltinInterpError::ArgumentMismatch(
                                #i,
                                #t,
                                t.clone(),
                            ));
                        }
                    };
            }
        });

    let interp_arguments: Vec<_> = (0..parameter_types_len)
        .map(|c| format_ident!("arg{c}"))
        .collect();

    let (x, y, z) = ast.generics.split_for_impl();

    let generated = quote! {
        #ast
        impl #x BuiltinFunction for #name #y #z {
            fn name(&self) -> String {
                #builtin_name.to_string()
            }

            fn parameters(&self) -> tvs::Params {
                tvs::Params {
                    args: vec![#(#parameter_types),*]
                }
            }

            fn return_type(&self) -> Type {
                #return_type
            }


            fn interpw(&self, args: Value) -> BuiltinInterpResult {
                let args = match args {
                    Value::ArgumentList(args) => args,
                    _ => return Err(BuiltinInterpError::ArgumentsInvalid),
                };

                if args.len() != #parameter_types_len {
                    return Err(BuiltinInterpError::ArgumentMiscount(#parameter_types_len, args.len()));
                }

                #(#type_check)*
                self.interp(#(#interp_arguments),*)
            }

        }
    };

    generated.into()
}

#[proc_macro_derive(TypedGenericMetadata)]
pub fn derive_typed_generic_metadata(item: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(item).unwrap();
    let name = &ast.ident;
    let untyped_name = format_ident!("{}", ast.ident.to_string().trim_start_matches("Typed"));
    let metadata_type = format!("MI.{untyped_name}");

    let (x, y, z) = ast.generics.split_for_impl();

    let yy_all: Vec<_> = ast
        .generics
        .type_params()
        .map(|tp| {
            let mut ntp = tp.clone();
            ntp.ident = format_ident!("__{}", &ntp.ident);
            ntp.into_token_stream()
        })
        .collect();

    let yy_names: Vec<_> = ast
        .generics
        .type_params()
        .map(|tp| format_ident!("__{}", &tp.ident))
        .collect();

    let yyy_all = yy_all.iter().fold(quote! {}, |mut existing, next| {
        existing.extend(next.into_token_stream());
        existing.extend(quote! {,});
        existing
    });
    let yyy_names = yy_names.iter().fold(quote! {}, |mut existing, next| {
        existing.extend(next.into_token_stream());
        existing.extend(quote! {,});
        existing
    });

    let r = quote! {
    impl #x #name #y #z {
        pub fn typed_generic_metadata_name() -> String {
            #metadata_type.to_string()
        }

        pub fn typed_value<#yyy_all>(sr: #untyped_name<#yyy_names>) -> #name::<#yyy_names> {
            #name::<#yyy_names> {
                tpe: #metadata_type.to_string(),
                value: sr,
            }
        }
    }
    };

    r.into()
}
