#[borderless::agent]
pub mod transport_rfq_autobidder {
    use borderless::{
        collections::LazyVec, contracts::env::sink, info, prelude::*, time::timestamp,
    };
    use commerce_types::{core::*, transport::*};
    // TODO: Expose this API for agents
    use borderless::__private::dev::rand;

    #[derive(State)]
    pub struct TransportRfqAutobidder {
        /// Bid counter
        bids_to_place: LazyVec<TransportationRqState>,
    }

    impl TransportRfqAutobidder {
        #[action]
        fn incoming_rq(
            &mut self,
            request_id: String,
            status: TransportStatus,
            mode: TransportMode,
            service_level: Option<String>,
            equipment: Option<Equipment>,
            consignor: Address,
            consignee: Address,
            stops: Vec<LegStop>,
            cargo: CargoSummary,
            pickup: StepTiming,
            delivery: StepTiming,
            accessorials: Vec<Accessorial>,
            references: std::collections::HashMap<String, String>,
            rate: Option<UnitRate>,
            events: Vec<TransportEvent>,
        ) {
            let state = TransportationRqState {
                request_id,
                status,
                mode,
                service_level,
                equipment,
                consignor,
                consignee,
                stops,
                cargo,
                pickup,
                delivery,
                accessorials,
                references,
                rate,
                events,
            };
            info!("Received state: {state:#?}");
            self.bids_to_place.push(state);
        }

        #[schedule(interval = "30s", delay = "5s")]
        pub fn autobid(&mut self) -> Result<Vec<ContractCall>> {
            let mut calls = Vec::new();
            while let Some(state) = self.bids_to_place.pop() {
                info!("-- Hello, I would like to take a bid");
                let planned_pickup = generate_tw(rand(20, 28) as i64, rand(30, 90) as i64)?;
                let planned_delivery = generate_tw(rand(44, 52) as i64, rand(30, 90) as i64)?;
                let rate = generate_rate(&state.cargo);

                // Generate message for the RFQ sink
                let call = sink("rfq")?
                    .call_method("bid")
                    .with_value(json!({
                        "request_id": state.request_id,
                        "rate": rate,
                        "planned_pickup": planned_pickup,
                        "planned_delivery": planned_delivery,
                    }))
                    .build()?;
                calls.push(call);
            }
            Ok(calls)
        }
    }

    /// Generates a time-window with `hours_offset` from now and `minutes_spread` apart
    fn generate_tw(hours_offset: i64, minutes_spread: i64) -> Result<TimeWindow> {
        let now = timestamp();
        let earliest_utc = DateTime::from_timestamp_millis(now + hours_offset * 3600 * 1000)
            .context("failed to generate timestamp")?;
        let latest_utc = DateTime::from_timestamp_millis(
            now + hours_offset * 3600 * 1000 + minutes_spread * 3600,
        )
        .context("failed to generate timestamp")?;
        Ok(TimeWindow {
            earliest_utc,
            latest_utc,
        })
    }

    /// Generates a semi-random rate based on the cargo-summary
    fn generate_rate(cargo: &CargoSummary) -> UnitRate {
        let base = rand(12, 18) as f32;
        let eur = base * cargo.gross_weight_kg / 100.0; // 12-18 ct / kg
        let cents = rand(0, 99) as u32;
        let rate = Money::euro(eur.ceil() as i64, cents);
        UnitRate {
            rate,
            unit_of_measure: "FLAT".to_string(),
            price_basis_quantity: None,
            term_reference: None,
        }
    }
}
