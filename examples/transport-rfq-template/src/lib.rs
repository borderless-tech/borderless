#[borderless::contract]
pub mod contract {
    use borderless::prelude::ledger::{settle_debt, transfer};
    use borderless::prelude::*;
    use borderless::{collections::HashMap, time::timestamp};
    use commerce_types::core::UnitRate;
    use commerce_types::transport::{TimeWindow, TransportationRfq, TransportationRqState};

    #[derive(State)]
    pub struct State {
        open_requests: HashMap<String, TransportationRqState>,
        closed_requests: HashMap<String, TransportationRqState>,
    }

    impl State {
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

        pub fn bid(
            &mut self,
            request_id: String,
            rate: UnitRate,
            planned_pickup: TimeWindow,
            planned_delivery: TimeWindow,
        ) -> Result<Message> {
            let state = self
                .open_requests
                .get_mut(&request_id)
                .context(format!("found no request with id={request_id}"))?;

            todo!()
        }
    }
}
