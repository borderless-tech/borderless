use crate::lazyvec_product;
use borderless::__private::dev::rand;
use borderless::__private::storage_keys::make_user_key;
use borderless::__private::storage_traits::Storeable;
use borderless::collections::lazyvec::LazyVec;
use borderless::{ensure, info, warn, Context, Result};

pub(crate) const LAZYVEC_INTEGRITY: u64 = 20000;
const N: usize = 5000;

fn load_vec() -> LazyVec<u64> {
    let storage_key = make_user_key(LAZYVEC_INTEGRITY);
    LazyVec::decode(storage_key)
}

pub(crate) fn lazyvec_basics() -> Result<()> {
    let storage_key = make_user_key(LAZYVEC_INTEGRITY);
    let mut lazy_vec: LazyVec<lazyvec_product::Product> = LazyVec::decode(storage_key);

    if lazy_vec.exists() && !lazy_vec.is_empty() {
        warn!("LazyVec with given storage key already exists in DB. Wipe it out...");
        lazy_vec.clear();
    }

    info!("Executing the LazyVec integrity test suite...");
    is_empty()?;
    clear()?;
    contains()?;
    push()?;
    pop()?;
    insert()?;
    remove()?;

    info!("All integrity tests run successfully!");
    lazy_vec.commit(storage_key);
    Ok(())
}

fn is_empty() -> Result<()> {
    let vec = load_vec();
    ensure!(vec.is_empty(), "Test [is_empty] failed");
    Ok(())
}

fn clear() -> Result<()> {
    let mut vec = load_vec();
    for i in 0..N {
        vec.push(i as u64);
    }
    vec.clear();
    ensure!(vec.is_empty(), "Test [clear] failed");
    Ok(())
}

fn contains() -> Result<()> {
    let mut vec = load_vec();
    for _ in 0..N {
        vec.push(0);
    }
    let target: u64 = 30000;
    ensure!(!vec.contains(target), "Test [contains] failed with error 1");
    vec.insert(700, target);
    ensure!(vec.contains(target), "Test [contains] failed with error 2");
    vec.remove(700);
    ensure!(!vec.contains(target), "Test [contains] failed with error 3");
    Ok(())
}

fn push() -> Result<()> {
    let mut lazy_vec = load_vec();
    let mut vec = Vec::with_capacity(N);
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        vec.push(random);
    }
    ensure!(
        lazy_vec.len() == vec.len(),
        "Test [push] failed with error 1"
    );

    // Check integrity
    for i in 0..N {
        let val = lazy_vec.get(i).context("Get({i}) must return some value")?;
        ensure!(vec.get(i) == Some(&val), "Test [push] failed with error 2")
    }
    Ok(())
}

fn pop() -> Result<()> {
    let mut lazy_vec = load_vec();
    let mut vec = Vec::with_capacity(N);
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        vec.push(random);
    }
    ensure!(
        lazy_vec.len() == vec.len(),
        "Test [pop] failed with error 1"
    );

    // Check integrity
    for _ in 0..N {
        ensure!(
            lazy_vec.pop() == vec.pop(),
            "Test [pop] failed with error 2"
        )
    }
    ensure!(lazy_vec.is_empty(), "Test [pop] failed with error 3");
    ensure!(lazy_vec.pop().is_none(), "Test [pop] failed with error 4");

    Ok(())
}

fn insert() -> Result<()> {
    let mut lazy_vec = load_vec();
    let mut vec = Vec::with_capacity(N);
    // Insert some values so the data structures are not empty before the test
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        vec.push(random);
    }
    ensure!(
        lazy_vec.len() == vec.len(),
        "Test [insert] failed with error 1"
    );

    // Insert new elements to random positions
    for _i in 0..N {
        let pos = rand(0, lazy_vec.len() as u64) as usize;
        let random = rand(0, u64::MAX);
        lazy_vec.insert(pos, random);
        vec.insert(pos, random)
    }
    ensure!(
        lazy_vec.len() == vec.len(),
        "Test [insert] failed with error 2"
    );

    // Check integrity
    let end = lazy_vec.len();
    for i in 0..end {
        let val = lazy_vec.get(i).context("Get({i}) must return some value")?;
        ensure!(
            vec.get(i) == Some(&val),
            "Test [insert] failed with error 3"
        )
    }
    Ok(())
}

fn remove() -> Result<()> {
    let mut lazy_vec = load_vec();
    let mut vec = Vec::with_capacity(N);
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        vec.push(random);
    }
    ensure!(
        lazy_vec.len() == vec.len(),
        "Test [remove] failed with error 1"
    );

    for _ in 0..N {
        let pos: usize = rand(0, lazy_vec.len() as u64) as usize;
        ensure!(
            lazy_vec.remove(pos) == vec.remove(pos),
            "Test [remove] failed with error 2"
        );
    }
    ensure!(lazy_vec.is_empty(), "Test [remove] failed with error 3");

    Ok(())
}
