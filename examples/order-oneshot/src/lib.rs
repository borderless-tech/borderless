#[borderless::contract]
pub mod order_oneshot {
    use borderless::prelude::*;

    #[derive(State)]
    pub struct Order {
        cnt: u64,
    }
    /*
     * Order Requested
     * Order Processed
     * Order
     * */

    impl Order {
        #[action]
        pub fn buy(&mut self) -> Result<()> {
            ledger::transfer("buyer", "seller")
                .with_amount("85 €".parse()?)
                .with_tax("16,15 €".parse()?)
                .with_tag(format!("bought item-{}", self.cnt))
                .execute()?;
            self.cnt += 1;
            Ok(())
        }

        #[action]
        pub fn settle(&self, item_no: u64) -> Result<()> {
            ledger::settle_debt("buyer", "seller")
                .with_amount("85 €".parse()?)
                .with_tag(format!("settle item-{}", item_no))
                .execute()?;
            Ok(())
        }

        #[action]
        pub fn cancel_last(&mut self) -> Result<()> {
            ledger::cancellation("buyer", "seller")
                .with_amount("85 €".parse()?)
                .with_tag(format!("cancel item-{}", self.cnt.saturating_sub(1)))
                .execute()?;
            self.cnt -= 1;
            Ok(())
        }
    }
}
