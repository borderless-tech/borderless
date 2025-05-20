use borderless::__private::{registers::*, *};
use borderless::agents::{Init, Schedule, WsConfig};
use borderless::events::MethodOrId;
use borderless::http::{as_json, as_text, get, post_json, send_request, Json, Method, Request};
use borderless::serialize::Value;
use borderless::time::SystemTime;
use borderless::{error, events::CallAction, info, new_error, Context, Result};
use serde::{Deserialize, Serialize};

#[no_mangle]
pub extern "C" fn process_action() {
    dev::tic();
    let result = exec_run();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

#[no_mangle]
pub extern "C" fn process_ws_msg() {
    dev::tic();
    let result = exec_ws();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

/// Function that is called by the host when the agent is initialized
#[no_mangle]
pub extern "C" fn on_init() {
    dev::tic();
    let result = exec_init();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("initialization successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("initialization failed: {e:?}"),
    }
}

fn exec_init() -> Result<()> {
    let mut my_init = Init {
        schedules: Vec::new(),
        ws_config: None,
    };
    my_init.schedules.push(Schedule {
        method: MethodOrId::ByName {
            method: "schedule-1".to_string(),
        },
        period: 5_000,
        delay: 2_000,
    });
    my_init.schedules.push(Schedule {
        method: MethodOrId::ByName {
            method: "schedule-2".to_string(),
        },
        period: 10_000,
        delay: 0,
    });
    my_init.ws_config = Some(WsConfig {
        url: "ws://localhost:5555".to_string(),
        no_msg_timeout: 30,
        reconnect: true,
        ping_interval: 60,
        binary: false,
    });

    let bytes = my_init.to_bytes()?;
    write_register(REGISTER_OUTPUT, &bytes);
    Ok(())
}

/// Function that is called by the host when the agent is shut down
#[no_mangle]
pub extern "C" fn on_shutdown() {
    todo!()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Post {
    title: String,
    body: String,
    user_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostRes {
    id: usize,
    title: String,
    body: String,
    user_id: usize,
}

fn exec_ws() -> Result<()> {
    let input = read_register(REGISTER_INPUT).context("missing input register")?;
    let msg = String::from_utf8(input)?;
    info!("received msg: {msg}");
    Ok(())
}

fn exec_run() -> Result<()> {
    // --- This will be the first snippet
    // Read action
    let input = read_register(REGISTER_INPUT).context("missing input register")?;
    info!("read {} bytes", input.len());

    let action = CallAction::from_bytes(&input)?;
    let s = action.pretty_print()?;
    info!("{s}");

    let method = action
        .method_name()
        .context("missing required method-name")?;

    // TODO:
    // - [x] Complete http api
    // - [ ] add websocket api
    // - [ ] add schedule api

    /*
     * Regarding the schedules; I think the easiest way would be to treat the schedules just like actions without parameters.
     * When a sw-agent spins up, it should return a list with "schedule objects", that basically contain the action-id that should be called and the information when it shoud be called.
     *
     * So there is no real "schedule-api"; but instead an "init-api", that defines what should happen when an agent is initialized
     *
     */

    match method {
        "asdf" => {
            info!(" hello my friend ");

            let now = SystemTime::now();

            // Test GET request
            info!("--- Test GET request: ");
            let request = Request::builder()
                .method(Method::GET)
                .uri("https://jsonplaceholder.typicode.com/users")
                .body(())?;

            let response = send_request(request)?;

            let elapsed = now.elapsed();
            info!("status {}, time-elapsed={elapsed:?}", response.status());

            let value: Value = as_json(response)?;
            info!("{}", value.to_string());

            // Test POST request
            info!("--- Test POST request: ");
            let post = Post {
                title: "random".to_string(),
                body: "something that someone would write".to_string(),
                user_id: 12345,
            };

            let request = Request::builder()
                .method(Method::POST)
                .uri("https://jsonplaceholder.typicode.com/posts")
                .body(Json(post.clone()))?;

            let response = send_request(request)?;
            info!("status {}", response.status());

            let value: Value = as_json(response)?;
            info!("{}", value.to_string());

            info!("--- Test PUT request: ");
            let request = Request::builder()
                .method(Method::PUT)
                .uri("https://jsonplaceholder.typicode.com/posts/1")
                .body(Json(PostRes {
                    id: 1,
                    title: "asdf".to_string(),
                    body: "asdf".to_string(),
                    user_id: 123455,
                }))?;

            // Use fancy monad here
            let updated: PostRes = send_request(request).and_then(as_json)?;
            info!("{updated:?}");

            // Use the simple wrapper functions
            let something =
                get("https://jsonplaceholder.typicode.com/posts/1").and_then(as_text)?;
            info!("{something}");

            let updated: PostRes =
                post_json(post, "https://jsonplaceholder.typicode.com/posts").and_then(as_json)?;
            info!("{updated:?}");
        }
        "schedule-1" => {
            info!("This is schedule-1!");
            send_ws_msg("This is my message from schedule-1")?;
        }
        "schedule-2" => {
            info!("This is schedule-2!");
        }
        other => return Err(new_error!("unknown method: {other}")),
    }

    Ok(())
}
