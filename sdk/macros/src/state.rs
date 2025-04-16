use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Error, Field, Fields, Ident, Result, Type};
use xxhash_rust::const_xxh3::xxh3_64;

pub fn impl_state(input: DeriveInput) -> Result<TokenStream2> {
    let DeriveInput { ident, data, .. } = input;

    let fields = parse_input(data)?;

    let idents: Vec<Ident> = fields.iter().flat_map(|f| f.ident.clone()).collect();
    let ftypes: Vec<Type> = fields.iter().map(|f| f.ty.clone()).collect();

    let ident_strings: Vec<String> = idents.iter().map(|f| format!("{f}")).collect();

    let errs: Vec<_> = idents
        .iter()
        .map(|i| format!("failed to read parse field '{i}'"))
        .collect();

    let storage_keys: Vec<u64> = fields.iter().map(|f| storage_key(f, &ident)).collect();

    let storeable = quote! { ::borderless::__private::storage_traits::Storeable };

    Ok(quote! {
        impl ::borderless::__private::storage_traits::State for #ident {
            fn load() -> ::borderless::Result<Self> {
                // Decode every field based on the Storeable implementation
                #(
                    let #idents = <#ftypes as #storeable>::decode(#storage_keys);
                )*
                Ok(Self {
                    #(#idents),*
                })
            }

            fn init(mut value: ::borderless::serialize::Value) -> ::borderless::Result<Self> {
                use ::borderless::Context;
                #(
                    let base_value = value.get_mut(#ident_strings).take().context(#errs)?;
                    let #idents = <#ftypes as #storeable>::parse_value(base_value.clone(), #storage_keys).context(#errs)?;
                )*
                Ok(Self {
                    #(#idents),*
                })
            }

            fn http_get(path: String) -> Option<String> {
                todo!()
            }

            fn commit(self) {
                // call .commit() on every field
                #(
                    <#ftypes as #storeable>::commit(self.#idents, #storage_keys);
                )*
            }
        }
    })
}

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
                    "ContractState must at least have one field",
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
