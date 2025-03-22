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

use borderless_sdk_core::{dev, read_register, registers::REGISTER_INPUT_ACTION};

fn exec_run() -> Result<()> {
    let input = read_register(REGISTER_INPUT_ACTION).context("missing input register")?;

    info!("{input:?}");

    Ok(())
}
