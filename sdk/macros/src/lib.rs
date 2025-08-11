use agent::AgentArgs;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item, ItemMod};

mod action;
mod agent;
mod contract;
mod schedule;
mod state;
mod utils;

// TODO's:
// - [ ] Check existence of serde crate
// - [ ] Check re-naming of borderless crate

#[proc_macro_attribute]
pub fn contract(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let module = parse_macro_input!(input as ItemMod);

    // Check if module has some content
    if module.content.is_none() {
        return syn::Error::new_spanned(
            module,
            "Macro can only be implemented on modules that have some content",
        )
        .to_compile_error()
        .into();
    }
    let (brace, mut items) = module.content.unwrap();

    // Generate new tokens based on the module's content
    let new_tokens = match contract::parse_module_content(brace.span.join(), &items, &module.ident)
    {
        Ok(tokens) => tokens,
        Err(e) => return e.to_compile_error().into(),
    };

    // Add these new tokens to the existing items
    items.push(Item::Verbatim(new_tokens));

    // Also generate an error if the visibility is not public
    match module.vis {
        syn::Visibility::Public(_) => (),
        _ => {
            let tokens =
                syn::Error::new_spanned(module.mod_token, "Contract module must be public")
                    .to_compile_error();
            items.push(Item::Verbatim(tokens));
        }
    }

    // let wasm_exports = contract::generate_wasm_exports(&module.ident);
    let wasm_exports = quote! {};

    // Generate a new module from the content of the original module
    let new_module = ItemMod {
        attrs: module.attrs,
        vis: module.vis,
        unsafety: module.unsafety,
        mod_token: module.mod_token,
        ident: module.ident,
        content: Some((brace, items)),
        semi: module.semi,
    };

    // Convert into token stream
    quote! {
        #new_module
        #wasm_exports
    }
    .into()
}

#[proc_macro_attribute]
pub fn agent(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let module = parse_macro_input!(input as ItemMod);
    let parsed_attrs = syn::parse_macro_input!(attrs as AgentArgs);

    // Check if module has some content
    if module.content.is_none() {
        return syn::Error::new_spanned(
            module,
            "Macro can only be implemented on modules that have some content",
        )
        .to_compile_error()
        .into();
    }
    let (brace, mut items) = module.content.unwrap();

    // If the agent has access to the websocket or not depends on the attributes
    let use_ws = parsed_attrs.websocket.unwrap_or_default();

    // Generate new tokens based on the module's content
    let new_tokens = match agent::parse_module_content(brace.span.join(), &items, use_ws) {
        Ok(tokens) => tokens,
        Err(e) => return e.to_compile_error().into(),
    };

    // Add these new tokens to the existing items
    items.push(Item::Verbatim(new_tokens));

    // Also generate an error if the visibility is not public
    match module.vis {
        syn::Visibility::Public(_) => (),
        _ => {
            let tokens = syn::Error::new_spanned(module.mod_token, "Agent module must be public")
                .to_compile_error();
            items.push(Item::Verbatim(tokens));
        }
    }

    let wasm_exports = agent::generate_wasm_exports(&module.ident);

    // Generate websocket tokens, if websocket == true
    let wasm_ws_exports = if use_ws {
        agent::generate_ws_wasm_exports(&module.ident)
    } else {
        quote! {}
    };

    // Generate a new module from the content of the original module
    let new_module = ItemMod {
        attrs: module.attrs,
        vis: module.vis,
        unsafety: module.unsafety,
        mod_token: module.mod_token,
        ident: module.ident,
        content: Some((brace, items)),
        semi: module.semi,
    };

    // Convert into token stream
    quote! {
        #new_module
        #wasm_exports
        #wasm_ws_exports
    }
    .into()
}

#[proc_macro_derive(State)]
pub fn derive_contract_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let output = state::impl_state(input);

    match output {
        syn::Result::Ok(token_stream) => token_stream,
        syn::Result::Err(err) => err.to_compile_error(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn action(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn schedule(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    input
}
