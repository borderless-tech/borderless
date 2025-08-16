#[borderless::contract]
pub mod contract {
    // use borderless::prelude::ledger::{settle_debt, transfer};
    use borderless::collections::HashMap;
    use borderless::prelude::*;
    use commerce_types::core::{DateTime, UnitRate, Utc};
    use commerce_types::transport::{
        TimeWindow, TransportStatus, TransportationRfq, TransportationRqState,
    };

    #[derive(State)]
    pub struct State {
        open_requests: HashMap<String, TransportationRqState>,
        closed_requests: HashMap<String, TransportationRqState>,
    }

    impl State {
        /// Create a new RFQ for transportation
        #[action(web_api = true, roles = "buyer")]
        pub fn request_for_quotation(&mut self, rq: TransportationRfq) -> Result<Message> {
            let request_id = rq.request_id.clone();
            if self.open_requests.contains_key(&request_id)
                || self.closed_requests.contains_key(&request_id)
            {
                return Err(new_error!("duplicate request-id"));
            }
            let state: TransportationRqState = rq.into();
            let msg = message("/transport/rfq").with_content(&state)?;
            self.open_requests.insert(request_id.clone(), state);
            Ok(msg)
        }

        /// Place a bid as carrier and set rate, planned-pickup and planned-delivery time-windows
        #[action(web_api = true, roles = "carrier")]
        pub fn bid(
            &mut self,
            request_id: String,
            rate: UnitRate,
            planned_pickup: TimeWindow,
            planned_delivery: TimeWindow,
        ) -> Result<Message> {
            let mut state = self
                .open_requests
                .get_mut(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(
                state.status,
                TransportStatus::RfqSent | TransportStatus::BidSent
            ) {
                return Err(new_error!(
                    "status of request={request_id} must be 'RFQ_SENT'"
                ));
            }

            // Prepare message
            let msg = message("/transport/bid").with_value(json! ({
                "request_id": request_id,
                "rate": rate,
                "planned_pickup": planned_pickup,
                "planned_delivery": planned_delivery,
            }));

            // Update state
            state.rate = Some(rate);
            state.status = TransportStatus::BidSent;
            state.pickup.set_plan(planned_pickup);
            state.delivery.set_plan(planned_delivery);

            Ok(msg)
        }

        /// Decline the transport - this is the "cancel" equivalent for the carrier
        #[action(web_api = true, roles = "carrier")]
        pub fn decline(&mut self, request_id: String) -> Result<Message> {
            let mut state = self
                .open_requests
                .remove(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(
                state.status,
                TransportStatus::BidSent | TransportStatus::RfqSent
            ) {
                return Err(new_error!(
                    "status of request={request_id} must be 'RFQ_SENT' or 'BID_SENT'"
                ));
            }

            state.status = TransportStatus::Cancelled;

            let msg = message("/transport/cancelled").with_value(json!({
                "request_id": request_id,
            }));
            // Put to closed requests
            self.closed_requests.insert(request_id, state);
            Ok(msg)
        }

        /// Award a carrier with the transport
        #[action(web_api = true, roles = "buyer")]
        pub fn award(&mut self, request_id: String) -> Result<Message> {
            let mut state = self
                .open_requests
                .get_mut(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(state.status, TransportStatus::BidSent) {
                return Err(new_error!(
                    "status of request={request_id} must be 'BID_SENT'"
                ));
            }

            state.status = TransportStatus::Awarded;

            let msg = message("/transport/award").with_value(json!({
                "request_id": request_id,
                "rate": state.rate,
                "pickup": state.pickup,
                "delivery": state.delivery,
                "consignor": state.consignor,
                "consignee": state.consignee,
            }));
            Ok(msg)
        }

        /// Cancel the request - this is sent by the buyer if another carrier has won the tender or if the transport isn't needed anyomore
        #[action(web_api = true, roles = "buyer")]
        pub fn cancel(&mut self, request_id: String) -> Result<Message> {
            let mut state = self
                .open_requests
                .remove(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(
                state.status,
                TransportStatus::BidSent | TransportStatus::RfqSent
            ) {
                return Err(new_error!(
                    "status of request={request_id} must be 'RFQ_SENT' or 'BID_SENT'"
                ));
            }

            state.status = TransportStatus::Cancelled;

            let msg = message("/transport/cancelled").with_value(json!({
                "request_id": request_id,
            }));
            // Put to closed requests
            self.closed_requests.insert(request_id, state);
            Ok(msg)
        }

        /// Confirm the booking and tighten the pickup- and delivery-windows
        #[action(web_api = true, roles = "carrier")]
        pub fn book(
            &mut self,
            request_id: String,
            planned_pickup: TimeWindow,
            planned_delivery: TimeWindow,
        ) -> Result<Message> {
            let mut state = self
                .open_requests
                .get_mut(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(state.status, TransportStatus::Awarded) {
                return Err(new_error!(
                    "status of request={request_id} must be 'AWARDED'"
                ));
            }

            state.status = TransportStatus::Booked;
            state.pickup.set_plan(planned_pickup);
            state.delivery.set_plan(planned_delivery);

            let msg = message("/transport/booked").with_value(json!({
                "request_id": request_id,
                "pickup": state.pickup,
                "delivery": state.delivery,
            }));
            Ok(msg)
        }

        /// Confirm the pickup and set the estimated time of arrival (ETA)
        #[action(web_api = true, roles = "carrier")]
        pub fn pickup(
            &mut self,
            request_id: String,
            actual_pickup: DateTime<Utc>,
            estimated_delivery: DateTime<Utc>,
        ) -> Result<Message> {
            let mut state = self
                .open_requests
                .get_mut(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(state.status, TransportStatus::Awarded) {
                return Err(new_error!(
                    "status of request={request_id} must be 'AWARDED'"
                ));
            }

            state.status = TransportStatus::InTransit;
            state.pickup.set_actual(actual_pickup);
            state.delivery.set_estimate(estimated_delivery);

            let msg = message("/transport/in-transit").with_value(json!({
                "request_id": request_id,
                "pickup": state.pickup,
                "delivery": state.delivery,
            }));
            Ok(msg)
        }

        /// Update the estimated time of arrival (ETA) for a transport in transit
        #[action(web_api = true, roles = "carrier")]
        pub fn update_eta(
            &mut self,
            request_id: String,
            estimated_delivery: DateTime<Utc>,
        ) -> Result<Message> {
            let mut state = self
                .open_requests
                .get_mut(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(state.status, TransportStatus::InTransit) {
                return Err(new_error!(
                    "status of request={request_id} must be 'IN_TRANSIT'"
                ));
            }

            state.delivery.set_estimate(estimated_delivery);

            let msg = message("/transport/in-transit").with_value(json!({
                "request_id": request_id,
                "pickup": state.pickup,
                "delivery": state.delivery,
            }));
            Ok(msg)
        }

        /// Complete the delivery
        #[action(web_api = true, roles = "carrier")]
        pub fn complete(
            &mut self,
            request_id: String,
            actual_delivery: DateTime<Utc>,
        ) -> Result<Message> {
            let mut state = self
                .open_requests
                .remove(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            if !matches!(state.status, TransportStatus::InTransit) {
                return Err(new_error!(
                    "status of request={request_id} must be 'IN_TRANSIT'"
                ));
            }

            state.status = TransportStatus::Completed;
            state.delivery.set_actual(actual_delivery);

            let msg = message("/transport/completed").with_value(json!({
                "request_id": request_id,
                "pickup": state.pickup,
                "delivery": state.delivery,
            }));
            Ok(msg)
        }
    }
}
