#[borderless::contract]
pub mod flipper {
    use borderless::*;
    use collections::lazyvec::LazyVec;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct History {
        switch: bool,
        counter: u32,
    }

    // This is our state
    #[derive(Debug, State)]
    pub struct Flipper {
        switch: bool,
        counter: u32,
        history: LazyVec<History>,
    }

    pub enum Roles {
        Flipper,
        Observer,
    }

    impl Flipper {
        #[action]
        pub fn flip_switch(&mut self) {
            info!("hello");
            self.set_switch(!self.switch);
        }

        #[action(web-api = true)]
        pub fn set_switch(&mut self, switch: bool) {
            self.history.push(History {
                switch: self.switch,
                counter: self.counter,
            });
            self.counter += 1;
            self.switch = switch;
        }
    }
}
