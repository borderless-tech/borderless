mod basics;
mod product;

use borderless::{error, info, new_error, warn, Context, Result};

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

use borderless::__private::storage_traits::Storeable;
use borderless::__private::{dev, read_register, registers::*, storage_keys::make_user_key};
use borderless::collections::lazyvec::LazyVec;
use borderless::contract::CallAction;

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

    match method {
        "test_product" => {
            test_product()?;
        }
        "test_integrity" => {
            test_integrity()?;
        }
        other => return Err(new_error!("Unknown method: {other}")),
    }
    Ok(())
}

use crate::basics::{test_integrity, TEST_INTEGRITY_BASE_KEY};
use crate::product::{test_product, TEST_PRODUCT_BASE_KEY};
use borderless::contract::Introduction;

fn exec_introduction() -> Result<()> {
    // Read action
    let input = read_register(REGISTER_INPUT).context("missing input register")?;
    info!("read {} bytes", input.len());

    let introduction = Introduction::from_bytes(&input)?;
    let s = introduction.pretty_print()?;
    info!("{s}");

    let storage_key = make_user_key(TEST_PRODUCT_BASE_KEY);
    let mut lazy_vec: LazyVec<product::Product> = LazyVec::open(storage_key);
    if lazy_vec.exists() {
        warn!("LazyVec with given storage key already exists in DB. Wipe it out...");
        lazy_vec.clear();
        lazy_vec.commit(storage_key);
    } else {
        info!("Create new LazyVec for the product test");
        let lazy_vec: LazyVec<product::Product> = LazyVec::new(storage_key);
        lazy_vec.commit(storage_key);
    }

    let storage_key = make_user_key(TEST_INTEGRITY_BASE_KEY);
    let mut lazy_vec: LazyVec<u64> = LazyVec::open(storage_key);
    if lazy_vec.exists() {
        warn!("LazyVec with given storage key already exists in DB. Wipe it out...");
        lazy_vec.clear();
        lazy_vec.commit(storage_key);
    } else {
        info!("Create new LazyVec for the integrity test");
        let lazy_vec: LazyVec<u64> = LazyVec::new(storage_key);
        lazy_vec.commit(storage_key);
    }
    Ok(())
}
