use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Error, Ident, Item, Result};

use crate::{
    action::{get_actions, ActionFn},
    utils::check_if_state,
};

// TODO: Check, if module contains all required elements for a contract

pub fn parse_module_content(
    mod_span: Span,
    mod_items: &[Item],
    _mod_ident: &Ident,
) -> Result<TokenStream2> {
    let read_input = quote! {
        let input = read_register(REGISTER_INPUT).context("missing input register")?;
    };

    let state = get_state(mod_items, &mod_span)?;
    let actions = get_actions(&state, mod_items)?;
    let action_types = actions.iter().map(ActionFn::gen_type_tokens);
    let call_action: Vec<_> = actions.iter().map(|a| a.gen_call_tokens(&state)).collect();
    let action_names: Vec<_> = actions.iter().map(ActionFn::method_name).collect();
    let action_ids: Vec<_> = actions.iter().map(ActionFn::method_id).collect();

    // let _args: __ArgsType = ::borderless::serialize::from_value(action.params.clone())?;
    let check_action: Vec<_> = actions
        .iter()
        .map(|a| {
            a.gen_check_tokens(
                quote! { ::borderless::serialize::from_value(action.params.clone())? },
            )
        })
        .collect();

    // let _args: __ArgsType = ::borderless::serialize::from_slice(&payload)?;
    let check_payload: Vec<_> = actions
        .iter()
        .map(|a| a.gen_check_tokens(quote! { ::borderless::serialize::from_slice(&payload)? }))
        .collect();

    let action_symbols = quote! {
        #[doc(hidden)]
        #[automatically_derived]
        const ACTION_SYMBOLS: &[(&str, u32)] = &[
            #(
                (#action_names, #action_ids)
            ),*
        ];
    };

    let actions_enum = impl_actions_enum(&actions);

    // Generate the nested match block for matching the action method by name or id
    // match &action.method { ... => match method_name => { ... => FUNC } }
    let match_and_call_action = match_action(&action_names, &action_ids, &call_action);
    let match_and_check_action = match_action(&action_names, &action_ids, &check_action);

    let exec_post = quote! {
        #[automatically_derived]
        fn post_action_response(path: String, payload: Vec<u8>) -> Result<CallAction> {
            let path = path.replace("-", "_"); // Convert from kebab-case to snake_case
            let path = path.strip_prefix('/').unwrap_or(&path); // stip leading "/"

            let content = String::from_utf8(payload.clone()).unwrap_or_default();
            info!("{content}");

            #[allow(unreachable_code)]
            match path {
                "" => {
                    let action = CallAction::from_bytes(&payload).context("failed to parse action")?;
                    #match_and_check_action
                    // At this point, the action is validated and can be returned
                    Ok(action)
                }
                #(
                #action_names => {
                    #check_payload
                    let value = ::borderless::serialize::to_value(&_args)?;
                    Ok(CallAction::by_method(#action_names, value))
                }
                )*
                other => Err(new_error!("unknown method: {other}")),
            }
        }
    };

    let as_state = quote! {
        <#state as ::borderless::__private::storage_traits::State>
    };
    let get_symbols = quote! {
        #[automatically_derived]
        pub(crate) fn get_symbols() -> Result<()> {
            let symbols = Symbols::from_symbols(#as_state::symbols(), ACTION_SYMBOLS);
            let bytes = symbols.to_bytes()?;
            write_register(REGISTER_OUTPUT, &bytes);
            Ok(())
        }
    };

    let exec_txn = quote! {
        #[automatically_derived]
        pub(crate) fn exec_txn() -> Result<()> {
            #read_input

            let action = CallAction::from_bytes(&input)?;
            let s = action.pretty_print()?;
            info!("{s}");
            let mut state = #as_state::load()?;
            #match_and_call_action
            #as_state::commit(state);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_introduction() -> Result<()> {
            #read_input
            let introduction = Introduction::from_bytes(&input)?;
            let s = introduction.pretty_print()?;
            info!("{s}");
            let state = #as_state::init(introduction.initial_state)?;
            #as_state::commit(state);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_revocation() -> Result<()> {
            #read_input
            let r = Revocation::from_bytes(&input)?;
            info!("Revoked contract. Reason: {}", r.reason);
            Ok(())
        }
    };

    let exec_http = quote! {
        #[automatically_derived]
        pub(crate) fn exec_get_state() -> Result<()> {
            let path = read_string_from_register(REGISTER_INPUT_HTTP_PATH).context("missing http-path")?;
            let result = #as_state::http_get(path)?;
            let status: u16 = if result.is_some() { 200 } else { 404 };
            let payload = result.unwrap_or_default();
            write_register(REGISTER_OUTPUT_HTTP_STATUS, status.to_be_bytes());
            write_string_to_register(REGISTER_OUTPUT_HTTP_RESULT, payload);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_post_action() -> Result<()> {
            let path = read_string_from_register(REGISTER_INPUT_HTTP_PATH).context("missing http-path")?;
            let payload = read_register(REGISTER_INPUT_HTTP_PAYLOAD).context("missing http-payload")?;
            match post_action_response(path, payload) {
                Ok(action) => {
                    write_register(REGISTER_OUTPUT_HTTP_STATUS, 200u16.to_be_bytes());
                    write_register(REGISTER_OUTPUT_HTTP_RESULT, action.to_bytes()?);
                }
                Err(e) => {
                    write_register(REGISTER_OUTPUT_HTTP_STATUS, 400u16.to_be_bytes());
                    write_string_to_register(REGISTER_OUTPUT_HTTP_RESULT, e.to_string());
                }
            };
            Ok(())
        }
    };

    // Combine everything in the __derived module:
    let derived = quote! {
        #[doc(hidden)]
        #[automatically_derived]
        pub(super) mod __derived {
            use super::*;
            use ::borderless::prelude::*;
            use ::borderless::__private::{
                read_field, read_register, read_string_from_register, registers::*,
                storage_keys::make_user_key, write_field, write_register, write_string_to_register,
            };
            #action_symbols
            #(#action_types)*
            #exec_post
            #get_symbols
            #exec_txn
            #exec_http
        }

        pub(super) mod actions {
            use super::__derived::*;
            #actions_enum
        }
    };
    Ok(derived)
}

pub fn generate_wasm_exports(mod_ident: &Ident) -> TokenStream2 {
    let derived = quote! { #mod_ident::__derived };

    quote! {
    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn process_transaction() {
        let result = #derived::exec_txn();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn process_introduction() {
        let result = #derived::exec_introduction();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn process_revocation() {
        let result = #derived::exec_revocation();
        match result {
            Ok(()) => ::borderless::info!("execution successful"),
            Err(e) => ::borderless::error!("execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn http_get_state() {
        let result = #derived::exec_get_state();
        if let Err(e) = result {
            ::borderless::error!("http-get failed: {e:?}");
        }
    }

    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn http_post_action() {
        let result = #derived::exec_post_action();
        if let Err(e) = result {
            ::borderless::error!("http-post failed: {e:?}");
        }
    }

    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn get_symbols() {
        let result = #derived::get_symbols();
        if let Err(e) = result {
            ::borderless::error!("get-symbols failed: {e:?}");
        }
    }
    }
}

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

fn match_action(
    action_names: &[String],
    action_ids: &[u32],
    call_fns: &[TokenStream2],
) -> TokenStream2 {
    quote! {
    match &action.method {
        MethodOrId::ByName { method } => match method.as_str() {
            #(
            #action_names => {
                #call_fns
            }
            )*
            other => { return Err(new_error!("Unknown method: {other}")) }
        }
        MethodOrId::ById { method_id } => match method_id {
            #(
            #action_ids => {
                #call_fns
            }
            )*
            other => { return Err(new_error!("Unknown method-id: 0x{other:04x}")) }
        }
    }
    }
}

fn impl_actions_enum(actions: &[ActionFn]) -> TokenStream2 {
    let fields = actions.iter().map(ActionFn::gen_field);
    let match_items = actions.iter().map(ActionFn::gen_field_match);
    quote! {
        #[allow(private_interfaces)]
        pub enum Actions {
            #( #fields ),*
        }

        #[automatically_derived]
        impl TryFrom<Actions> for ::borderless::events::CallAction {
            type Error = ::borderless::serialize::Error;

            fn try_from(value: Actions) -> ::std::result::Result<::borderless::events::CallAction, Self::Error> {
                let action = match value {
                    #(
                    #match_items
                    )*
                };
                Ok(action)
            }
        }
    }
}
