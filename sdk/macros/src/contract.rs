use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Error, FnArg, ImplItem, ItemImpl, Pat, PatIdent, Result, ReturnType, Type};
use syn::{Ident, Item};

use crate::utils::{check_if_action, check_if_state};

pub fn parse_module_content(
    mod_span: Span,
    mod_items: &Vec<Item>,
    _mod_ident: &Ident,
) -> Result<TokenStream2> {
    let read_input = quote! {
        let input = read_register(REGISTER_INPUT).context("missing input register")?;
    };

    let state = get_state(&mod_items, &mod_span)?;

    let actions = get_actions(&state, &mod_items)?;

    let action_types = actions.iter().map(ActionFn::gen_type_tokens);

    let call_action = actions.iter().map(|a| a.gen_call_tokens(&state));

    let action_names = actions.iter().map(ActionFn::method_name);

    let read_state = quote! {
        let mut state = <#state as ::borderless::__private::storage_traits::State>::load()?;
    };

    let match_method = quote! {
        match method {
            #(
                #action_names => {
                    #call_action
                }
                _ => (),
            )*
        }
    };

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
            #(#action_types)*
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

/// Helper struct to bundle all necessary information for our action-functions
struct ActionFn {
    ident: Ident,
    mut_self: bool,
    output: ReturnType,
    args: Vec<(Ident, Box<Type>)>,
}

impl ActionFn {
    fn gen_type_tokens(&self) -> TokenStream2 {
        let ident = format_ident!("__{}Args", self.ident.to_string().to_case(Case::Pascal));
        if self.args.is_empty() {
            quote! {
                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct #ident ;
            }
        } else {
            let fields = self.args.iter().map(|a| a.0.clone());
            let types = self.args.iter().map(|a| a.1.clone());
            quote! {
                #[derive(serde::Serialize, serde::Deserialize)]
                pub struct #ident {
                    #(
                        #fields: #types
                    ),*
                }
            }
        }
    }

    // References 'action' and 'state' in generated tokens
    fn gen_call_tokens(&self, state_ident: &Ident) -> TokenStream2 {
        let args_ident = format_ident!("__{}Args", self.ident.to_string().to_case(Case::Pascal));
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

    fn method_name(&self) -> String {
        self.ident.to_string()
    }

    fn method_id(&self) -> u32 {
        todo!("generate method-id from function name or via attribute")
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
                    if !s.reference.is_some() {
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

        // TODO: Check that arguments are serializable
        actions.push(ActionFn {
            ident: impl_fn.sig.ident.clone(),
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
