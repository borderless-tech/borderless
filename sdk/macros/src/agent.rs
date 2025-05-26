use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Ident, Item, Result};

use crate::{
    action::{get_actions, impl_actions_enum, match_action, ActionFn},
    schedule::get_schedules,
    state::get_state,
};

// TODO: Add attributes
pub fn parse_module_content(
    mod_span: Span,
    mod_items: &[Item],
    _mod_ident: &Ident,
) -> Result<TokenStream2> {
    let read_input = quote! {
        let input = read_register(REGISTER_INPUT).context("missing input register")?;
    };

    let state = get_state(mod_items, &mod_span)?;
    let mut actions = get_actions(&state, mod_items)?;

    let schedules = get_schedules(&state, mod_items)?;

    // Since schedules are also treated as actions, just add them to the list of actions.
    // TODO: Duplicate check !
    actions.extend(schedules.iter().map(|s| s.to_action()));

    let schedules = schedules.into_iter().map(|s| s.into_schedule_tokens());

    let action_types = actions.iter().map(ActionFn::gen_type_tokens);

    let call_action: Vec<_> = actions.iter().map(|a| a.gen_call_tokens(&state)).collect(); // <- TODO: This contains logic only for contracts !
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
    let _check_payload: Vec<_> = actions
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
    let _match_and_check_action = match_action(&action_names, &action_ids, &check_action);

    let exec_post = quote! {
        #[automatically_derived]
        fn post_action_response(path: String, payload: Vec<u8>) -> Result<CallAction> {
            todo!("implement calling actions on agents")
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

    let exec_basic_fns = quote! {
        #[automatically_derived]
        pub(crate) fn exec_action() -> Result<()> {
            #read_input

            let action = CallAction::from_bytes(&input)?;
            let s = action.pretty_print()?;
            info!("{s}");
            let mut state = #as_state::load()?;
            #match_and_call_action
            let events = _match_result?;
            if !events.is_empty() {
                let bytes = events.to_bytes()?;
                write_register(REGISTER_OUTPUT, &bytes);
            }
            #as_state::commit(state);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_introduction() -> Result<()> {
            #read_input
            // TODO: Use different introduction type for agents
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
            info!("Revoked agent. Reason: {}", r.reason);
            Ok(())
        }
    };

    let exec_init_shutdown = quote! {
        #[automatically_derived]
        pub(crate) fn exec_init() -> Result<()> {
            // TODO: Websocket
            let mut my_init = ::borderless::agents::Init {
                schedules: Vec::new(),
                ws_config: None,
            };
            #(
            my_init.schedules.push(#schedules);
            )*
            // Write output
            let bytes = my_init.to_bytes()?;
            write_register(REGISTER_OUTPUT, &bytes);
            Ok(())
        }

        #[automatically_derived]
        pub(crate) fn exec_shutdown() -> Result<()> {
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
            // TODO: Should we really do a separate handling here, or should we use the normal "exec_action" way ?
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

    let as_ws_handler = quote! {
        <#state as ::borderless::agents::WebsocketHandler>
    };

    let exec_ws = quote! {
        #[automatically_derived]
        pub(crate) fn on_ws_open() -> Result<()> {
            // Load state
            let mut state = #as_state::load()?;
            let action_output = #as_ws_handler::on_open(&mut state).map_err(::borderless::Error::msg)?.unwrap_or_default();
            let events = action_output.convert_out_events()?;
            if !events.is_empty() {
                let bytes = events.to_bytes()?;
                write_register(REGISTER_OUTPUT, &bytes);
            }
            // Commit state
            #as_state::commit(state);
            Ok(())
        }

        #[automatically_derived]
        pub(crate) fn on_ws_msg() -> Result<()> {
            #read_input

            // Load state
            let mut state = #as_state::load()?;
            let action_output = #as_ws_handler::on_message(&mut state, input).map_err(::borderless::Error::msg)?.unwrap_or_default();
            let events = action_output.convert_out_events()?;
            if !events.is_empty() {
                let bytes = events.to_bytes()?;
                write_register(REGISTER_OUTPUT, &bytes);
            }
            // Commit state
            #as_state::commit(state);
            Ok(())
        }

        #[automatically_derived]
        pub(crate) fn on_ws_close() -> Result<()> {
            // Load state
            let mut state = #as_state::load()?;
            let action_output = #as_ws_handler::on_close(&mut state).map_err(::borderless::Error::msg)?.unwrap_or_default();
            let events = action_output.convert_out_events()?;
            if !events.is_empty() {
                let bytes = events.to_bytes()?;
                write_register(REGISTER_OUTPUT, &bytes);
            }
            // Commit state
            #as_state::commit(state);
            Ok(())
        }

        #[automatically_derived]
        pub(crate) fn on_ws_error() -> Result<()> {
            // Load state
            let mut state = #as_state::load()?;
            let action_output = #as_ws_handler::on_error(&mut state).map_err(::borderless::Error::msg)?.unwrap_or_default();
            let events = action_output.convert_out_events()?;
            if !events.is_empty() {
                let bytes = events.to_bytes()?;
                write_register(REGISTER_OUTPUT, &bytes);
            }
            // Commit state
            #as_state::commit(state);
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
            #exec_basic_fns
            #exec_init_shutdown
            #exec_http
            #exec_ws
        }

        pub(super) mod actions {
            use super::__derived::*;
            #actions_enum
        }
    };
    Ok(derived)
}

pub fn generate_ws_wasm_exports(mod_ident: &Ident) -> TokenStream2 {
    let derived = quote! { #mod_ident::__derived };

    quote! {
    #[no_mangle]
    pub extern "C" fn on_ws_open() {
        let result = #derived::on_ws_open();
        match result {
            Ok(()) => ::borderless::info!("on_ws_open: success."),
            Err(e) => ::borderless::error!("on_ws_open execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn on_ws_msg() {
        let result = #derived::on_ws_msg();
        match result {
            Ok(()) => ::borderless::info!("on_ws_msg: success."),
            Err(e) => ::borderless::error!("on_ws_msg execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn on_ws_error() {
        let result = #derived::on_ws_error();
        match result {
            Ok(()) => ::borderless::info!("on_ws_error: success."),
            Err(e) => ::borderless::error!("on_ws_error execution failed: {e:?}"),
        }
    }

    #[no_mangle]
    pub extern "C" fn on_ws_close() {
        let result = #derived::on_ws_close();
        match result {
            Ok(()) => ::borderless::info!("on_ws_close: success."),
            Err(e) => ::borderless::error!("on_ws_close execution failed: {e:?}"),
        }
    }
    }
}

pub fn generate_wasm_exports(mod_ident: &Ident) -> TokenStream2 {
    let derived = quote! { #mod_ident::__derived };

    quote! {
    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn on_init() {
        let result = #derived::exec_init();
        match result {
            Ok(()) => ::borderless::info!("initialization successful"),
            Err(e) => ::borderless::error!("initialization failed: {e:?}"),
        }
    }

    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn on_shutdown() {
        let result = #derived::exec_shutdown();
        match result {
            Ok(()) => ::borderless::info!("shutdown successful"),
            Err(e) => ::borderless::error!("shutdown failed: {e:?}"),
        }
    }

    #[no_mangle]
    #[automatically_derived]
    pub extern "C" fn process_action() {
        let result = #derived::exec_action();
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
