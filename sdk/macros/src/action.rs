use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Error, FnArg, ImplItem, ItemImpl, LitBool, LitStr, Meta, Pat, PatIdent, Result, ReturnType,
    Token, Type,
};
use syn::{Ident, Item};
use xxhash_rust::const_xxh3::xxh3_64;

use crate::utils::check_if_action;

/// Helper struct to bundle all necessary information for our action-functions
pub struct ActionFn {
    /// Ident (name) of the action
    ident: Ident,
    /// Associated method-id - either calculated or overriden by the user
    method_id: u32,
    /// Indicates that the method-name does not match the function name
    name_override: Option<String>,
    /// Weather or not the action should be exposed via api
    web_api: bool,
    /// Roles, that are allowed to call the function. Empty means no restrictions.
    roles: Vec<String>,
    /// Weather or not the function requires &mut self
    mut_self: bool,
    /// Return type of the function
    output: ReturnType,
    /// Arguments of the function
    args: Vec<(Ident, Box<Type>)>,
    _span: Span,
}

impl ActionFn {
    pub fn gen_field(&self) -> TokenStream2 {
        let fields = self.args.iter().map(|a| a.0.clone());
        let types = self.args.iter().map(|a| a.1.clone());
        let ident = self.field_ident();
        quote! {
            #ident { #( #fields: #types ),* }
        }
    }

    pub fn gen_field_match(&self) -> TokenStream2 {
        let fields: Vec<_> = self.args.iter().map(|a| a.0.clone()).collect();
        let field_ident = self.field_ident();
        let method_name = self.method_name();
        let args_ident = self.args_ident(); // NOTE: This requires use super::__derived::*;
        quote! {
            Actions::#field_ident { #(#fields),* } => {
                let args = #args_ident { #(#fields),* };
                let args_value = ::borderless::serialize::to_value(&args)?;
                ::borderless::events::CallAction::by_method(#method_name, args_value)
            }
        }
    }

    pub fn gen_type_tokens(&self) -> TokenStream2 {
        let args_ident = self.args_ident();
        let fields = self.args.iter().map(|a| a.0.clone());
        let types = self.args.iter().map(|a| a.1.clone());
        quote! {
            #[doc(hidden)]
            #[automatically_derived]
            #[derive(serde::Serialize, serde::Deserialize)]
            pub(crate) struct #args_ident {
                #(
                    pub(crate) #fields: #types
                ),*
            }
        }
    }

    /// Generates the parsing of the function arguments + calling of the associated state function.
    ///
    /// References 'action' and 'state' in generated tokens
    pub fn gen_call_tokens(&self, state_ident: &Ident) -> TokenStream2 {
        let args_ident = self.args_ident();
        let fn_ident = &self.ident;
        let return_type = match &self.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ty) => quote! { #ty },
        };
        let mut_state = if self.mut_self {
            quote! { &mut state }
        } else {
            quote! { &state }
        };
        let access_check = self.writer_access();
        if self.args.is_empty() {
            quote! {
                #access_check
                let result = #state_ident::#fn_ident(#mut_state);
                <#return_type as ::borderless::events::ActionOutEvent>::convert_out_events(result)
            }
        } else {
            let arg_idents = self.args.iter().map(|a| a.0.clone());
            quote! {
                #access_check
                let args: __derived::#args_ident = ::borderless::serialize::from_value(action.params)?;
                let result = #state_ident::#fn_ident(#mut_state, #(args.#arg_idents),*);
                <#return_type as ::borderless::events::ActionOutEvent>::convert_out_events(result)
            }
        }
    }

    /// Generates the check, to parse the args from action.params (used in the http-post function)
    ///
    /// References 'action' in generated tokens
    pub fn gen_check_tokens(&self, value: TokenStream2) -> TokenStream2 {
        let args_ident = self.args_ident();
        let access_check = self.writer_access();
        if self.web_api {
            quote! {
                #access_check
                let _args: __derived::#args_ident = #value;
            }
        } else {
            let err_msg = format!("action '{}' cannot be called via web-api", self.ident);
            quote! {
                return Err(new_error!(#err_msg));
                let _args: __derived::#args_ident = #value;
            }
        }
    }

    /// Returns the method name
    ///
    /// This is either the name of the function, or the user-defined method-name
    /// that was passed to the action macro.
    pub fn method_name(&self) -> String {
        if let Some(rename) = &self.name_override {
            rename.clone()
        } else {
            self.ident.to_string()
        }
    }

    pub fn method_id(&self) -> u32 {
        self.method_id
    }

    fn writer_access(&self) -> TokenStream2 {
        let roles = self.roles.iter();
        if self.roles.is_empty() {
            quote! { /* no access restriction */ }
        } else {
            let fn_name = self.ident.to_string();
            quote! {
                let writer_roles = ::borderless::contracts::env::writer_roles();
                if !writer_roles.iter().any(|role| #( role.eq_ignore_ascii_case(#roles) )||* ) {
                    let writer = ::borderless::contracts::env::writer();
                    return Err(new_error!("writer {} has no access to action '{}'", writer, #fn_name));
                }
            }
        }
    }

    fn args_ident(&self) -> Ident {
        format_ident!("__{}Args", self.ident.to_string().to_case(Case::Pascal))
    }

    fn field_ident(&self) -> Ident {
        format_ident!("{}", self.ident.to_string().to_case(Case::Pascal))
    }
}

/// Returns a list of parsed `ActionFn`
///
/// Takes the identifier of the `State` and the list of module Items as input.
pub fn get_actions(state_ident: &Ident, mod_items: &[Item]) -> Result<Vec<ActionFn>> {
    // First, get the impl-block of our state:
    for item in mod_items {
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
        let mut action_args = None;
        for attr in impl_fn.attrs.iter() {
            if check_if_action(attr) {
                let args = if let Meta::List(list) = &attr.meta {
                    (
                        syn::parse2::<ActionArgs>(list.tokens.clone())?,
                        list.tokens.span(),
                    )
                } else {
                    (ActionArgs::default(), attr.span())
                };
                action_args = Some(args);
                break;
            }
        }
        // Ignore, if parsing went wrong
        let (action_args, args_span) = match action_args {
            Some(a) => a,
            None => continue,
        };
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

        // Check, that there are no actions with the same ID
        let method_id = calc_method_id(state_ident, &impl_fn.sig.ident, action_args.rename.clone());
        if let Some(rename) = &action_args.rename {
            if !rename.is_case(Case::Snake) {
                return Err(Error::new(
                    args_span,
                    format!("invalid method name '{rename}' - method names must be snake_case"),
                ));
            }
        }
        if let Some(a) = actions
            .iter()
            .find(|a: &&ActionFn| a.method_id == method_id)
        {
            return Err(Error::new_spanned(
                impl_fn,
                format!(
                    "duplicate method-id for {} and {} - this is likely because you used 'rename' on one action",
                    a.ident, impl_fn.sig.ident
                ),
            ));
        }
        actions.push(ActionFn {
            ident: impl_fn.sig.ident.clone(),
            method_id,
            name_override: action_args.rename,
            web_api: action_args.expose_api,
            roles: action_args.roles,
            mut_self,
            output: impl_fn.sig.output.clone(),
            args,
            _span: impl_fn.span(),
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
fn calc_method_id(state_ident: &Ident, action_ident: &Ident, rename: Option<String>) -> u32 {
    let action_name = rename.unwrap_or_else(|| action_ident.to_string());
    let full_name = format!("{}::{}", state_ident, action_name).to_uppercase();
    // Since we only want 32-bits, we simply truncate the output:
    xxh3_64(full_name.as_bytes()) as u32
}

pub fn impl_actions_enum(actions: &[ActionFn]) -> TokenStream2 {
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

// #[derive(Debug, FromMeta)]
// pub struct ActionArgs {
//     #[darling(default)]
//     pub rename: Option<String>,

//     #[darling(default, rename = "web_api")]
//     pub web_api: bool,

//     #[darling(default)]
//     pub roles: String,
// }
//
#[allow(dead_code)]
struct Rename {
    id_token: kw::rename,
    eq_token: Token![=],
    value: LitStr,
}

#[allow(dead_code)]
struct Roles {
    id_token: kw::roles,
    eq_token: Token![=],
    values: Vec<String>,
}

#[allow(dead_code)]
struct ExposeApi {
    web_api_token: kw::web_api,
    eq_token: Token![=],
    value: LitBool,
}

#[allow(dead_code)]
struct ActionArgs {
    rename: Option<String>,
    expose_api: bool,
    roles: Vec<String>,
}

impl Default for ActionArgs {
    fn default() -> Self {
        Self {
            rename: None,
            expose_api: true,
            roles: Vec::new(),
        }
    }
}

impl Parse for Rename {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Rename {
            id_token: input.parse::<kw::rename>()?,
            eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl Parse for ExposeApi {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ExposeApi {
            web_api_token: input.parse::<kw::web_api>()?,
            eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl Parse for Roles {
    fn parse(input: ParseStream) -> Result<Self> {
        let id_token = input.parse::<kw::roles>()?;
        let eq_token = input.parse()?;
        let value: LitStr = input.parse()?;
        let values = value.value().split(',').map(ToString::to_string).collect();
        Ok(Roles {
            id_token,
            eq_token,
            values,
        })
    }
}

impl Parse for ActionArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rename = None;
        let mut expose_api = false;
        let mut roles = Vec::new();

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::rename) {
                let parsed: Rename = input.parse()?;
                rename = Some(parsed.value.value());
            } else if lookahead.peek(kw::web_api) {
                let parsed: ExposeApi = input.parse()?;
                expose_api = parsed.value.value();
            } else if lookahead.peek(kw::roles) {
                let parsed: Roles = input.parse()?;
                roles = parsed.values;
            } else {
                return Err(lookahead.error());
            }
            // If there is still something to parse, it should be separated by a ","
            if !input.is_empty() {
                let _sep: Token![,] = input.parse()?;
            }
        }

        Ok(Self {
            rename,
            expose_api,
            roles,
        })
    }
}

mod kw {
    syn::custom_keyword!(rename);
    syn::custom_keyword!(roles);
    syn::custom_keyword!(web_api);
}
