mod contract_actions;

#[borderless::agent]
pub mod cc_agent {
    use borderless::events::Events;
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
        pub fn increase_process(&mut self, _number: u32) -> Result<Events, Error> {
            todo!("Implement messages system")
        }

        /// Increases the number and calls the next contract
        #[action]
        pub fn increase_contract(&mut self, _number: u32) -> Result<Events, Error> {
            todo!("Implement messages system")
        }
    }
}
