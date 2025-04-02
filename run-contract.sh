#!/usr/bin/env bash

DIR=$(dirname $0)
cd $DIR || exit 1
SCRIPT=$(basename $0)

# Check contract-name (first argument)
if [[ -z $1 ]]; then
  echo $1
  echo "Usage: $SCRIPT CONTRACT_NAME ACTION [optional] DB_PATH"
  exit 2
elif [[ $1 == "-h" || $1 == "--help" ]]; then
  echo "Usage: $SCRIPT CONTRACT_NAME ACTION [optional] DB_PATH"
  exit 0
fi

# Check action
if [[ -z $2 ]]; then
  echo "Usage: $SCRIPT CONTRACT_NAME ACTION [optional] DB_PATH"
  exit 3
else
  ACTION="$2"
fi

# Check db-path (second, optional argument)
if [[ -z $3 ]]; then
  DB_PATH="./db"
else
  DB_PATH="$2"
fi

# The package and the contract binary may have different names,
# as rust replaces "-" with "_" in binaries
PACKAGE=$1
CONTRACT=${1//-/_}

if [[ $WASMTIME_BACKTRAC_DETAILS == "1" ]]; then
  # Build and execute debug
  cargo build -p $PACKAGE --target=wasm32-unknown-unknown || exit 3 
  cargo run -p runner -- --db $DB_PATH contract $DIR/target/wasm32-unknown-unknown/debug/$CONTRACT.wasm process $ACTION
else
  # Build and execute release
  cargo build -p $PACKAGE --target=wasm32-unknown-unknown --release || exit 3 
  cargo run -p runner -- --db $DB_PATH contract $DIR/target/wasm32-unknown-unknown/release/$CONTRACT.wasm process $ACTION
fi
