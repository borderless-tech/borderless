#[borderless::contract]
pub mod lazyvec_test {
    use crate::product::Product;
    use borderless::__private::dev::rand;
    use borderless::collections::lazyvec::LazyVec;
    use borderless::{Result, *};

    const N: usize = 5000;

    // This is our state
    #[derive(State)]
    pub struct LazyVecState {
        counter: u32,
        catalog: LazyVec<Product>,
        records: LazyVec<u64>,
    }

    impl LazyVecState {
        #[action]
        fn run_basics(&mut self) -> Result<()> {
            self.is_empty()?;
            self.clear()?;
            self.contains()?;
            self.push()?;
            self.pop()?;
            self.insert()?;
            self.remove()?;
            Ok(())
        }

        #[action]
        fn run_complex(&mut self) -> Result<()> {
            self.add_product()?;
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
    }
}
