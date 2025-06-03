#[borderless::agent]
pub mod flipper {
    use borderless::{Result, *};
    use collections::lazyvec::LazyVec;
    use events::ActionOutput;
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
        pub fn set_other(&self, switch: bool) -> Result<ActionOutput> {
            let mut out = ActionOutput::default();
            out.add_event(Sinks::Flipper(Actions::SetSwitch { switch }));
            Ok(out)
        }
    }
}
