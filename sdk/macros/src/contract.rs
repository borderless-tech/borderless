use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Error, Result};
use syn::{Ident, Item};

use crate::utils::check_if_state;

pub fn parse_module_content(
    mod_span: Span,
    mod_items: &Vec<Item>,
    _mod_ident: &Ident,
) -> Result<TokenStream2> {
    let read_input = quote! {
        let input = read_register(REGISTER_INPUT).context("missing input register")?;
    };

    let state = get_state(&mod_items, &mod_span)?;
    // TODO: Parse state_ident !
    //
    let read_state = quote! {
        let mut state = <#state as ::borderless::__private::storage_traits::State>::load()?;
        ::borderless::info!("-- {state:#?}");
    };

    let match_method = quote! {};

    let commit_state = quote! {
        <#state as ::borderless::__private::storage_traits::State>::commit(state);
    };

    let wasm_impl = quote! {
        pub fn exec_txn() -> Result<()> {
            #read_input

            let action = CallAction::from_bytes(&input)?;
            let s = action.pretty_print()?;
            info!("{s}");

            let method = action
                .method_name()
                .context("missing required method-name")?;

            #read_state
            #match_method
            #commit_state
            Ok(())
        }

        pub fn exec_introduction() -> Result<()> {
            #read_input
            let introduction = Introduction::from_bytes(&input)?;
            let s = introduction.pretty_print()?;
            info!("{s}");
            // TODO: Parse state from introduction
            let state = <#state as ::borderless::__private::storage_traits::State>::init(introduction.initial_state)?;

            #commit_state
            Ok(())
        }

        pub fn exec_revocation() -> Result<()> {
            #read_input
            let r = Revocation::from_bytes(&input)?;
            info!("Revoked contract. Reason: {}", r.reason);
            Ok(())
        }

        pub fn exec_get_state() -> Result<()> {
            Ok(())
        }

        pub fn exec_post_action() -> Result<()> {
            Ok(())
        }
    };

    Ok(quote! {
        #[automatically_derived]
        pub(super) mod __derived {
            use super::*;
            use ::borderless::*;
            use ::borderless::__private::{
                read_field, read_register, read_string_from_register, registers::*,
                storage_keys::make_user_key, write_field, write_register, write_string_to_register,
            };
            use ::borderless::contract::*;
            #wasm_impl
        }
    })
}

pub fn generate_wasm_exports(mod_ident: &Ident) -> TokenStream2 {
    let derived = quote! { #mod_ident::__derived };

    quote! {
    #[no_mangle]
    pub extern "C" fn process_transaction() {
        let result = #derived::exec_txn();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn process_introduction() {
        let result = #derived::exec_introduction();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn process_revocation() {
        let result = #derived::exec_revocation();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn http_get_state() {
        let result = #derived::exec_get_state();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn http_post_action() {
        let result = #derived::exec_post_action();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }
    }
}

// TODO: Make this the parse_items function later on, that checks that every required item is in the module
fn get_state(items: &[Item], mod_span: &Span) -> Result<Ident> {
    for item in items {
        if let Item::Struct(item_struct) = item {
            if check_if_state(item_struct) {
                return Ok(item_struct.ident.clone());
            }
        }
    }
    Err(Error::new(
        *mod_span,
        "Each module requires a 'State' - use #[derive(State)]",
    ))
}

/*
 *
 * use proc_macro_crate::{crate_name, FoundCrate};

let borderless_crate = match crate_name("borderless") {
    Ok(FoundCrate::Itself) => quote!(crate),
    Ok(FoundCrate::Name(name)) => {
        let ident = syn::Ident::new(&name, Span::call_site());
        quote!(::#ident)
    }
    Err(_) => panic!("borderless crate not found in dependencies"),
};
quote! {
    #borderless_crate::info!("...");
}

-> Will also work, if the crate is renamed by the user

 */
