#[borderless::contract]
pub mod hashmap_test {
    use borderless::__private::dev::rand;
    use borderless::action;
    use borderless::collections::hashmap::HashMap;
    use borderless::{Result, *};
    use std::collections::HashMap as StdHashMap;

    const N: u64 = 5000;

    // This is our state
    #[derive(State)]
    pub struct HashMapState {
        counter: u32,
        records: HashMap<u64, u64>,
    }

    impl HashMapState {
        #[action]
        fn run_basics(&mut self) -> Result<()> {
            self.is_empty()?;
            self.clear()?;
            self.len()?;
            self.contains_key()?;
            self.insert()?;
            self.remove()?;
            self.keys()?;
            Ok(())
        }

        fn is_empty(&self) -> Result<()> {
            ensure!(self.records.is_empty(), "Test [is_empty] failed");
            Ok(())
        }

        fn clear(&mut self) -> Result<()> {
            for i in 0..N {
                let random = rand(0, u64::MAX);
                self.records.insert(i, random);
            }
            self.records.clear();
            // Check integrity
            ensure!(self.records.is_empty(), "Test [clear] failed");
            Ok(())
        }

        fn len(&mut self) -> Result<()> {
            for i in 0..N {
                // Check integrity
                ensure!(self.records.len() == i as usize, "Error 1 in [len]");
                let random = rand(0, u64::MAX);
                self.records.insert(i, random);
            }
            Ok(())
        }

        fn contains_key(&mut self) -> Result<()> {
            for i in 0..N {
                let random = rand(0, u64::MAX);
                self.records.insert(i, random);
            }
            // Check integrity
            let target: u64 = 30000;
            ensure!(
                !self.records.contains_key(target),
                "Error 1 in [contains_key]"
            );
            self.records.insert(target, 0);
            ensure!(
                self.records.contains_key(target),
                "Error 2 in [contains_key]"
            );
            self.records.remove(target);
            ensure!(
                !self.records.contains_key(target),
                "Error 3 in [contains_key]"
            );
            Ok(())
        }

        fn insert(&mut self) -> Result<()> {
            // A trusted reference used to know what the correct behavior should be
            let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

            for i in 0..N {
                let random = rand(0, u64::MAX);
                self.records.insert(i, random);
                oracle.insert(i, random);
            }
            // Check integrity
            for i in 0..N {
                let val = self
                    .records
                    .get(i)
                    .context("Get({i}) must return some value")?;
                ensure!(oracle.get(&i) == Some(&val), "Error 1 in [insert]")
            }
            Ok(())
        }

        fn remove(&mut self) -> Result<()> {
            // A trusted reference used to know what the correct behavior should be
            let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

            for i in 0..N {
                let random = rand(0, u64::MAX);
                self.records.insert(i, random);
                oracle.insert(i, random);
            }
            // Check integrity
            for i in 0..N {
                let x = self.records.remove(i);
                let y = oracle.remove(&i);
                ensure!(x == y, "Error 1 in [remove]")
            }
            Ok(())
        }

        fn keys(&mut self) -> Result<()> {
            // A trusted reference used to know what the correct behavior should be
            let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

            for i in 0..N {
                let random = rand(0, u64::MAX);
                self.records.insert(i, random);
                oracle.insert(i, random);
            }
            // Collect and sort both key-lists
            let mut hashmap_keys: Vec<u64> = self.records.keys().map(|p| *p).collect();
            let mut oracle_keys: Vec<u64> = oracle.keys().cloned().collect();
            hashmap_keys.sort_unstable();
            oracle_keys.sort_unstable();
            // Check integrity
            assert_eq!(hashmap_keys, oracle_keys);
            Ok(())
        }
    }
}
