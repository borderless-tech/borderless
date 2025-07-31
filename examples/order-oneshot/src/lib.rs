#[borderless::contract]
pub mod order_oneshot {
    use borderless::{prelude::*, serialize::Value};

    #[derive(State)]
    pub struct Order {
        items: Value,
        /// 'Due-Date'
        ts_due: u64,
    }
    /*
     * Order Requested
     * Order Processed
     * Order
     * */

    impl Order {}
}
