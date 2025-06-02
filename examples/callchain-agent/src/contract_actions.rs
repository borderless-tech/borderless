// GENERATED CODE - DO NOT TOUCH
//
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Actions {
    SetNumber { number: u32 },
    CallNext {},
}

#[derive(Serialize, Deserialize)]
struct SetNumberArgs {
    number: u32,
}

#[derive(Serialize, Deserialize)]
struct CallNextArgs {}

impl TryFrom<Actions> for ::borderless::events::CallAction {
    type Error = ::borderless::serialize::Error;
    fn try_from(
        value: Actions,
    ) -> ::std::result::Result<::borderless::events::CallAction, Self::Error> {
        let action = match value {
            Actions::SetNumber { number } => {
                let args = SetNumberArgs { number };
                let args_value = ::borderless::serialize::to_value(&args)?;
                ::borderless::events::CallAction::by_method("set_number", args_value)
            }
            Actions::CallNext {} => {
                let args = CallNextArgs {};
                let args_value = ::borderless::serialize::to_value(&args)?;
                ::borderless::events::CallAction::by_method("call_next", args_value)
            }
        };
        Ok(action)
    }
}
