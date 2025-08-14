#[borderless::contract]
pub mod contract {
    use borderless::collections::LazyVec;
    use borderless::prelude::ledger::{settle_debt, transfer};
    use borderless::prelude::*;
    use borderless::{collections::HashMap, time::timestamp};
    use commerce_types::order::{Item, OrderRequest};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct OrderState {
        pub init: OrderRequest,
        pub confirmed: Option<OrderRequest>,
        pub order_received: Option<i64>, // timestamp - use chrono here
        pub invoice_received: Option<i64>, // use invoice model here
        pub paid: Option<i64>,
    }

    impl OrderState {
        /// Creates a new `OrderState` from an existing `OrderRequest`
        pub fn new(order: OrderRequest) -> Self {
            OrderState {
                init: order,
                confirmed: None,
                order_received: None,
                invoice_received: None,
                paid: None,
            }
        }

        /// Returns `true` if the order if fulfilled
        pub fn is_fulfilled(&self) -> bool {
            self.confirmed.is_some()
                && self.order_received.is_some()
                && self.invoice_received.is_some()
                && self.paid.is_some()
        }
    }

    #[derive(State)]
    pub struct Order {
        open_orders: HashMap<String, OrderState>,
        closed_orders: HashMap<String, OrderState>,
        // NOTE: This is here to check, which items can be bought
        item_catalogue: LazyVec<Item>,
    }

    impl Order {
        #[action(web_api = true, roles = "buyer")]
        pub fn create_order(&mut self, order: OrderRequest) -> Result<Message> {
            let order_id = order.header.order_id.clone();
            // Return error if order-id does already exist
            if self.open_orders.contains_key(&order_id)
                || self.closed_orders.contains_key(&order_id)
            {
                return Err(Error::msg("cannot create order - duplicate order-id"));
            }

            for line in order.items.iter() {
                let supplier_id = &line.item.item_id.supplier_part_id;
                if !self
                    .item_catalogue
                    .iter()
                    .any(|item| item.item_id.supplier_part_id == *supplier_id)
                {
                    return Err(Error::msg("requested item does not exist"));
                }
            }

            // Prepare message
            let msg = message("/order/create").with_content(&order)?;

            // Create state
            let state = OrderState::new(order);
            self.open_orders.insert(order_id, state);

            Ok(msg)
        }

        #[action(web_api = true, roles = "buyer")]
        pub fn update_order(&mut self, order: OrderRequest) -> Result<Message> {
            let order_id = order.header.order_id.clone();
            // Return error if order-id does already exist
            if self.closed_orders.contains_key(&order_id) {
                return Err(Error::msg("cannot update closed order"));
            }

            let mut state = self
                .open_orders
                .get_mut(&order_id)
                .context(format!("found no order with id={order_id}"))?;

            // NOTE: This logic needs some expansion in the future
            if state.confirmed.is_some() {
                return Err(Error::msg("cannot update order that was already confirmed"));
            }

            // Prepare message
            let msg = message("/order/update").with_content(&order)?;

            // Update state
            state.init = order;

            Ok(msg)
        }

        #[action(web_api = true, roles = "seller")]
        pub fn confirm_order(&mut self, order: OrderRequest) -> Result<Message> {
            let order_id = order.header.order_id.clone();
            let mut state = self
                .open_orders
                .get_mut(&order_id)
                .context(format!("found no order with id={order_id}"))?;

            // Cannot confirm order twice
            if state.confirmed.is_some() {
                return Err(new_error!("order {order_id} is already confirmed"));
            }

            // Neither shipping nor billing address can be changed
            if state.init.header.ship_to != order.header.ship_to
                || state.init.header.bill_to != order.header.bill_to
            {
                return Err(new_error!(
                    "shipping and billing address cannot change from original request"
                ));
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
                .open_orders
                .get_mut(&order_id)
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
                .with_tag(&order_id)
                .execute()?;

            if state.is_fulfilled() {
                let closed = state.clone();
                drop(state);
                self.open_orders.remove(&order_id);
                self.closed_orders.insert(order_id, closed);
            }
            Ok(msg)
        }

        #[action(web_api = true, roles = "buyer")]
        pub fn confirm_invoice(&mut self, order_id: String) -> Result<Message> {
            let mut state = self
                .open_orders
                .get_mut(&order_id)
                .context(format!("found no order with id={order_id}"))?;

            // Cannot receive invoice before order is confirmed
            if state.confirmed.is_none() {
                return Err(new_error!("order {order_id} is not confirmed yet"));
            }

            if state.invoice_received.is_some() {
                return Err(new_error!(
                    "order {order_id} already has the invoice confirmed"
                ));
            }

            let timestamp = timestamp();
            state.invoice_received = Some(timestamp);

            // Prepare message
            let msg = message("/order/invoice").with_value(value!( {
                "order_id": order_id,
                "timestamp": timestamp,
            }));

            if state.is_fulfilled() {
                let closed = state.clone();
                drop(state);
                self.open_orders.remove(&order_id);
                self.closed_orders.insert(order_id, closed);
            }
            Ok(msg)
        }

        #[action(web_api = true, roles = "seller")]
        pub fn confirm_payment(&mut self, order_id: String) -> Result<Message> {
            let mut state = self
                .open_orders
                .get_mut(&order_id)
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
                .with_tag(&order_id)
                .execute()?;

            if state.is_fulfilled() {
                let closed = state.clone();
                drop(state);
                self.open_orders.remove(&order_id);
                self.closed_orders.insert(order_id, closed);
            }

            Ok(msg)
        }
    }
}
