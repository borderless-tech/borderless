use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Error, FnArg, ImplItem, ItemImpl, LitStr, Meta, Pat, PatIdent, Result, ReturnType, Token, Type,
};
use syn::{Ident, Item};

use crate::action::{calc_method_id, ActionFn};
use crate::utils::check_if_schedule;

/// Helper struct to bundle all necessary information for our schedule-functions
pub struct ScheduleFn {
    /// Ident (name) of the action
    ident: Ident,
    /// Associated method-id - either calculated or overriden by the user
    method_id: u32,
    /// Schedule delay in milliseconds
    delay_millis: Option<u64>,
    /// Schedule interval in milliseconds
    interval_millis: u64,
    /// Weather or not the function requires &mut self
    mut_self: bool,
    /// Return type of the function
    output: ReturnType,
    _span: Span,
}

impl ScheduleFn {
    /// Converts the schedule into an action
    ///
    /// Internally schedules are just actions - they are just parsed by a different macro.
    pub fn to_action(&self) -> ActionFn {
        ActionFn {
            ident: self.ident.clone(),
            method_id: self.method_id,
            name_override: None,
            web_api: false,
            roles: vec![],
            mut_self: self.mut_self,
            output: self.output.clone(),
            args: vec![],
            _span: self._span,
        }
    }

    /// Generates schedule tokens
    pub fn into_schedule_tokens(self) -> TokenStream2 {
        let method_id = self.method_id;
        let interval = self.interval_millis;
        let delay = self.delay_millis;
        quote! {
        ::borderless::agents::Schedule::by_method_id(#method_id, #interval, #delay)
        }
    }
}

/// Returns a list of parsed `ScheduleFn`
///
/// Takes the identifier of the `State` and the list of module Items as input.
pub fn get_schedules(state_ident: &Ident, mod_items: &[Item]) -> Result<Vec<ScheduleFn>> {
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
        return get_schedules_from_impl(state_ident, item_impl);
    }
    Err(Error::new_spanned(
        state_ident,
        format!("No impl Block defined for '{state_ident}'"),
    ))
}

fn get_schedules_from_impl(state_ident: &Ident, impl_block: &ItemImpl) -> Result<Vec<ScheduleFn>> {
    let mut schedules = Vec::new();
    for item in impl_block.items.iter() {
        let impl_fn = match item {
            ImplItem::Fn(f) => f,
            _ => continue,
        };
        let mut schedule_args = None;
        for attr in impl_fn.attrs.iter() {
            if check_if_schedule(attr) {
                let args = if let Meta::List(list) = &attr.meta {
                    (
                        syn::parse2::<ScheduleArgs>(list.tokens.clone())?,
                        list.tokens.span(),
                    )
                } else {
                    return Err(Error::new_spanned(
                        &attr,
                        "Schedules require an interval - e.g. interval = 5m",
                    ));
                };
                schedule_args = Some(args);
                break;
            }
        }
        // Ignore, if parsing went wrong
        let (schedule_args, _args_span) = match schedule_args {
            Some(a) => a,
            None => continue,
        };
        // At this point, the function is an action
        let mut has_self = false;
        let mut mut_self = false;
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
                        return Err(Error::new_spanned(
                            &ident,
                            "Schedules must not take any input parameters except 'self'",
                        ));
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
                "Schedule functions must act on state - so either &self or &mut self",
            ));
        }

        // Check, that there are no schedules with the same ID
        let method_id = calc_method_id(state_ident, &impl_fn.sig.ident, None);

        // TODO: We also require a duplicate check against the actions, which has to be done from the outside.
        if let Some(a) = schedules
            .iter()
            .find(|a: &&ScheduleFn| a.method_id == method_id)
        {
            return Err(Error::new_spanned(
                impl_fn,
                format!(
                    "duplicate method-id for {} and {} - this is likely because you used 'rename' on one action or schedule",
                    a.ident, impl_fn.sig.ident
                ),
            ));
        }
        schedules.push(ScheduleFn {
            ident: impl_fn.sig.ident.clone(),
            method_id,
            delay_millis: schedule_args.delay.map(|d| d.as_millis()),
            interval_millis: schedule_args.interval.as_millis(),
            mut_self,
            output: impl_fn.sig.output.clone(),
            _span: impl_fn.span(),
        });
    }
    // NOTE: It is okay, to define no schedules
    Ok(schedules)
}

#[derive(Debug)]
pub enum Time {
    Millis(u64),
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
}

impl Time {
    pub fn as_millis(&self) -> u64 {
        match self {
            Time::Millis(ms) => *ms,
            Time::Seconds(s) => s * 1000,
            Time::Minutes(m) => m * 60 * 1000,
            Time::Hours(h) => h * 3600 * 1000,
        }
    }
}

impl Parse for Time {
    fn parse(input: ParseStream) -> Result<Self> {
        let value: LitStr = input.parse()?;
        let value_str = value.value();

        if value_str.is_empty() {
            return Err(input.error("Invalid timestamp format"));
        }
        let (num_str, unit_str) = if value_str.ends_with("ms") {
            value_str.split_at(value_str.len() - 2)
        } else {
            value_str.split_at(value_str.len() - 1)
        };
        let num: u64 = num_str
            .parse()
            .map_err(|_| input.error("Invalid number format"))?;

        let time = match unit_str {
            "ms" => {
                if num < 250 {
                    return Err(input.error("schedule periods or delays cannot be set below 250ms"));
                }
                Time::Millis(num)
            }
            "s" => Time::Seconds(num),
            "m" => Time::Minutes(num),
            "h" => Time::Hours(num),
            _ => return Err(input.error("Invalid time unit - expected one of 'ms', 's', 'm', 'h'")),
        };

        Ok(time)
    }
}

#[allow(dead_code)]
struct Delay {
    id_token: kw::delay,
    eq_token: Token![=],
    value: Time,
}

#[allow(dead_code)]
struct Interval {
    id_token: kw::interval,
    eq_token: Token![=],
    value: Time,
}

#[allow(dead_code)]
struct ScheduleArgs {
    delay: Option<Time>,
    interval: Time,
}

impl Parse for Interval {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Interval {
            id_token: input.parse::<kw::interval>()?,
            eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl Parse for Delay {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Delay {
            id_token: input.parse::<kw::delay>()?,
            eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl Parse for ScheduleArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut delay = None;
        let mut interval = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::delay) {
                let parsed: Delay = input.parse()?;
                delay = Some(parsed.value);
            } else if lookahead.peek(kw::interval) {
                let parsed: Interval = input.parse()?;
                interval = Some(parsed.value);
            } else {
                return Err(lookahead.error());
            }
            // If there is still something to parse, it should be separated by a ","
            if !input.is_empty() {
                let _sep: Token![,] = input.parse()?;
            }
        }
        if interval.is_none() {
            return Err(input.error("schedules require an interval"));
        }

        Ok(Self {
            delay,
            interval: interval.unwrap(),
        })
    }
}

mod kw {
    syn::custom_keyword!(delay);
    syn::custom_keyword!(interval);
}
