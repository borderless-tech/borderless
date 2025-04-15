use borderless_sdk::{error, info, new_error, Context, Result};

#[no_mangle]
pub extern "C" fn process_transaction() {
    dev::tic();
    let result = exec_run();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

#[no_mangle]
pub extern "C" fn process_introduction() {
    dev::tic();
    let result = exec_introduction();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

#[no_mangle]
pub extern "C" fn process_revocation() {
    dev::tic();
    let result = exec_run();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

#[no_mangle]
pub extern "C" fn http_get_state() {}

#[no_mangle]
pub extern "C" fn http_post_action() {}

use borderless_sdk::__private::storage_traits::Storeable;
use borderless_sdk::__private::{dev, read_register, registers::*, storage_keys::make_user_key};
use borderless_sdk::collections::lazyvec::LazyVec;
use borderless_sdk::contract::CallAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Product {
    name: String,
    price: u64,
    available: bool,
}

fn exec_run() -> Result<()> {
    // Read action
    let input = read_register(REGISTER_INPUT).context("missing input register")?;
    info!("read {} bytes", input.len());

    let action = CallAction::from_bytes(&input)?;
    let s = action.pretty_print()?;
    info!("{s}");

    let method = action
        .method_name()
        .context("missing required method-name")?;

    let storage_key = make_user_key(1000);
    let lazy_vec: LazyVec<Product> = LazyVec::open(storage_key);

    match method {
        "add_product" => {}
        other => return Err(new_error!("Unknown method: {other}")),
    }

    // Commit state
    info!("New LazyVec length is: {}", lazy_vec.len());
    lazy_vec.commit(storage_key);
    Ok(())
}

use borderless_sdk::contract::Introduction;
fn exec_introduction() -> Result<()> {
    // Read action
    let input = read_register(REGISTER_INPUT).context("missing input register")?;
    info!("read {} bytes", input.len());

    let introduction = Introduction::from_bytes(&input)?;
    let s = introduction.pretty_print()?;
    info!("{s}");

    let storage_key = make_user_key(1000);
    if LazyVec::<Product>::open(storage_key).exists() {
        return Err(new_error!(
            "LazyVec with provided storage key already exists"
        ));
    }
    // Create and store new LazyVec
    let lazy_vec: LazyVec<Product> = LazyVec::new(storage_key);
    lazy_vec.commit(storage_key);
    Ok(())
}
