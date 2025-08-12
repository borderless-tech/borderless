#[borderless::contract]
pub mod contract {
    use borderless::prelude::ledger::{settle_debt, transfer};
    use borderless::prelude::*;
    use borderless::{collections::HashMap, time::timestamp};
    use commerce_types::OrderRequest;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct OrderState {
        pub created: OrderRequest,
        pub confirmed: Option<OrderRequest>,
        pub order_received: Option<i64>, // timestamp - use chrono here
        pub invoice_received: Option<i64>, // use invoice model here
        pub paid: Option<i64>,
    }

    impl OrderState {
        pub fn new(order: OrderRequest) -> Self {
            OrderState {
                created: order,
                confirmed: None,
                order_received: None,
                invoice_received: None,
                paid: None,
            }
        }
    }

    #[derive(State)]
    pub struct Order {
        orders: HashMap<String, OrderState>,
    }

    impl Order {
        #[action(web_api = true, roles = "buyer")]
        pub fn create_order(&mut self, order: OrderRequest) -> Result<Message> {
            let order_id = order.header.order_id.clone();
            // Return error if order-id does already exist
            if self.orders.contains_key(&order_id) {
                return Err(Error::msg("cannot create order - duplicate order-id"));
            }
            // Prepare message
            let msg = message("/order/create").with_content(&order)?;

            // Create state
            let state = OrderState::new(order);
            self.orders.insert(order_id, state);

            Ok(msg)
        }

        #[action(web_api = true, roles = "seller")]
        pub fn confirm_order(&mut self, order: OrderRequest) -> Result<Message> {
            let order_id = order.header.order_id.clone();
            let mut state = self
                .orders
                .get_mut(order_id.clone())
                .context(format!("found no order with id={order_id}"))?;

            // Cannot confirm order twice
            if state.confirmed.is_some() {
                return Err(new_error!("order {order_id} is already confirmed"));
            }

            // Prepare message
            let msg = message("/order/confirm").with_content(&order)?;

            // Set state
            state.confirmed = Some(order);
            Ok(msg)
        }

        #[action(web_api = true, roles = "buyer")]
        pub fn confirm_receival(&mut self, order_id: String) -> Result<Message> {
            let mut state = self
                .orders
                .get_mut(order_id.clone())
                .context(format!("found no order with id={order_id}"))?;

            // Cannot receive order before it is confirmed
            if state.confirmed.is_none() {
                return Err(new_error!("order {order_id} is not confirmed yet"));
            }
            let timestamp = timestamp();
            state.order_received = Some(timestamp);

            // Prepare message
            let header = &state.confirmed.as_ref().unwrap().header;
            let msg = message("/order/receive").with_value(value!( {
                "order_id": order_id,
                "total": header.total,
                "tax": header.tax,
                "timestamp": timestamp,
            }));

            // Create a debt equal to the total cost of the order
            transfer("buyer", "seller")
                .with_amount(header.total)
                .with_tax_opt(header.tax)
                .with_tag(order_id)
                .execute()?;

            Ok(msg)
        }

        #[action(web_api = true, roles = "buyer")]
        pub fn confirm_invoice(&mut self, order_id: String) -> Result<Message> {
            let mut state = self
                .orders
                .get_mut(order_id.clone())
                .context(format!("found no order with id={order_id}"))?;

            // Cannot receive invoice before order is confirmed
            if state.confirmed.is_none() {
                return Err(new_error!("order {order_id} is not confirmed yet"));
            }
            let timestamp = timestamp();
            state.invoice_received = Some(timestamp);

            // Prepare message
            let msg = message("/order/invoice").with_value(value!( {
                "order_id": order_id,
                "timestamp": timestamp,
            }));
            Ok(msg)
        }

        #[action(web_api = true, roles = "seller")]
        pub fn confirm_payment(&mut self, order_id: String) -> Result<Message> {
            let mut state = self
                .orders
                .get_mut(order_id.clone())
                .context(format!("found no order with id={order_id}"))?;

            // Cannot pay twice
            if state.paid.is_some() {
                return Err(new_error!("order {order_id} is already paid for"));
            }
            let timestamp = timestamp();
            state.paid = Some(timestamp);

            // Prepare message
            let header = &state.confirmed.as_ref().unwrap().header;
            let msg = message("/order/payment").with_value(value!( {
                "order_id": order_id,
                "total": header.total,
                "tax": header.tax,
                "timestamp": timestamp,
            }));

            // Settle the created debt from this contract
            settle_debt("buyer", "seller")
                .with_amount(header.total)
                .with_tax_opt(header.tax)
                .with_tag(order_id)
                .execute()?;

            Ok(msg)
        }
    }
}
