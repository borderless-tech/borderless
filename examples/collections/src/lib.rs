mod hashmap_basics;
mod lazyvec_basics;
mod lazyvec_product;

use borderless::{error, info, new_error, warn, Context, Result};
use serde_json::json;

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
use borderless::collections::hashmap::HashMap;
use borderless::collections::lazyvec::LazyVec;
use borderless::events::CallAction;

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
        "lazyvec_test" => {
            lazyvec_basics()?;
            lazyvec_product()?;
        }
        "hashmap_test" => {
            hashmap_basics()?;
        }
        other => return Err(new_error!("Unknown method: {other}")),
    }
    Ok(())
}

use crate::hashmap_basics::{hashmap_basics, HASHMAP_BASICS};
use crate::lazyvec_basics::{lazyvec_basics, LAZYVEC_INTEGRITY};
use crate::lazyvec_product::{lazyvec_product, LAZYVEC_PRODUCT};
use borderless::contracts::Introduction;

fn exec_introduction() -> Result<()> {
    // Read action
    let input = read_register(REGISTER_INPUT).context("missing input register")?;
    info!("read {} bytes", input.len());

    let introduction = Introduction::from_bytes(&input)?;
    let s = introduction.pretty_print()?;
    info!("{s}");

    // Init collections
    init_lazyvec()?;
    init_hashmap()?;
    Ok(())
}

fn init_lazyvec() -> Result<()> {
    // Init LazyVec related to integrity tests
    let storage_key = make_user_key(LAZYVEC_PRODUCT);
    let mut lazy_vec: LazyVec<lazyvec_product::Product> = LazyVec::decode(storage_key);
    if lazy_vec.exists() {
        warn!("LazyVec with given storage key already exists in DB. Wipe it out...");
        lazy_vec.clear();
    } else {
        info!("Create new LazyVec for the product test");
        lazy_vec = LazyVec::parse_value(json!([]), storage_key)?;
    }
    lazy_vec.commit(storage_key);

    // Init LazyVec related to product tests
    let storage_key = make_user_key(LAZYVEC_INTEGRITY);
    let mut lazy_vec: LazyVec<u64> = LazyVec::decode(storage_key);
    if lazy_vec.exists() {
        warn!("LazyVec with given storage key already exists in DB. Wipe it out...");
        lazy_vec.clear();
    } else {
        info!("Create new LazyVec for the integrity test");
        lazy_vec = LazyVec::parse_value(json!([]), storage_key)?;
    }
    lazy_vec.commit(storage_key);
    Ok(())
}

fn init_hashmap() -> Result<()> {
    let storage_key = make_user_key(HASHMAP_BASICS);
    let mut hashmap: HashMap<u64, u64> = HashMap::decode(storage_key);
    if hashmap.exists() {
        warn!("HashMap with given storage key already exists in DB. Wipe it out...");
        hashmap.clear();
    } else {
        info!("Create new HashMap for the integrity test");
        hashmap = HashMap::parse_value(json!([]), storage_key)?;
    }
    hashmap.commit(storage_key);
    Ok(())
}
