mod hashmap_test;
mod lazyvec_test;
mod product;

#[borderless::contract]
pub mod collections {
    use crate::product::Product;
    use borderless::collections::hashmap::HashMap;
    use borderless::collections::lazyvec::LazyVec;

    use borderless::{Result, *};

    use crate::hashmap_test as map;
    use crate::lazyvec_test as vec;

    // This is our state
    #[derive(State)]
    pub struct State {
        counter: u32,
        catalog: LazyVec<Product>,
        records: LazyVec<u64>,
        points: HashMap<u64, u64>,
        listing: HashMap<String, Product>,
    }

    impl State {
        #[action]
        fn run_basics(&mut self) -> Result<()> {
            // Run LazyVec basics
            info!("--------------------------------- ");
            info!("Running LazyVec basics action");
            vec::is_empty(&mut self.records)?;
            vec::clear(&mut self.records)?;
            vec::contains(&mut self.records)?;
            vec::push(&mut self.records)?;
            vec::pop(&mut self.records)?;
            vec::insert(&mut self.records)?;
            vec::remove(&mut self.records)?;

            // Run HashMap basics
            info!("--------------------------------- ");
            info!("Running HashMap basics action");
            map::is_empty(&self.points)?;
            map::clear(&mut self.points)?;
            map::len(&mut self.points)?;
            map::contains_key(&mut self.points)?;
            map::insert(&mut self.points)?;
            map::remove(&mut self.points)?;
            map::keys(&mut self.points)?;

            info!("--------------------------------- ");
            info!("All basic tests run successfully!");
            Ok(())
        }

        #[action]
        fn run_complex(&mut self) -> Result<()> {
            // Run LazyVec complex
            info!("--------------------------------- ");
            info!("Running LazyVec add_product action");
            vec::add_product(&mut self.catalog)?;
            info!("--------------------------------- ");
            info!("Running Hashmap add_product action");
            map::add_product(&mut self.listing)?;
            info!("--------------------------------- ");
            info!("All complex tests run successfully!");
            Ok(())
        }
    }
}
