use std::fmt::Display;

use borderless_sdk::{error, info, new_error, Context, Result};

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
    contract::CallAction, dev, read_field, read_register, registers::REGISTER_INPUT_ACTION,
    serialize::from_value, storage_begin_acid_txn, storage_commit_acid_txn, write_field,
};
use xxhash_rust::xxh32::xxh32;

// This is our state
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
}

fn exec_run() -> Result<()> {
    // Read action
    let input = read_register(REGISTER_INPUT_ACTION).context("missing input register")?;
    info!("read {} bytes", input.len());

    let action = CallAction::from_bytes(&input)?;
    let s = action.pretty_print()?;
    info!("{s}");

    let method = action.method.context("missing required method-name")?;

    let storage_key_switch = xxh32("FLIPPER::switch".as_bytes(), 0xff);
    let storage_key_counter = xxh32("FLIPPER::counter".as_bytes(), 0xff);

    // HACK: Just to test this out
    if method == "introduce" {
        let switch = action
            .params
            .get("switch")
            .context("missing switch")?
            .as_bool()
            .context("switch is not a bool")?;
        let counter = action
            .params
            .get("counter")
            .context("missing counter")?
            .as_u64()
            .context("counter is not a number")?;
        info!("Introduce new flipper: switch={switch}, counter={counter}");

        storage_begin_acid_txn();
        write_field(storage_key_switch, 0, &switch);
        write_field(storage_key_counter, 0, &counter);
        storage_commit_acid_txn();

        return Ok(());
    }

    // Read state
    let switch = read_field(storage_key_switch, 0).context("missing field switch")?;
    let counter = read_field(storage_key_counter, 0).context("missing field counter")?;
    let mut state = Flipper { switch, counter };

    match method.as_str() {
        "flip_switch" => {
            state.flip_switch();
        }
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

    // Commit state
    storage_begin_acid_txn();
    write_field(storage_key_switch, 0, &state.switch);
    write_field(storage_key_counter, 0, &state.counter);
    storage_commit_acid_txn();
    info!("Commited flipper: {state}");

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
