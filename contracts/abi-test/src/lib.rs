use std::fmt::Display;

use borderless_sdk::__private::http::to_payload;
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
    let result = exec_revocation();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

#[no_mangle]
pub extern "C" fn http_get_state() {
    dev::tic();
    let result = exec_get_state();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

#[no_mangle]
pub extern "C" fn http_post_action() {
    dev::tic();
    let result = exec_post_action();
    let elapsed = dev::toc();
    match result {
        Ok(()) => info!("execution successful. Time elapsed: {elapsed:?}"),
        Err(e) => error!("execution failed: {e:?}"),
    }
}

use borderless_sdk::__private::{
    dev, read_field, read_register, read_string_from_register, registers::*,
    storage_keys::make_user_key, write_field, write_register, write_string_to_register,
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
    let storage_key_switch = make_user_key(xxh3_64("FLIPPER::switch".as_bytes()));
    let storage_key_counter = make_user_key(xxh3_64("FLIPPER::counter".as_bytes()));
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
        "print_env" => {
            // This does not act on the state
            test_env();
        }
        other => return Err(new_error!("unknown method: {other}")),
    }

    // Commit state
    write_field(storage_key_switch, 0, &state.switch);
    write_field(storage_key_counter, 0, &state.counter);
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
    let state: Flipper = from_value(introduction.initial_state.clone())?;
    info!(
        "Introduce new flipper: switch={}, counter={}",
        state.switch, state.counter
    );

    let storage_key_switch = make_user_key(xxh3_64("FLIPPER::switch".as_bytes()));
    let storage_key_counter = make_user_key(xxh3_64("FLIPPER::counter".as_bytes()));

    // Write state
    write_field(storage_key_switch, 0, &state.switch);
    write_field(storage_key_counter, 0, &state.counter);

    Ok(())
}

fn exec_revocation() -> Result<()> {
    info!("Revoked contract without any further actions.");
    Ok(())
}

// Test out, if the "environment variables" work as expected
fn test_env() {
    use borderless_sdk::contract::env;
    info!("Contract-ID: {}", env::contract_id());
    info!("Participants: {:#?}", env::participants());
    info!("Roles: {:#?}", env::roles());
    info!("Sinks: {:#?}", env::sinks());
    info!("Description: {:?}", env::desc());
    info!("Metadata: {:?}", env::meta());
    info!("Writer: {}", env::writer());
    info!("Block-ID: {}", env::block_id());
    info!("BlockCtx: {}", env::block_ctx());
    info!("Tx-ID: {}", env::tx_id());
    info!("TxCtx: {}", env::tx_ctx());
}

fn exec_get_state() -> Result<()> {
    let path = read_string_from_register(REGISTER_INPUT_HTTP_PATH).context("missing http-path")?;
    let (status, payload) = get_state_response(path)?;

    write_register(REGISTER_OUTPUT_HTTP_STATUS, status.to_be_bytes());
    write_string_to_register(REGISTER_OUTPUT_HTTP_RESULT, payload);

    Ok(())
}

fn get_state_response(path: String) -> Result<(u16, String)> {
    let path = path.strip_prefix('/').unwrap_or(&path);

    let storage_key_switch = make_user_key(xxh3_64("FLIPPER::switch".as_bytes()));
    let storage_key_counter = make_user_key(xxh3_64("FLIPPER::counter".as_bytes()));

    // Extract query string
    let (path, _query) = match path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (path, None),
    };

    // Quick-shot, check if the user wants to access the entire state
    if path.is_empty() {
        let switch = read_field(storage_key_switch, 0).context("missing field switch")?;
        let counter = read_field(storage_key_counter, 0).context("missing field counter")?;
        let state = Flipper { switch, counter };
        return Ok((200, to_payload(&state, "")?.unwrap()));
    }
    let (prefix, suffix) = match path.find('/') {
        Some(idx) => path.split_at(idx),
        None => (path, ""),
    };

    let payload = match prefix {
        "switch" => {
            let switch: bool = read_field(storage_key_switch, 0).context("missing field switch")?;
            to_payload(&switch, suffix)?
        }
        "counter" => {
            let counter: u32 =
                read_field(storage_key_counter, 0).context("missing field counter")?;
            to_payload(&counter, suffix)?
        }
        _ => None,
    };

    let status = if payload.is_some() { 200 } else { 404 };
    Ok((status, payload.unwrap_or_default()))
}

fn exec_post_action() -> Result<()> {
    let path = read_string_from_register(REGISTER_INPUT_HTTP_PATH).context("missing http-path")?;
    let payload = read_register(REGISTER_INPUT_HTTP_PAYLOAD).context("missing http-payload")?;
    match post_action_response(path, payload) {
        Ok(action) => {
            write_register(REGISTER_OUTPUT_HTTP_STATUS, 200u16.to_be_bytes());
            write_register(REGISTER_OUTPUT_HTTP_RESULT, action.to_bytes()?);
        }
        Err(e) => {
            write_register(REGISTER_OUTPUT_HTTP_STATUS, 400u16.to_be_bytes());
            write_string_to_register(REGISTER_OUTPUT_HTTP_RESULT, e.to_string());
        }
    };

    Ok(())
}

// TODO: Ok, this will be more complicated, if we handle all possible cases..
//
// /action               -> accept CallAction object
// /action/{method_name} -> accept params for function
// /action/{method_id}   -> accept params for function
//
// We can also implement a GET method for the actions, which will return a list of all possible actions and their schemas.
// TODO: Add own error type here, so we can convert to the correct status code
fn post_action_response(path: String, payload: Vec<u8>) -> Result<CallAction> {
    let path = path.replace("-", "_"); // Convert from kebab-case to snake_case
    let path = path.strip_prefix('/').unwrap_or(&path); // stip leading "/"

    let content = String::from_utf8(payload.clone()).unwrap_or_default();
    info!("{content}");

    // TODO: Check access of writer
    match path {
        "" => {
            let action = CallAction::from_bytes(&payload).context("failed to parse action")?;

            let method = action.method_name().unwrap();

            match method {
                "flip_switch" => {
                    let _args: FlipSwitchArgs = from_value(action.params.clone())?;
                }
                "set_switch" => {
                    let _args: SetSwitchArgs = from_value(action.params.clone())?;
                }
                "print_env" => {
                    // Empty args
                    let _args: FlipSwitchArgs = from_value(action.params.clone())?;
                }
                other => return Err(new_error!("unknown method: {other}")),
            }
            // At this point, the action is validated and can be returned
            Ok(action)
        }
        "flip_switch" => {
            let args: FlipSwitchArgs = borderless_sdk::serialize::from_slice(&payload)?;
            let value = borderless_sdk::serialize::to_value(&args)?;
            Ok(CallAction::by_method("flip_switch", value))
        }
        "set_switch" => {
            let args: SetSwitchArgs = borderless_sdk::serialize::from_slice(&payload)?;
            let value = borderless_sdk::serialize::to_value(&args)?;
            Ok(CallAction::by_method("set_switch", value))
        }
        "print_env" => {
            let args: FlipSwitchArgs = borderless_sdk::serialize::from_slice(&payload)?;
            let value = borderless_sdk::serialize::to_value(&args)?;
            Ok(CallAction::by_method("print_env", value))
        }
        other => Err(new_error!("unknown method: {other}")),
    }
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

#[derive(serde::Deserialize, serde::Serialize)]
struct SetSwitchArgs {
    switch: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct FlipSwitchArgs {}
