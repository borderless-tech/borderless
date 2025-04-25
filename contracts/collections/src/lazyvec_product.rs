use borderless::__private::dev::rand;
use borderless::__private::storage_keys::make_user_key;
use borderless::__private::storage_traits::Storeable;
use borderless::collections::lazyvec::LazyVec;
use borderless::{info, new_error, warn, Result};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub(crate) const TEST_PRODUCT_BASE_KEY: u64 = 10000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    name: String,
    price: u64,
    available: bool,
    related: Vec<Product>,
}

impl Product {
    fn new(name: impl AsRef<str>, price: u64, available: bool) -> Self {
        Product {
            name: name.as_ref().to_owned(),
            price,
            available,
            related: vec![],
        }
    }

    fn generate_product() -> Self {
        let names = ["Toy", "Headset", "Computer", "Mirror", "T-Shirt", "Bottle"];
        let idx = rand(0, 6) as usize;
        let price = rand(5, 1000);

        let mut product = Product::new(names[idx], price, true);
        // Add related product
        if price % 2 == 0 || price % 5 == 0 || price % 7 == 0 {
            product.add_related_product(Self::generate_product());
        }
        // Return the new instance
        product
    }

    fn add_related_product(&mut self, product: Product) {
        self.related.push(product);
    }
}

impl Display for Product {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ {}, {}, {}, related=[",
            self.name, self.price, self.available
        )?;
        for p in &self.related {
            write!(f, "{}, ", p)?
        }
        write!(f, "] ]")
    }
}

pub fn lazyvec_product() -> Result<()> {
    // Load LazyVec from DB
    let storage_key = make_user_key(TEST_PRODUCT_BASE_KEY);
    let mut lazy_vec = LazyVec::decode(storage_key);

    info!("Number of products BEFORE: {}", lazy_vec.len());
    if lazy_vec.len() > 100000 {
        warn!("Too many products! Clearing...");
        lazy_vec.clear();
        lazy_vec.commit(storage_key);
        return Ok(());
    }

    let n = 5000;
    let start = lazy_vec.len();
    let end = start + n;

    for i in start..end {
        let product = Product::generate_product();
        lazy_vec.push(product.clone());

        let from_vec = lazy_vec.get(i).unwrap();
        if *from_vec != product {
            return Err(new_error!("{} !== {}", *from_vec, product));
        }
    }
    info!("Number of products AFTER: {}", lazy_vec.len());
    lazy_vec.commit(storage_key);
    Ok(())
}
