#[borderless::agent(websocket = true)]
pub mod hello {
    use borderless::{
        agents::{WebsocketHandler, WsConfig},
        *,
    };
    use events::ActionOutput;

    #[derive(State)]
    pub struct Hello {
        ws_url: String,
        cnt_schedule: usize,
        cnt_ws: usize,
    }

    impl Hello {
        #[action]
        pub fn say_hello(&mut self) {
            info!("Hello!");
        }

        #[schedule(delay = 10s, repeat = 5s)]
        pub fn send_hello(&mut self) -> Result<()> {
            self.cnt_schedule += 1;
            let msg = format!("Hello - this is message no. {}", self.cnt_schedule).into_bytes();
            self.send_ws_msg(msg)?;
            Ok(())
        }
    }

    impl WebsocketHandler for Hello {
        type Err = String;

        fn open_ws(&self) -> WsConfig {
            WsConfig {
                url: self.ws_url.clone(),
                reconnect: true,
                ping_interval: 30,
                binary: false,
            }
        }

        fn on_message(&mut self, msg: Vec<u8>) -> Result<Option<ActionOutput>, Self::Err> {
            let msg = String::from_utf8(msg).map_err(|e| e.to_string())?;
            self.cnt_ws += 1;
            info!("- received ws-msg no {}: {msg}", self.cnt_ws);
            Ok(None)
        }
    }
}
