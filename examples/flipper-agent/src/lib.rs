#[borderless::agent]
pub mod flipper {
    use borderless::agents::env as agent_env;
    use borderless::contracts::env as contract_env;
    use borderless::prelude::*;
    use borderless::{Result, *};
    use collections::lazyvec::LazyVec;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct History {
        switch: bool,
        counter: u32,
    }

    // This is our state
    #[derive(State)]
    pub struct Flipper {
        switch: bool,
        counter: u32,
        history: LazyVec<History>,
    }

    impl Flipper {
        #[action]
        fn flip_switch(&mut self) {
            self.set_switch(!self.switch);
        }

        #[action]
        fn set_switch(&mut self, switch: bool) {
            self.history.push(History {
                switch: self.switch,
                counter: self.counter,
            });
            self.counter += 1;
            self.switch = switch;
        }

        #[action]
        pub fn set_other(&self, switch: bool) -> Result<ContractCall> {
            let call = contract_env::sink("flipper")?
                .call_method("set_switch")
                .with_value(value!({ "switch": switch }))
                .build()?;
            Ok(call)
        }

        #[action]
        pub fn subscribe(&self, publisher: Id, topic: String, method: String) -> Result<()> {
            agent_env::subscribe(Topic {
                publisher,
                topic,
                method,
            })
        }
    }
}
