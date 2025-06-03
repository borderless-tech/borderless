mod contract_actions;

#[borderless::agent]
pub mod cc_agent {
    use super::contract_actions::Actions as ContractActions;
    use borderless::*;
    use events::ActionOutput;
    use serde::{Deserialize, Serialize};

    // --- This is the code that the user writes
    #[derive(State, Serialize, Deserialize, PartialEq, Eq, Debug)]
    pub struct CC {
        pub last_number: u32,
    }

    #[derive(NamedSink)]
    pub enum Sinks {
        NextProcess(self::actions::Actions),
        NextContract(ContractActions),
    }

    impl CC {
        /// Increases the number and calls the next process
        #[action]
        pub fn increase_process(&mut self, number: u32) -> Result<ActionOutput, Error> {
            self.last_number = number;
            let mut out = ActionOutput::default();
            out.add_event(Sinks::NextProcess(
                self::actions::Actions::IncreaseContract { number: number + 1 },
            ));
            Ok(out)
        }

        /// Increases the number and calls the next contract
        #[action]
        pub fn increase_contract(&mut self, number: u32) -> Result<ActionOutput, Error> {
            self.last_number = number;
            let mut out = ActionOutput::default();
            out.add_event(Sinks::NextContract(ContractActions::SetNumber {
                number: number + 1,
            }));
            Ok(out)
        }
    }
}
