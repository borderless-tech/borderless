mod product;

#[borderless::contract]
pub mod collections {
    use crate::product::Product;
    use borderless::__private::dev::rand;
    use borderless::collections::hashmap::HashMap;
    use borderless::collections::lazyvec::LazyVec;
    use std::collections::HashMap as StdHashMap;

    use borderless::{Result, *};

    const N: usize = 5000;
    const M: u64 = 5000;

    // This is our state
    #[derive(State)]
    pub struct State {
        counter: u32,
        catalog: LazyVec<Product>,
        records: LazyVec<u64>,
        points: HashMap<u64, u64>,
    }

    impl State {
        #[action]
        fn run_basics(&mut self) -> Result<()> {
            self.basics_lazyvec()?;
            self.basics_hashmap()?;
            Ok(())
        }

        #[action]
        fn run_complex(&mut self) -> Result<()> {
            self.add_product()?;
            Ok(())
        }

        fn basics_lazyvec(&mut self) -> Result<()> {
            self.is_empty()?;
            self.clear()?;
            self.contains()?;
            self.push()?;
            self.pop()?;
            self.insert()?;
            self.remove()?;
            Ok(())
        }

        fn is_empty(&self) -> Result<()> {
            ensure!(self.records.is_empty(), "Test [is_empty] failed");
            Ok(())
        }

        fn clear(&mut self) -> Result<()> {
            for i in 0..N {
                self.records.push(i as u64);
            }
            self.records.clear();
            ensure!(self.records.is_empty(), "Test [clear] failed");
            Ok(())
        }

        fn contains(&mut self) -> Result<()> {
            for _ in 0..N {
                self.records.push(0);
            }
            let pos = 700;
            let target: u64 = 30000;
            ensure!(!self.records.contains(target), "Error 1 in [contains]");
            self.records.insert(pos, target);
            ensure!(self.records.contains(target), "Error 2 in [contains]");
            self.records.remove(pos);
            ensure!(!self.records.contains(target), "Error 3 in [contains]");
            Ok(())
        }

        fn push(&mut self) -> Result<()> {
            let mut oracle = Vec::with_capacity(N);
            for _ in 0..N {
                let random = rand(0, u64::MAX);
                self.records.push(random);
                oracle.push(random);
            }
            ensure!(self.records.len() == oracle.len(), "Error 1 in [push]");

            // Check integrity
            for i in 0..N {
                let val = self
                    .records
                    .get(i)
                    .context("Get({i}) must return some value")?;
                ensure!(oracle.get(i) == Some(&val), "Error 2 in [push]")
            }
            Ok(())
        }

        fn pop(&mut self) -> Result<()> {
            let mut oracle = Vec::with_capacity(N);
            for _ in 0..N {
                let random = rand(0, u64::MAX);
                self.records.push(random);
                oracle.push(random);
            }
            ensure!(self.records.len() == oracle.len(), "Error 1 in [pop]");

            // Check integrity
            for _ in 0..N {
                ensure!(self.records.pop() == oracle.pop(), "Error 2 in [pop]")
            }
            ensure!(self.records.is_empty(), "Error 3 in [pop]");
            ensure!(self.records.pop().is_none(), "Error 4 in [pop]");

            Ok(())
        }

        fn insert(&mut self) -> Result<()> {
            let mut oracle = Vec::with_capacity(N);
            // Insert some values so the data structures are not empty before the test
            for _ in 0..N {
                let random = rand(0, u64::MAX);
                self.records.push(random);
                oracle.push(random);
            }
            ensure!(self.records.len() == oracle.len(), "Error 1 in [insert]");

            // Insert new elements to random positions
            for _i in 0..N {
                let pos = rand(0, self.records.len() as u64) as usize;
                let random = rand(0, u64::MAX);
                self.records.insert(pos, random);
                oracle.insert(pos, random)
            }
            ensure!(self.records.len() == oracle.len(), "Error 2 in [insert]");

            // Check integrity
            let end = self.records.len();
            for i in 0..end {
                let val = self
                    .records
                    .get(i)
                    .context("Get({i}) must return some value")?;
                ensure!(oracle.get(i) == Some(&val), "Error 3 in [insert]")
            }
            Ok(())
        }

        fn remove(&mut self) -> Result<()> {
            let mut oracle = Vec::with_capacity(N);
            for _ in 0..N {
                let random = rand(0, u64::MAX);
                self.records.push(random);
                oracle.push(random);
            }
            ensure!(self.records.len() == oracle.len(), "Error 1 in [remove]");

            for _ in 0..N {
                let pos: usize = rand(0, self.records.len() as u64) as usize;
                ensure!(
                    self.records.remove(pos) == oracle.remove(pos),
                    "Error 2 in [remove]"
                );
            }
            ensure!(self.records.is_empty(), "Error 3 in [remove]");
            Ok(())
        }

        pub fn add_product(&mut self) -> Result<()> {
            info!("Number of products BEFORE: {}", self.catalog.len());
            if self.catalog.len() > 100000 {
                warn!("Too many products! Clearing...");
                self.catalog.clear();
                return Ok(());
            }

            let start = self.catalog.len();
            let end = start + N;

            for i in start..end {
                let product = Product::generate_product();
                self.catalog.push(product.clone());

                let from_vec = self.catalog.get(i).unwrap();
                if *from_vec != product {
                    return Err(new_error!("{} !== {}", *from_vec, product));
                }
            }
            info!("Number of products AFTER: {}", self.catalog.len());
            Ok(())
        }

        fn basics_hashmap(&mut self) -> Result<()> {
            self.is_empty_map()?;
            self.clear_map()?;
            self.len_map()?;
            self.contains_key_map()?;
            self.insert_map()?;
            self.remove_map()?;
            self.keys_map()?;
            Ok(())
        }

        fn is_empty_map(&self) -> Result<()> {
            ensure!(self.points.is_empty(), "Test [is_empty] failed");
            Ok(())
        }

        fn clear_map(&mut self) -> Result<()> {
            for i in 0..M {
                let random = rand(0, u64::MAX);
                self.points.insert(i, random);
            }
            self.points.clear();
            // Check integrity
            ensure!(self.points.is_empty(), "Test [clear] failed");
            Ok(())
        }

        fn len_map(&mut self) -> Result<()> {
            for i in 0..M {
                // Check integrity
                ensure!(self.points.len() == i as usize, "Error 1 in [len]");
                let random = rand(0, u64::MAX);
                self.points.insert(i, random);
            }
            Ok(())
        }

        fn contains_key_map(&mut self) -> Result<()> {
            for i in 0..M {
                let random = rand(0, u64::MAX);
                self.points.insert(i, random);
            }
            // Check integrity
            let target: u64 = 30000;
            ensure!(
                !self.points.contains_key(target),
                "Error 1 in [contains_key]"
            );
            self.points.insert(target, 0);
            ensure!(
                self.points.contains_key(target),
                "Error 2 in [contains_key]"
            );
            self.points.remove(target);
            ensure!(
                !self.points.contains_key(target),
                "Error 3 in [contains_key]"
            );
            Ok(())
        }

        fn insert_map(&mut self) -> Result<()> {
            // A trusted reference used to know what the correct behavior should be
            let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

            for i in 0..M {
                let random = rand(0, u64::MAX);
                self.points.insert(i, random);
                oracle.insert(i, random);
            }
            // Check integrity
            for i in 0..M {
                let val = self
                    .points
                    .get(i)
                    .context("Get({i}) must return some value")?;
                ensure!(oracle.get(&i) == Some(&val), "Error 1 in [insert]")
            }
            Ok(())
        }

        fn remove_map(&mut self) -> Result<()> {
            // A trusted reference used to know what the correct behavior should be
            let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

            for i in 0..M {
                let random = rand(0, u64::MAX);
                self.points.insert(i, random);
                oracle.insert(i, random);
            }
            // Check integrity
            for i in 0..M {
                let x = self.points.remove(i);
                let y = oracle.remove(&i);
                ensure!(x == y, "Error 1 in [remove]")
            }
            Ok(())
        }

        fn keys_map(&mut self) -> Result<()> {
            // A trusted reference used to know what the correct behavior should be
            let mut oracle = StdHashMap::<u64, u64>::with_capacity(N as usize);

            for i in 0..M {
                let random = rand(0, u64::MAX);
                self.points.insert(i, random);
                oracle.insert(i, random);
            }
            // Collect and sort both key-lists
            let mut hashmap_keys: Vec<u64> = self.points.keys().map(|p| *p).collect();
            let mut oracle_keys: Vec<u64> = oracle.keys().cloned().collect();
            hashmap_keys.sort_unstable();
            oracle_keys.sort_unstable();
            // Check integrity
            assert_eq!(hashmap_keys, oracle_keys);
            Ok(())
        }
    }
}
