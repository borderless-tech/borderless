use borderless_sdk::{error, info, new_error, Context, Error, Result};

#[no_mangle]
pub extern "C" fn run() {
    dev::tic();
    let result = exec_run();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

use borderless_sdk_core::{
    contract::CallAction, dev, read_register, registers::REGISTER_INPUT_ACTION,
    serialize::from_value,
};

fn exec_run() -> Result<()> {
    let input = read_register(REGISTER_INPUT_ACTION).context("missing input register")?;
    info!("read {} bytes", input.len());

    let action = CallAction::from_bytes(&input)?;

    let s = action.pretty_print()?;
    info!("{s}");

    let method = action.method.context("missing required method-name")?;
    match method.as_str() {
        "call_my_fn" => {
            let params: CallMyFnArgs = from_value(action.params)?;
            call_my_fn(params.foo, params.baa);
        }
        "other_fn" => {
            let params: OtherFnArgs = from_value(action.params)?;
            other_fn(params.text);
        }
        other => return Err(new_error!("unknown method: {other}")),
    }

    Ok(())
}

// NOTE: Let's dig into this, what the sdk macro should derive
#[derive(serde::Deserialize)]
struct CallMyFnArgs {
    foo: u32,
    baa: u32,
}
fn call_my_fn(foo: u32, baa: u32) {
    info!("foo = {foo}, baa = {baa}");
}

#[derive(serde::Deserialize)]
struct OtherFnArgs {
    text: String,
}
fn other_fn(text: String) {
    info!("text = {text}");
}
