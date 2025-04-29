use borderless::__private::dev::rand;
use borderless::collections::lazyvec::LazyVec;
use borderless::ensure;
use borderless::{Result, *};

const N: usize = 5000;

pub(crate) fn is_empty(lazy_vec: &LazyVec<u64>) -> Result<()> {
    ensure!(lazy_vec.is_empty(), "Test [is_empty] failed");
    Ok(())
}

pub(crate) fn clear(lazy_vec: &mut LazyVec<u64>) -> Result<()> {
    for i in 0..N {
        lazy_vec.push(i as u64);
    }
    lazy_vec.clear();
    ensure!(lazy_vec.is_empty(), "Test [clear] failed");
    Ok(())
}

pub(crate) fn contains(lazy_vec: &mut LazyVec<u64>) -> Result<()> {
    for _ in 0..N {
        lazy_vec.push(0);
    }
    let pos = 700;
    let target: u64 = 30000;
    ensure!(!lazy_vec.contains(target), "Error 1 in [contains]");
    lazy_vec.insert(pos, target);
    ensure!(lazy_vec.contains(target), "Error 2 in [contains]");
    lazy_vec.remove(pos);
    ensure!(!lazy_vec.contains(target), "Error 3 in [contains]");
    Ok(())
}

pub(crate) fn push(lazy_vec: &mut LazyVec<u64>) -> Result<()> {
    let mut oracle = Vec::with_capacity(N);
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        oracle.push(random);
    }
    ensure!(lazy_vec.len() == oracle.len(), "Error 1 in [push]");

    // Check integrity
    for i in 0..N {
        let val = lazy_vec.get(i).context("Get({i}) must return some value")?;
        ensure!(oracle.get(i) == Some(&val), "Error 2 in [push]")
    }
    Ok(())
}

pub(crate) fn pop(lazy_vec: &mut LazyVec<u64>) -> Result<()> {
    let mut oracle = Vec::with_capacity(N);
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        oracle.push(random);
    }
    ensure!(lazy_vec.len() == oracle.len(), "Error 1 in [pop]");

    // Check integrity
    for _ in 0..N {
        ensure!(lazy_vec.pop() == oracle.pop(), "Error 2 in [pop]")
    }
    ensure!(lazy_vec.is_empty(), "Error 3 in [pop]");
    ensure!(lazy_vec.pop().is_none(), "Error 4 in [pop]");
    Ok(())
}

pub(crate) fn insert(lazy_vec: &mut LazyVec<u64>) -> Result<()> {
    let mut oracle = Vec::with_capacity(N);
    // Insert some values so the data structures are not empty before the test
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        oracle.push(random);
    }
    ensure!(lazy_vec.len() == oracle.len(), "Error 1 in [insert]");

    // Insert new elements to random positions
    for _i in 0..N {
        let pos = rand(0, lazy_vec.len() as u64) as usize;
        let random = rand(0, u64::MAX);
        lazy_vec.insert(pos, random);
        oracle.insert(pos, random)
    }
    ensure!(lazy_vec.len() == oracle.len(), "Error 2 in [insert]");

    // Check integrity
    let end = lazy_vec.len();
    for i in 0..end {
        let val = lazy_vec.get(i).context("Get({i}) must return some value")?;
        ensure!(oracle.get(i) == Some(&val), "Error 3 in [insert]")
    }
    Ok(())
}

pub(crate) fn remove(lazy_vec: &mut LazyVec<u64>) -> Result<()> {
    let mut oracle = Vec::with_capacity(N);
    for _ in 0..N {
        let random = rand(0, u64::MAX);
        lazy_vec.push(random);
        oracle.push(random);
    }
    ensure!(lazy_vec.len() == oracle.len(), "Error 1 in [remove]");

    for _ in 0..N {
        let pos: usize = rand(0, lazy_vec.len() as u64) as usize;
        ensure!(
            lazy_vec.remove(pos) == oracle.remove(pos),
            "Error 2 in [remove]"
        );
    }
    ensure!(lazy_vec.is_empty(), "Error 3 in [remove]");
    Ok(())
}
