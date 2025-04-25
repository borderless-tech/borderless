use borderless::__private::{registers::*, *};
use borderless::http::{send_request, Method, Request};
use borderless::{error, events::CallAction, info, new_error, Context, Result};

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
    // - Complete http api
    // - add websocket api
    // - add schedule api

    let request = Request::builder()
        .method(Method::GET)
        .uri("https://jsonplaceholder.typicode.com/users")
        .body(())?;

    let response = send_request(request)?;
    info!("status {}", response.status());

    let body = response.body();
    let value: borderless::serialize::Value = borderless::serialize::from_slice(&body)?;
    info!("{}", value.to_string());

    match method {
        "asdf" => {
            info!(" hello my friend ");
        }
        other => return Err(new_error!("unknown method: {other}")),
    }

    Ok(())
}
