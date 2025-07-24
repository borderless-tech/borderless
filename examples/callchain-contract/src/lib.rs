/// Module to test the interaction chain contract->process->process->contract
///
/// We do this by initializing a contract with a number, and feeding that number + 1 into a process.
/// Each processing entity will add +1 to the number, and then we check if the number has increased exactly 3 times
// This contract is the "sink" at the end and also the "source" -> we go into circles here
mod agent_actions;

#[borderless::contract]
pub mod cc_contract {
    use borderless::events::Events;
    use borderless::*;
    use serde::{Deserialize, Serialize};

    // --- This is the code that the user writes
    #[derive(State, Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub struct CC {
        pub number: u32,
    }

    impl CC {
        /// Sets the number - is private so you cannot call it via API
        #[action(web_api = false)]
        pub fn set_number(&mut self, number: u32) {
            self.number = number;
        }

        /// Starts calling the process
        #[action(web_api = true)]
        pub fn call_next(&mut self) -> Result<Events> {
            todo!("Use new contract sink concept")
        }
    }
}
