use crate::product::Product;
use borderless::__private::dev::rand;
use borderless::collections::hashmap::HashMap;
use borderless::ensure;
use borderless::{Result, *};
use std::collections::HashMap as StdHashMap;

const N: u64 = 5000;

pub(crate) fn is_empty(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    hashmap.clear();
    ensure!(hashmap.is_empty(), "Test [is_empty] failed");
    Ok(())
}

pub(crate) fn clear(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
    }
    hashmap.clear();
    // Check integrity
    ensure!(hashmap.is_empty(), "Test [clear] failed");
    Ok(())
}

pub(crate) fn len(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    hashmap.clear();
    for i in 0..N {
        // Check integrity
        ensure!(hashmap.len() == i as usize, "Error 1 in [len]");
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
    }
    Ok(())
}

pub(crate) fn contains_key(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    hashmap.clear();
    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
    }
    // Check integrity
    let target: u64 = 30000;
    ensure!(!hashmap.contains_key(target), "Error 1 in [contains_key]");
    hashmap.insert(target, 0);
    ensure!(hashmap.contains_key(target), "Error 2 in [contains_key]");
    hashmap.remove(target);
    ensure!(!hashmap.contains_key(target), "Error 3 in [contains_key]");
    Ok(())
}

pub(crate) fn insert(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    hashmap.clear();
    // A trusted reference used to know what the correct behavior should be
    let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
        oracle.insert(i, random);
    }
    // Check integrity
    for i in 0..N {
        let val = hashmap.get(i).context("Get({i}) must return some value")?;
        ensure!(oracle.get(&i) == Some(&val), "Error 1 in [insert]")
    }
    Ok(())
}

pub(crate) fn remove(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    hashmap.clear();
    // A trusted reference used to know what the correct behavior should be
    let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
        oracle.insert(i, random);
    }
    // Check integrity
    for i in 0..N {
        let x = hashmap.remove(i);
        let y = oracle.remove(&i);
        ensure!(x == y, "Error 1 in [remove]")
    }
    Ok(())
}

pub(crate) fn keys(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    hashmap.clear();
    // A trusted reference used to know what the correct behavior should be
    let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
        oracle.insert(i, random);
    }
    // Collect and sort both key-lists
    let mut hashmap_keys: Vec<u64> = hashmap.keys().map(|p| *p).collect();
    let mut oracle_keys: Vec<u64> = oracle.keys().cloned().collect();
    hashmap_keys.sort_unstable();
    oracle_keys.sort_unstable();
    // Check integrity
    assert_eq!(hashmap_keys, oracle_keys);
    Ok(())
}

pub(crate) fn values(hashmap: &mut HashMap<u64, u64>) -> Result<()> {
    hashmap.clear();
    // A trusted reference used to know what the correct behavior should be
    let mut oracle = Vec::with_capacity(N as usize);

    for i in 0..N {
        let random = rand(0, u64::MAX);
        hashmap.insert(i, random);
        oracle.push(random);
    }
    for v in hashmap.values() {
        assert!(oracle.contains(&*v));
    }
    Ok(())
}

pub(crate) fn add_product(hashmap: &mut HashMap<String, Product>) -> Result<()> {
    info!("Number of products BEFORE: {}", hashmap.len());
    if hashmap.len() > 100000 {
        warn!("Too many products! Clearing...");
        hashmap.clear();
        return Ok(());
    }

    let start = hashmap.len() as u64;
    let end = start + N;

    for i in start..end {
        let product = Product::generate_product();
        let key = format!("{}{}", product.name, i);
        hashmap.insert(key.clone(), product.clone());

        let from_map = hashmap.get(key).unwrap();
        if *from_map != product {
            return Err(new_error!("{} !== {}", *from_map, product));
        }
    }
    info!("Number of products AFTER: {}", hashmap.len());
    Ok(())
}
