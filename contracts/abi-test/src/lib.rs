use borderless_sdk::{error, info, Context, Result};

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
};

fn exec_run() -> Result<()> {
    let input = read_register(REGISTER_INPUT_ACTION).context("missing input register")?;
    info!("read {} bytes", input.len());

    let action = CallAction::from_bytes(&input)?;

    let s = action.pretty_print()?;
    info!("{s}");

    Ok(())
}
