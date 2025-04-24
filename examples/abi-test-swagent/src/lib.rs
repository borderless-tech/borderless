use borderless::__private::{registers::*, *};
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

    let result = send_http_rq(
        HttpMethod::Get,
        "https://jsonplaceholder.typicode.com/users",
        &[],
    );
    let value: borderless::serialize::Value = borderless::serialize::from_slice(&result.unwrap())?;
    info!("{}", value.to_string());

    match method {
        "asdf" => {
            info!(" hello my friend ");
        }
        other => return Err(new_error!("unknown method: {other}")),
    }

    Ok(())
}
