#[borderless::contract]
pub mod order_oneshot {
    use borderless::collections::HashMap;
    use borderless::prelude::*;
    use commerce_types::OrderRequest;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct OrderState {
        pub created: OrderRequest,
        pub confirmed: Option<OrderRequest>,
        pub order_received: Option<u64>, // timestamp - use chrono here
        pub invoice_received: Option<u64>, // use invoice model here
        pub paid: Option<u64>,
    }

    #[derive(State)]
    pub struct Order {
        orders: HashMap<String, OrderState>,
    }

    impl Order {
        #[action(web_api = true, roles = "buyer")]
        pub fn create_order(&mut self, order: OrderState) -> Result<()> {
            ledger::transfer("buyer", "seller")
                .with_amount("85 €".parse()?)
                .with_tax("16,15 €".parse()?)
                .execute()?;
            Ok(())
        }

        #[action(web_api = true, roles = "seller")]
        pub fn confirm_order(&self, item_no: u64) -> Result<()> {
            ledger::settle_debt("buyer", "seller")
                .with_amount("85 €".parse()?)
                .with_tag(format!("settle item-{}", item_no))
                .execute()?;
            Ok(())
        }

        #[action(web_api = true, roles = "buyer")]
        pub fn confirm_receival(&mut self) -> Result<()> {
            Ok(())
        }

        #[action(web_api = true, roles = "buyer")]
        pub fn confirm_invoice(&mut self) -> Result<()> {
            Ok(())
        }

        #[action(web_api = true, roles = "seller")]
        pub fn confirm_payment(&mut self) -> Result<()> {
            Ok(())
        }
    }
}
