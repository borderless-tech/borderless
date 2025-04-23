use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{spanned::Spanned, Data, DataStruct, DeriveInput, Error, Fields, Result};

pub fn impl_named_sink(input: DeriveInput) -> Result<TokenStream2> {
    let DeriveInput { ident, data, .. } = input;

    // Variants snake-case, camel-case and fields for each variant
    match data {
        Data::Struct(DataStruct { struct_token, .. }) => Err(Error::new_spanned(
            struct_token,
            "NamedSink can only be derived for enums",
        )),
        Data::Union(syn::DataUnion { union_token, .. }) => Err(Error::new_spanned(
            union_token,
            "NamedSink can only be derived for enums",
        )),
        Data::Enum(syn::DataEnum { variants, .. }) => {
            let mut var_idents = Vec::with_capacity(variants.len());
            let mut inner_types = Vec::with_capacity(variants.len());
            for v in variants.into_iter() {
                let span = v.span();
                let field = match v.fields {
                    Fields::Named(_) | Fields::Unit => {
                        return Err(Error::new(
                            span,
                            "NamedSink can only be derived for enums with unnamed fields",
                        ))
                    }
                    Fields::Unnamed(u) => u,
                };
                if field.unnamed.len() != 1 {
                    return Err(Error::new(
                        span,
                        "Unnamed fields must have exactly one member",
                    ));
                }
                let inner = field.unnamed.into_iter().next().unwrap();
                var_idents.push(v.ident);
                inner_types.push(inner.ty);
            }

            let sink_names = var_idents.iter().map(|i| i.to_string());

            let trait_impl = quote! {

                #[doc(hidden)]
                const _: () = {
                    #[allow(unused_extern_crates, clippy::useless_attribute)]
                    extern crate borderless as _borderless;

                    #[doc(hidden)]
                    #[automatically_derived]
                    const fn __check_into_action<T: TryInto<_borderless::events::CallAction>>() {}
                    #(
                    __check_into_action::<#inner_types>();
                    )*

                    #[automatically_derived]
                    impl _borderless::NamedSink for #ident {
                        fn into_action(self) -> ::std::result::Result<(&'static str, _borderless::events::CallAction), _borderless::serialize::Error> {
                            match self {
                                #(
                                #ident::#var_idents(inner) => {
                                    let action: _borderless::events::CallAction = inner.try_into()?;
                                    Ok((#sink_names, action))
                                }
                                )*
                            }
                        }
                    }
                };
            };
            Ok(trait_impl)
        }
    }
}
