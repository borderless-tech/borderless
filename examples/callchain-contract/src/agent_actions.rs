use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Actions {
    IncreaseProcess { number: u32 },
    IncreaseContract { number: u32 },
}

#[derive(Serialize, Deserialize)]
struct IncreaseProcessArgs {
    number: u32,
}

#[derive(Serialize, Deserialize)]
struct IncreaseContractArgs {
    number: u32,
}

#[automatically_derived]
impl TryFrom<Actions> for ::borderless::events::CallAction {
    type Error = ::borderless::serialize::Error;
    fn try_from(
        value: Actions,
    ) -> ::std::result::Result<::borderless::events::CallAction, Self::Error> {
        let action = match value {
            Actions::IncreaseProcess { number } => {
                let args = IncreaseProcessArgs { number };
                let args_value = ::borderless::serialize::to_value(&args)?;
                ::borderless::events::CallAction::by_method("increase_process", args_value)
            }
            Actions::IncreaseContract { number } => {
                let args = IncreaseContractArgs { number };
                let args_value = ::borderless::serialize::to_value(&args)?;
                ::borderless::events::CallAction::by_method("increase_contract", args_value)
            }
        };
        Ok(action)
    }
}
