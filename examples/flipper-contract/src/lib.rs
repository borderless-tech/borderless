#[borderless::contract]
pub mod flipper {
    use borderless::prelude::*;
    use borderless::{collections::lazyvec::LazyVec, ContractId};
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

    use self::actions::Actions;

    #[derive(NamedSink)]
    pub enum Sinks {
        Flipper(Actions),
    }

    impl Flipper {
        #[action]
        fn flip_switch(&mut self) {
            self.set_switch(!self.switch);
        }

        #[action(web_api = true, roles = "Flipper")]
        fn set_switch(&mut self, switch: bool) {
            self.history.push(History {
                switch: self.switch,
                counter: self.counter,
            });
            self.counter += 1;
            self.switch = switch;
        }

        #[action(web_api = true, roles = "Flipper")]
        pub fn set_other(&self, switch: bool) -> Result<ActionOutput> {
            let call = env::sink("otherflipper")?
                .call_method("set_switch")
                .with_value(value! { switch })
                .with_writer("flipper")
                .build()?;

            // Or emit messages:
            let msg = message("/foo/baa").with_value(value! { switch });

            todo!()
        }
    }
}
