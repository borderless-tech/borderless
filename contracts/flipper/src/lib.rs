#[borderless::contract]
pub mod flipper {
    use borderless::*;

    // This is our state
    #[derive(Debug, State)]
    pub struct Flipper {
        switch: bool,
        counter: u32,
    }

    pub enum Roles {
        Flipper,
        Observer,
    }

    impl Flipper {
        #[action]
        pub fn flip_switch(&mut self) {
            info!("hello");
            self.switch = !self.switch;
            self.counter += 1;
        }

        #[action(web-api = true)]
        pub fn set_switch(&mut self, switch: bool) {
            self.counter += 1;
            self.switch = switch;
        }
    }
}
