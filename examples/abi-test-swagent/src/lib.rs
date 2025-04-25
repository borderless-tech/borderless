use borderless::__private::{registers::*, *};
use borderless::http::{as_json, as_text, get, post_json, send_request, Json, Method, Request};
use borderless::serialize::Value;
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

            // Test GET request
            info!("--- Test GET request: ");
            let request = Request::builder()
                .method(Method::GET)
                .uri("https://jsonplaceholder.typicode.com/users")
                .body(())?;

            let response = send_request(request)?;
            info!("status {}", response.status());

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
        other => return Err(new_error!("unknown method: {other}")),
    }

    Ok(())
}
