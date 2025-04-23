use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Error, Field, Fields, Ident, Result, Type};
use xxhash_rust::const_xxh3::xxh3_64;

// TODO: Add option to hide fields from API access

pub fn impl_state(input: DeriveInput) -> Result<TokenStream2> {
    let DeriveInput { ident, data, .. } = input;

    let fields = parse_input(data)?;

    let idents: Vec<Ident> = fields.iter().flat_map(|f| f.ident.clone()).collect();
    let ftypes: Vec<Type> = fields.iter().map(|f| f.ty.clone()).collect();

    let ident_strings: Vec<_> = idents.iter().map(|f| format!("{f}")).collect();

    let errs: Vec<_> = idents
        .iter()
        .map(|i| format!("failed to read parse field '{i}'"))
        .collect();

    let storage_keys: Vec<u64> = fields.iter().map(|f| storage_key(f, &ident)).collect();

    let storeable = quote! { ::borderless::__private::storage_traits::Storeable };

    let to_payload = quote! { ::borderless::__private::storage_traits::ToPayload };

    let type_checks = ftypes.iter().map(|ty| {
        quote! {
            __check_storeable::<#ty>();
        }
    });

    Ok(quote! {
        #[doc(hidden)]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate borderless as _borderless;

            #[doc(hidden)]
            #[automatically_derived]
            const fn __check_storeable<T: _borderless::__private::storage_traits::Storeable>() {}
            #(#type_checks)*

            #[doc(hidden)]
            #[automatically_derived]
            const SYMBOLS: &[(&str, u64)] = &[
                #(
                    (#ident_strings, #storage_keys)
                ),*
            ];

            #[automatically_derived]
            impl _borderless::__private::storage_traits::State for #ident {
                fn load() -> _borderless::Result<Self> {
                    // Decode every field based on the Storeable implementation
                    #(
                        let #idents = <#ftypes as #storeable>::decode(#storage_keys);
                    )*
                    Ok(Self {
                        #(#idents),*
                    })
                }

                fn init(mut value: _borderless::serialize::Value) -> _borderless::Result<Self> {
                    use _borderless::Context;
                    #(
                        let base_value = value.get_mut(#ident_strings).take().context(#errs)?;
                        let #idents = <#ftypes as #storeable>::parse_value(base_value.clone(), #storage_keys).context(#errs)?;
                    )*
                    Ok(Self {
                        #(#idents),*
                    })
                }

                fn http_get(path: String) -> _borderless::Result<Option<String>> {
                    use _borderless::Context;
                    let path = path.strip_prefix('/').unwrap_or(&path);

                    // Extract query string
                    let (path, _query) = match path.split_once('?') {
                        Some((path, query)) => (path, Some(query)),
                        None => (path, None),
                    };
                    // Quick-shot, check if the user wants to access the entire state
                    if path.is_empty() {
                        // State does not implement serialize, so we have to to this field by field
                        let state = <Self as _borderless::__private::storage_traits::State>::load()?;
                        // Manually build the json with the parsed fields
                        let mut buf = String::with_capacity(100);
                        buf.push('{');
                        #(
                            let value = <#ftypes as #to_payload>::to_payload(&state.#idents, "")?.context(#errs)?;
                            buf.push('"');
                            buf.push_str(#ident_strings);
                            buf.push('"');
                            buf.push(':');
                            buf.push_str(&value);
                            buf.push(',');
                        )*
                        buf.pop();
                        buf.push('}');
                        return Ok(Some(buf));
                    }
                    let (prefix, suffix) = match path.find('/') {
                        Some(idx) => path.split_at(idx),
                        None => (path, ""),
                    };

                    match prefix {
                        #(
                            #ident_strings => {
                                let value = <#ftypes as #storeable>::decode(#storage_keys);
                                <#ftypes as #to_payload>::to_payload(&value, suffix)
                            }
                        )*
                        _ => Ok(None),
                    }
                }

                fn commit(self) {
                    // call .commit() on every field
                    #(
                        <#ftypes as #storeable>::commit(self.#idents, #storage_keys);
                    )*
                }

                fn symbols() -> &'static [(&'static str, u64)] {
                    SYMBOLS
                }
            }
        };

    })
}

/*
 *
// NOTE: This is something that's purely based on the state
fn get_state_response(path: String) -> Result<(u16, String)> {
}
 */

fn parse_input(data: Data) -> Result<Vec<Field>> {
    // Variants snake-case, camel-case and fields for each variant
    match data {
        Data::Struct(DataStruct {
            struct_token,
            fields,
            semi_token,
        }) => {
            // Filter out tuple and unit structs:
            let fields = match fields {
                Fields::Named(named_fields) => Ok(named_fields.named),
                Fields::Unnamed(unnamed) => Err(Error::new_spanned(
                    unnamed,
                    "State can only be implemented on structs with named fields",
                )),
                Fields::Unit => Err(Error::new_spanned(
                    semi_token,
                    "State cannot be implemented on unit structs",
                )),
            }?;
            if fields.is_empty() {
                return Result::Err(Error::new_spanned(
                    struct_token,
                    "State must at least have one field",
                ));
            }
            // Extract field identifier and types
            let mut ident_fields = Vec::new();
            for field in fields {
                if field.ident.is_none() {
                    unreachable!("State macro is not allowed on tuple structs!");
                }
                ident_fields.push(field);
            }
            Ok(ident_fields)
        }
        Data::Enum(syn::DataEnum { enum_token, .. }) => Err(Error::new_spanned(
            enum_token,
            "State cannot be implemented on enums. Only structs with named fields are allowed.",
        )),
        Data::Union(syn::DataUnion { union_token, .. }) => Err(Error::new_spanned(
            union_token,
            "State cannot be implemented on unions. Only structs with named fields are allowed.",
        )),
    }
}

/// Calculate the storage key for some field
/// TODO: We can add an attribute, so that the user can override the storage-key for this field.
fn storage_key(field: &Field, ident: &Ident) -> u64 {
    let field_name = field
        .ident
        .as_ref()
        .expect("checked for named fields before calling");

    // Use xx3_64 hash to calculate storage key
    let full_name = format!("{}::{}", ident, field_name);
    let storage_key = xxh3_64(full_name.to_uppercase().as_bytes());
    // NOTE: This is basically the make_user_key function; but since we cannot import it here,
    // we just automatically convert the key by setting the highest bit to "1".
    //
    // TODO: Write a test, that always checks that the macro keys are in user space !
    storage_key | (1 << 63)
}

/*
 *
 */
