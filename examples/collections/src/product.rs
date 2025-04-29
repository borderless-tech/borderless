use borderless::__private::dev::rand;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

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

    pub(crate) fn generate_product() -> Self {
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
