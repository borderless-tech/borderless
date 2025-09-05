use crate::__private::REGISTER_CURSOR;
use crate::contracts::ledger::LedgerEntry;
use crate::prelude::{Id, Topic};
use borderless_abi::LogLevel;
use core::cell::RefCell;
use nohash_hasher::IntMap;
use rand::Rng;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// The off_chain environment
thread_local! {
    /// Simulates a Database
    pub static DATABASE: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
}

thread_local! {
    /// Simulates the WASM memory
    pub static REGISTERS: RefCell<IntMap<u64, Vec<u8>>> = RefCell::new(IntMap::default());
}

thread_local! {
    /// Simulates the Ledger
    pub static LEDGER: RefCell<IntMap<u64, Vec<LedgerEntry>>> = RefCell::new(IntMap::default());
}

thread_local! {
    /// Simulates a timer
    pub static TIMER: RefCell<Instant> = RefCell::new(Instant::now());
}

pub fn print(level: LogLevel, msg: impl AsRef<str>) {
    println!("[{:?}] {}", level, msg.as_ref())
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let mut db = db.borrow_mut();
        db.remove(&key);
    })
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let db = db.borrow();
        db.contains_key(&key)
    })
}

pub fn storage_gen_sub_key() -> u64 {
    rand(0, u64::MAX)
}

pub fn storage_cursor(base_key: u64) -> u64 {
    DATABASE.with(|db| {
        let mut db = db.borrow_mut();
        REGISTERS.with(|registers| {
            let mut registers = registers.borrow_mut();

            // Discard the HashMap's number of elements
            let sub_key = calc_storage_key(base_key, 0);
            db.remove(&sub_key);

            // Clear registers content
            registers.retain(|&k, _| k < REGISTER_CURSOR);

            // Dump database content into registers starting at position REGISTER_CURSOR
            for (i, (bytes, _)) in db.iter().enumerate() {
                // Extract the sub-key bytes
                let mut bytes: [u8; 8] = bytes[8..16].try_into().unwrap();
                // Reinterpret bytes as little-endian
                bytes.reverse();
                // Insert sub-key into registers
                registers.insert(REGISTER_CURSOR.saturating_add(i as u64), bytes.to_vec());
            }
        });

        // Return number of elements in DB
        db.len() as u64
    })
}

pub fn storage_read(base_key: u64, sub_key: u64) -> Option<Vec<u8>> {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let db = db.borrow();
        db.get(&key).cloned()
    })
}

pub fn storage_write(base_key: u64, sub_key: u64, value: impl AsRef<[u8]>) {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let mut db = db.borrow_mut();
        db.insert(key, value.as_ref().to_vec());
    })
}

pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    REGISTERS.with(|registers| {
        let registers = registers.borrow();
        registers.get(&register_id).cloned()
    })
}

pub fn write_register(register_id: u64, data: impl AsRef<[u8]>) {
    REGISTERS.with(|registers| {
        let mut registers = registers.borrow_mut();
        registers.insert(register_id, data.as_ref().to_vec())
    });
}

pub fn subscribe(topic: Topic) -> crate::Result<()> {
    DATABASE.with(|db| {
        // let mut db = db.borrow_mut();
        // TODO Generate a key
    });
    Ok(())
}

pub fn unsubscribe(publisher: Id, topic: String) -> crate::Result<()> {
    DATABASE.with(|db| {
        // let mut db = db.borrow_mut();
        // TODO Generate a key
    });
    Ok(())
}

pub fn create_ledger_entry(entry: LedgerEntry) -> crate::Result<()> {
    // NOTE: We don't do a participant check here for sake of simplicity
    LEDGER.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        let key = entry.creditor.merge_compact(&entry.debitor);
        ledger.entry(key).or_default().push(entry);
    });
    Ok(())
}

pub fn tic() {
    TIMER.with(|timer| {
        *timer.borrow_mut() = Instant::now();
    })
}

pub fn toc() -> Duration {
    TIMER.with(|timer| Instant::now().duration_since(*timer.borrow()))
}

pub fn rand(min: u64, max: u64) -> u64 {
    let mut range = rand::rng();
    range.random_range(min..max)
}

/// Calculates a storage key from base key, and sub key (ignoring the contract-id).
fn calc_storage_key(base_key: u64, sub_key: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&base_key.to_be_bytes());
    out.extend_from_slice(&sub_key.to_be_bytes());
    out
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::__private::registers::REGISTER_CURSOR;
    use crate::prelude::ledger::{Currency, EntryType, LedgerEntry};
    use borderless_id_types::BorderlessId;

    const BASE_KEY: u64 = 10;
    const SUB_KEY: u64 = 20;
    const REGISTER_ID: u64 = 30;

    #[test]
    fn rand_test() -> anyhow::Result<()> {
        let v = rand(0, 10);
        assert!((0..10).contains(&v), "The generated scalar is out of range");
        Ok(())
    }

    #[test]
    fn basic_crud_test() -> anyhow::Result<()> {
        let dummy = vec![1, 2, 3];
        // Create value
        storage_write(BASE_KEY, SUB_KEY, dummy.clone());
        // Check database contains key
        assert!(storage_has_key(BASE_KEY, SUB_KEY));
        // Read value
        let value = storage_read(BASE_KEY, SUB_KEY);
        assert_eq!(value, Some(dummy), "Values do not match");
        // Delete value
        storage_remove(BASE_KEY, SUB_KEY);
        // Check database does NOT contain key
        assert!(!storage_has_key(BASE_KEY, SUB_KEY));
        Ok(())
    }

    #[test]
    fn register_test() -> anyhow::Result<()> {
        let dummy = vec![1, 2, 3];
        // Register is empty
        assert_eq!(read_register(REGISTER_ID), None);
        // Write value to register
        write_register(REGISTER_ID, dummy.clone());
        // Read value from register
        let value = read_register(REGISTER_ID);
        assert_eq!(value, Some(dummy), "Values do not match");
        Ok(())
    }

    #[test]
    fn ledger_test() -> anyhow::Result<()> {
        let entry = LedgerEntry {
            creditor: BorderlessId::generate(),
            debitor: BorderlessId::generate(),
            amount_milli: 100_000,
            tax_milli: 10_000,
            currency: Currency::EUR,
            kind: EntryType::CREATE,
            tag: "some-tag".to_string(),
        };
        let key = entry.creditor.merge_compact(&entry.debitor);
        create_ledger_entry(entry.clone())?;
        // Check that entry is present
        LEDGER.with(|ledger| {
            let ledger = ledger.borrow();
            let value = ledger.get(&key);
            assert!(value.is_some(), "found no entry");
            let value = value.unwrap();
            assert!(value.len() == 1);
            assert_eq!(value[0].creditor, entry.creditor);
            assert_eq!(value[0].debitor, entry.debitor);
            assert_eq!(value[0].amount_milli, entry.amount_milli);
            assert_eq!(value[0].tax_milli, entry.tax_milli);
            assert_eq!(value[0].currency, entry.currency);
            assert_eq!(value[0].tag, entry.tag);
        });
        Ok(())
    }

    #[test]
    fn cursor_test() -> anyhow::Result<()> {
        let n = 10u64;
        let mut vec: Vec<u64> = Vec::with_capacity(n as usize);
        let mut oracle: Vec<u64> = Vec::with_capacity(n as usize);
        let dummy = vec![1, 2, 3];

        // Store n elements in storage
        for i in 0..n {
            let sub_key = SUB_KEY.saturating_add(i);
            vec.push(sub_key);
            storage_write(BASE_KEY, sub_key, dummy.clone());
        }
        // Check cursor size is n
        assert_eq!(storage_cursor(BASE_KEY), n, "Keys length mismatch");
        // Retrieve the keys from the reserved registers
        for i in 0..n {
            let bytes = read_register(REGISTER_CURSOR.saturating_add(i)).unwrap();
            let bytes: [u8; 8] = bytes.try_into().unwrap();
            let key = u64::from_le_bytes(bytes);
            oracle.push(key);
        }
        // Sort vectors as keys come unordered
        vec.sort_unstable();
        oracle.sort_unstable();
        assert_eq!(vec, oracle, "Keys do not match");
        Ok(())
    }
}
