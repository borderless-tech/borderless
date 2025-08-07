mod contract_actions;

#[borderless::agent]
pub mod cc_agent {
    use borderless::contracts::env;
    use borderless::prelude::{json, message, ContractCall, Message};
    use borderless::*;
    use serde::{Deserialize, Serialize};

    // --- This is the code that the user writes
    #[derive(State, Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub struct CC {
        pub last_number: u32,
    }

    impl CC {
        /// Increases the number and calls the next process
        #[action]
        pub fn increase_process(&mut self, number: u32) -> Result<Message, Error> {
            self.last_number = number;
            let value = json!({"number": self.last_number + 1});
            let msg = message("TOPIC").with_value(value);
            Ok(msg)
        }

        /// Increases the number and calls the next contract
        #[action]
        pub fn increase_contract(&mut self, number: u32) -> Result<ContractCall, Error> {
            self.last_number = number;
            let value = json!({"number": self.last_number + 1});
            let call = env::sink("CONTRACT")?
                .call_method("set_number")
                .with_value(value)
                .build()?;
            Ok(call)
        }
    }
}
