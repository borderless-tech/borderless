#[borderless::agent(websocket = true, schedule = true, network = true)]
pub mod hello {
    use borderless::*;

    #[derive(State)]
    pub struct Hello {
        ws_location: String,
        cnt: usize,
    }

    impl Hello {
        #[action]
        fn say_hello(&mut self) {
            info!("Hello!");
        }

        #[schedule(delay = 10s, repeat = 5s)]
        fn send_hello(&mut self) {
            /* Send hello via websocket */
        }
    }
}
