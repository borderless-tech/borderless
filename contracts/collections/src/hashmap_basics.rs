use borderless::__private::dev::rand;
use borderless::__private::storage_keys::make_user_key;
use borderless::__private::storage_traits::Storeable;
use borderless::collections::hashmap::HashMap;
use borderless::{ensure, info, warn, Context, Result};
use std::collections::HashMap as StdHashMap;

pub(crate) const TEST_INTEGRITY_BASE_KEY: u64 = 30000;
const N: u64 = 5000;

fn load_map() -> HashMap<u64> {
    let storage_key = make_user_key(TEST_INTEGRITY_BASE_KEY);
    HashMap::decode(storage_key)
}

pub(crate) fn hashmap_basics() -> Result<()> {
    let storage_key = make_user_key(TEST_INTEGRITY_BASE_KEY);
    let mut hashmap: HashMap<u64> = HashMap::decode(storage_key);

    if hashmap.exists() && !hashmap.is_empty() {
        warn!("LazyVec with given storage key already exists in DB. Wipe it out...");
        hashmap.clear();
    }

    info!("Executing the Hashmap integrity test suite...");
    is_empty()?;
    clear()?;
    keys()?;
    contains_key()?;
    insert()?;
    remove()?;

    info!("All integrity tests run successfully!");
    hashmap.commit(storage_key);
    Ok(())
}

fn is_empty() -> Result<()> {
    let map = load_map();
    ensure!(map.is_empty(), "Test [is_empty] failed");
    Ok(())
}

fn insert() -> Result<()> {
    let mut hashmap = load_map();
    // A trusted reference used to know what the correct behavior should be
    let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
        oracle.insert(i, random);
    }
    ensure!(
        hashmap.len() == oracle.len(),
        "Test [insert] failed with error 1"
    );
    // Check integrity
    for i in 0..N {
        let val = hashmap.get(i).context("Get({i}) must return some value")?;
        ensure!(
            oracle.get(&i) == Some(&val),
            "Test [insert] failed with error 2"
        )
    }
    Ok(())
}

fn remove() -> Result<()> {
    let mut hashmap = load_map();
    // A trusted reference used to know what the correct behavior should be
    let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
        oracle.insert(i, random);
    }
    ensure!(
        hashmap.len() == oracle.len(),
        "Test [remove] failed with error 1"
    );
    // Check integrity
    for i in 0..N {
        let x = hashmap.remove(i);
        let y = oracle.remove(&i);
        ensure!(x == y, "Test [remove] failed with error 3")
    }
    Ok(())
}

fn keys() -> Result<()> {
    let mut hashmap = load_map();

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
    }
    // Check integrity
    for i in 0..N {
        let k = hashmap.get(i).context("Get({i}) must return some value")?;
        ensure!(*k == i, "Test [keys] failed with error 2")
    }
    Ok(())
}

fn clear() -> Result<()> {
    let mut hashmap = load_map();
    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
    }
    hashmap.clear();
    // Check integrity
    ensure!(hashmap.is_empty(), "Test [clear] failed");
    Ok(())
}

fn contains_key() -> Result<()> {
    let mut hashmap = load_map();

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
    }
    // Check integrity
    let target: u64 = 30000;
    ensure!(
        !hashmap.contains_key(target),
        "Test [contains_key] failed with error 1"
    );
    hashmap.insert(target, 0);
    ensure!(
        hashmap.contains_key(target),
        "Test [contains_key] failed with error 2"
    );
    hashmap.remove(target);
    ensure!(
        !hashmap.contains_key(target),
        "Test [contains_key] failed with error 3"
    );
    Ok(())
}
