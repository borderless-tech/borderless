use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::Result;
use syn::{Ident, Item};

pub fn parse_module_content(
    mod_span: Span,
    mod_items: &Vec<Item>,
    _mod_ident: &Ident,
) -> Result<TokenStream2> {
    let wasm_impl = quote! {
        pub fn exec_txn() -> Result<()> {
            Ok(())
        }

        pub fn exec_introduction() -> Result<()> {
            Ok(())
        }

        pub fn exec_revocation() -> Result<()> {
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
            use ::borderless::Result;
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
