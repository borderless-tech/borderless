use std::fmt::Display;

use borderless_sdk::contract::Introduction;
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

use borderless_sdk::internal::{
    dev, read_field, read_register, registers::REGISTER_INPUT, storage_begin_acid_txn,
    storage_commit_acid_txn, write_field,
};
use borderless_sdk::{contract::CallAction, serialize::from_value};

use serde::{Deserialize, Serialize};
use xxhash_rust::const_xxh3::xxh3_64;
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

    // Read state ( TODO )
    let storage_key_switch = xxh3_64("FLIPPER::switch".as_bytes());
    let storage_key_counter = xxh3_64("FLIPPER::counter".as_bytes());
    let switch = read_field(storage_key_switch, 0).context("missing field switch")?;
    let counter = read_field(storage_key_counter, 0).context("missing field counter")?;
    let mut state = Flipper { switch, counter };

    match method {
        "flip_switch" => {
            state.flip_switch();
        }
        "set_switch" => {
            let params: SetSwitchArgs = from_value(action.params)?;
            state.set_switch(params.switch);
        }
        other => return Err(new_error!("unknown method: {other}")),
    }

    // Commit state
    storage_begin_acid_txn();
    write_field(storage_key_switch, 0, &state.switch);
    write_field(storage_key_counter, 0, &state.counter);
    storage_commit_acid_txn();
    info!("Commited flipper: {state}");

    Ok(())
}

fn exec_introduction() -> Result<()> {
    // Read action
    let input = read_register(REGISTER_INPUT).context("missing input register")?;
    info!("read {} bytes", input.len());

    let introduction = Introduction::from_bytes(&input)?;
    let s = introduction.pretty_print()?;
    info!("{s}");

    // Parse initial state
    let state: Flipper = from_value(introduction.initial_state)?;
    info!(
        "Introduce new flipper: switch={}, counter={}",
        state.switch, state.counter
    );

    // TODO: Implement 'real' storage handling here,
    // and reserve the keyspace for the contract
    //
    // - add introduction data
    // - prepare logging
    // - prepare action buffer
    // ...
    // + define additional data, that the contract requires and how it is stored / passed into it
    let storage_key_switch = xxh3_64("FLIPPER::switch".as_bytes());
    let storage_key_counter = xxh3_64("FLIPPER::counter".as_bytes());

    storage_begin_acid_txn();
    write_field(storage_key_switch, 0, &state.switch);
    write_field(storage_key_counter, 0, &state.counter);
    storage_commit_acid_txn();

    Ok(())
}

// NOTE: Let's dig into this, what the sdk macro should derive
//

// This is our state
#[derive(Debug, Serialize, Deserialize)]
pub struct Flipper {
    switch: bool,
    counter: u32,
}

impl Display for Flipper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "switch={}, counter={}", self.switch, self.counter)
    }
}

impl Flipper {
    fn flip_switch(&mut self) {
        self.switch = !self.switch;
        self.counter += 1;
    }

    fn set_switch(&mut self, switch: bool) {
        self.counter += 1;
        self.switch = switch;
    }
}

#[derive(serde::Deserialize)]
struct SetSwitchArgs {
    switch: bool,
}
