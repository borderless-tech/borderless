#![allow(unused)]
//! Creates some utility functions to check and parse different attributes.
use proc_macro2::Span;
use syn::{Attribute, Fields, Ident, ItemEnum, MetaList, Result, Type};

/// Returns true if the meta-list has a 'derive' attribute
pub(crate) fn has_derive(meta_list: &MetaList) -> bool {
    let ident = Ident::new("derive", Span::call_site());
    meta_list.path.segments.iter().any(|p| p.ident == ident)
}

/// Checks if an enum is unit-only (with no tuple and struct variants).
pub(crate) fn check_if_unit(item_enum: &ItemEnum) -> Result<()> {
    for variant in item_enum.variants.iter() {
        match &variant.fields {
            Fields::Unit => {
                // Check if the enum field has an integer assigned to it
                // if variant.discriminant.is_none() {
                //     return Err(syn::Error::new_spanned(
                //         variant,
                //         format!(
                //             "It is advised to explicitly set the role-id for each enum variant.\
                //              \nThe 'Roles' enum is converted to an u32 by the sdk.\
                //              \nRust (by default) assigns the integers based on the ordering of the enum fields, which may cause problems.\
                //              \n\nTo avoid this do '{} = 0' (or any other positive integer)",
                //             variant.ident
                //         ),
                //     ));
                // }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "Only unit fields are allowed (with no tuple and struct variants)",
                ));
            }
        }
    }
    Ok(())
}
/// Checks if some attribute is an `#[action]`
pub(crate) fn check_if_action(attr: &Attribute) -> bool {
    check_attr_name(attr, "action")
}

/// Checks if some attribute is a `#[schedule]`
pub(crate) fn check_if_schedule(attr: &Attribute) -> bool {
    check_attr_name(attr, "schedule")
}

/// Checks if some attribute is a `#[backgroun_task]`
pub(crate) fn check_if_background_task(attr: &Attribute) -> bool {
    check_attr_name(attr, "background_task")
}

/// Checks if some attribute is a `#[http_handler]`
pub(crate) fn check_if_http_handler(attr: &Attribute) -> bool {
    check_attr_name(attr, "http_handler")
}

/// Checks if some attribute is `#[attr_name]` or `#[borderless::attr_name]`
fn check_attr_name(attr: &Attribute, attr_name: &'static str) -> bool {
    let target_attr = Ident::new(attr_name, Span::call_site());
    if let Some(ident) = attr.path().get_ident() {
        if *ident != target_attr {
            return false;
        }
    } else {
        let borderless = Ident::new("borderless", Span::call_site());
        let mut iter = attr.path().segments.iter();
        let first = iter.next();
        let sec = iter.next();
        // borderless::http_handler is the only allowed attribute
        match (first, sec) {
            (Some(first), Some(sec)) => {
                if first.ident != borderless || sec.ident != target_attr {
                    return false;
                }
            }
            _ => {
                return false;
            }
        }
    };
    true
}
