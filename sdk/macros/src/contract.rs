use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Error, FnArg, ImplItem, ItemImpl, Pat, PatIdent, Result, ReturnType, Type};
use syn::{Ident, Item};
use xxhash_rust::const_xxh3::xxh3_64;

use crate::utils::{check_if_action, check_if_state};

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
    let check_action: Vec<_> = actions.iter().map(ActionFn::gen_check_tokens).collect();
    let action_names: Vec<_> = actions.iter().map(ActionFn::method_name).collect();
    let action_ids: Vec<_> = actions.iter().map(ActionFn::method_id).collect();

    // TODO: Check that arguments for all methods are serializable

    let as_state = quote! {
        <#state as ::borderless::__private::storage_traits::State>
    };

    let read_state = quote! {
        let mut state = #as_state::load()?;
    };

    let commit_state = quote! {
        #as_state::commit(state);
    };

    // Matches the action (by either id or method) and calls the action fn
    let match_and_call_action = match_action(&action_names, &action_ids, &call_action);
    let match_and_check_action = match_action(&action_names, &action_ids, &check_action);

    let exec_post = quote! {
        #[automatically_derived]
        fn post_action_response(path: String, payload: Vec<u8>) -> Result<CallAction> {
            let path = path.replace("-", "_"); // Convert from kebab-case to snake_case
            let path = path.strip_prefix('/').unwrap_or(&path); // stip leading "/"

            let content = String::from_utf8(payload.clone()).unwrap_or_default();
            info!("{content}");

            // TODO: Check access of writer
            match path {
                "" => {
                    let action = CallAction::from_bytes(&payload).context("failed to parse action")?;
                    let payload = &action.params;
                    #match_and_check_action
                    // At this point, the action is validated and can be returned
                    Ok(action)
                }
                // TODO: Re-Design the action tokens, so you can work with this
                "flip_switch" => {
                    let args: FlipSwitchArgs = ::borderless::serialize::from_slice(&payload)?;
                    let value = borderless::serialize::to_value(&args)?;
                    Ok(CallAction::by_method("flip_switch", value))
                }
                "set_switch" => {
                    let args: SetSwitchArgs = ::borderless::serialize::from_slice(&payload)?;
                    let value = ::borderless::serialize::to_value(&args)?;
                    Ok(CallAction::by_method("set_switch", value))
                }
                "print_env" => {
                    let args: FlipSwitchArgs = ::borderless::serialize::from_slice(&payload)?;
                    let value = ::borderless::serialize::to_value(&args)?;
                    Ok(CallAction::by_method("print_env", value))
                }
                other => Err(new_error!("unknown method: {other}")),
            }
        }
    };

    let wasm_impl = quote! {
        #exec_post

        #[automatically_derived]
        pub fn exec_txn() -> Result<()> {
            #read_input

            let action = CallAction::from_bytes(&input)?;
            let s = action.pretty_print()?;
            info!("{s}");
            #read_state
            #match_and_call_action
            #commit_state
            Ok(())
        }

        #[automatically_derived]
        pub fn exec_introduction() -> Result<()> {
            #read_input
            let introduction = Introduction::from_bytes(&input)?;
            let s = introduction.pretty_print()?;
            info!("{s}");
            let state = #as_state::init(introduction.initial_state)?;
            #commit_state
            Ok(())
        }

        #[automatically_derived]
        pub fn exec_revocation() -> Result<()> {
            #read_input
            let r = Revocation::from_bytes(&input)?;
            info!("Revoked contract. Reason: {}", r.reason);
            Ok(())
        }

        #[automatically_derived]
        pub fn exec_get_state() -> Result<()> {
            let path = read_string_from_register(REGISTER_INPUT_HTTP_PATH).context("missing http-path")?;
            let result = #as_state::http_get(path)?;
            let status: u16 = if result.is_some() { 200 } else { 404 };
            let payload = result.unwrap_or_default();
            write_register(REGISTER_OUTPUT_HTTP_STATUS, status.to_be_bytes());
            write_string_to_register(REGISTER_OUTPUT_HTTP_RESULT, payload);
            Ok(())
        }

        #[automatically_derived]
        pub fn exec_post_action() -> Result<()> {
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

    Ok(quote! {
        #[doc(hidden)]
        #[automatically_derived]
        pub(super) mod __derived {
            use super::*;
            use ::borderless::*;
            use ::borderless::__private::{
                read_field, read_register, read_string_from_register, registers::*,
                storage_keys::make_user_key, write_field, write_register, write_string_to_register,
            };
            use ::borderless::contract::*;

            #[doc(hidden)]
            #[automatically_derived]
            const ACTION_SYMBOLS: &[(&str, u32)] = &[
                #(
                    (#action_names, #action_ids)
                ),*
            ];

            #(#action_types)*
            #wasm_impl
        }
    })
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

fn match_action(
    action_names: &[String],
    action_ids: &[u32],
    call_fns: &[TokenStream2],
) -> TokenStream2 {
    quote! {
    match action.method {
        MethodOrId::ByName { method } => match method {
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

// TODO:
#[allow(unused)]
/// Helper struct to bundle all necessary information for our action-functions
struct ActionFn {
    /// Ident (name) of the action
    ident: Ident,
    /// Associated method-id - either calculated or overriden by the user
    method_id: u32,
    /// true, if the method-id was set manually
    id_override: bool,
    /// Weather or not the function requires &mut self
    mut_self: bool,
    /// Return type of the function
    output: ReturnType,
    /// Arguments of the function
    args: Vec<(Ident, Box<Type>)>,
}

impl ActionFn {
    fn gen_type_tokens(&self) -> TokenStream2 {
        let args_ident = self.args_ident();
        if self.args.is_empty() {
            quote! {
                #[doc(hidden)]
                #[automatically_derived]
                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct #args_ident ;
            }
        } else {
            let fields = self.args.iter().map(|a| a.0.clone());
            let types = self.args.iter().map(|a| a.1.clone());
            quote! {
                #[doc(hidden)]
                #[automatically_derived]
                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct #args_ident {
                    #(
                        #fields: #types
                    ),*
                }
            }
        }
    }

    /// Generates the parsing of the function arguments + calling of the associated state function.
    ///
    /// References 'action' and 'state' in generated tokens
    fn gen_call_tokens(&self, state_ident: &Ident) -> TokenStream2 {
        let args_ident = self.args_ident();
        let fn_ident = &self.ident;
        let mut_state = if self.mut_self {
            quote! { &mut state }
        } else {
            quote! { &state }
        };
        if self.args.is_empty() {
            quote! {
                #state_ident::#fn_ident(#mut_state);
            }
        } else {
            let arg_idents = self.args.iter().map(|a| a.0.clone());
            quote! {
                let args: __derived::#args_ident = ::borderless::serialize::from_value(action.params)?;
                #state_ident::#fn_ident(#mut_state, #(args.#arg_idents),*);
            }
        }
    }

    /// Generates the check, to parse the args from action.params (used in the http-post function)
    ///
    /// References 'action' in generated tokens
    fn gen_check_tokens(&self) -> TokenStream2 {
        let args_ident = self.args_ident();
        quote! {
            let _args: __derived::#args_ident = ::borderless::serialize::from_value(action.params)?;
        }
    }

    fn args_ident(&self) -> Ident {
        format_ident!("__{}Args", self.ident.to_string().to_case(Case::Pascal))
    }

    fn method_name(&self) -> String {
        self.ident.to_string()
    }

    fn method_id(&self) -> u32 {
        self.method_id
    }
}

fn get_actions(state_ident: &Ident, items: &[Item]) -> Result<Vec<ActionFn>> {
    // First, get the impl-block of our state:
    for item in items {
        // Filter out everything irrelevant
        let item_impl = match item {
            Item::Impl(i) => i,
            _ => continue,
        };
        let type_path = match item_impl.self_ty.as_ref() {
            Type::Path(p) => p,
            _ => continue,
        };
        match type_path.path.segments.last() {
            Some(last_segment) => {
                if last_segment.ident != *state_ident {
                    continue;
                }
            }
            _ => continue,
        }
        // Then extract the actions from it
        return get_actions_from_impl(state_ident, item_impl);
    }
    Err(Error::new_spanned(
        state_ident,
        format!("No impl Block defined for '{state_ident}'"),
    ))
}

fn get_actions_from_impl(state_ident: &Ident, impl_block: &ItemImpl) -> Result<Vec<ActionFn>> {
    let mut actions = Vec::new();
    for item in impl_block.items.iter() {
        let impl_fn = match item {
            ImplItem::Fn(f) => f,
            _ => continue,
        };
        let mut is_action = false;
        for attr in impl_fn.attrs.iter() {
            if check_if_action(attr) {
                is_action = true;
                break;
            }
        }
        if !is_action {
            continue;
        }
        // At this point, the function is an action
        let mut has_self = false;
        let mut mut_self = false;
        let mut args: Vec<(Ident, Box<Type>)> = Vec::new();
        for input in impl_fn.sig.inputs.iter() {
            match input {
                FnArg::Receiver(s) => {
                    has_self = true;
                    mut_self = s.mutability.is_some();
                    if s.reference.is_none() {
                        return Err(Error::new_spanned(
                            item,
                            "Action functions must not consume state - use either &self or &mut self",
                        ));
                    }
                }
                FnArg::Typed(t) => {
                    // Extract the name from the pattern
                    if let Pat::Ident(PatIdent { ident, .. }) = &*t.pat {
                        args.push((ident.clone(), t.ty.clone()));
                    } else {
                        return Err(Error::new_spanned(
                            &t.pat,
                            "Only simple named arguments are supported in action functions",
                        ));
                    }
                }
            }
        }
        if !has_self {
            return Err(Error::new_spanned(
                item,
                "Action functions must act on state - so either &self or &mut self",
            ));
        }

        // TODO: Check, if the action has a method-id assigned to it
        // TODO: Check, that there are no actions with the same ID
        let method_id = calc_method_id(state_ident, &impl_fn.sig.ident);
        actions.push(ActionFn {
            ident: impl_fn.sig.ident.clone(),
            method_id,
            id_override: false,
            mut_self,
            output: impl_fn.sig.output.clone(),
            args,
        });
    }
    if actions.is_empty() {
        Err(Error::new_spanned(
            impl_block,
            format!("No actions defined for '{state_ident}'"),
        ))
    } else {
        Ok(actions)
    }
}

/// Calculate the method-id based on the state and action name.
fn calc_method_id(state_ident: &Ident, action_ident: &Ident) -> u32 {
    let full_name = format!("{}::{}", state_ident, action_ident).to_uppercase();
    // Since we only want 32-bits, we simply truncate the output:
    xxh3_64(full_name.as_bytes()) as u32
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
