use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::core::*;

/* ====================== Data model ====================== */

/// Data model for an incoming transportation request
///
/// This is identical to [`TransportationRqState`], but has no events, the rate is not set (as this is requested),
/// and the pickup and delivery timing are only a request (as nothing is planned yet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TransportationRfq {
    /// Your unique ID (tender/booking)
    pub request_id: String,

    /// Mode and service flavor (e.g., FTL/LTL/Express)
    pub mode: TransportMode,
    pub service_level: Option<String>,

    /// Equipment needs (container/truck/trailer etc.)
    pub equipment: Option<Equipment>,

    /// "Ship-From" / origin
    pub consignor: Address,
    /// "Ship-To" / final destination
    pub consignee: Address,

    /// Optional intermediate stops / milk run
    pub stops: Vec<LegStop>,

    /// Shipment synopsis
    pub cargo: CargoSummary,

    /// High-level pickup/delivery lifecycle timings
    pub requested_pickup: TimeWindow,
    pub requested_delivery: TimeWindow,

    /// Extras beyond “drive from A to B” (liftgate, hazardous, temperature control, etc.)
    pub accessorials: Vec<Accessorial>,

    /// Business references (PO, Delivery, Booking, BOL, SSCC, etc.)
    pub references: HashMap<String, String>,
}

/// The state of a transport service request/tender/booking (A→B or multi-stop)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TransportationRqState {
    /// Your unique ID (tender/booking)
    pub request_id: String,

    /// Current lifecycle state
    pub status: TransportStatus,

    /// Mode and service flavor (e.g., FTL/LTL/Express)
    pub mode: TransportMode,
    pub service_level: Option<String>,

    /// Equipment needs (container/truck/trailer etc.)
    pub equipment: Option<Equipment>,

    /// "Ship-From" / origin
    pub consignor: Address,
    /// "Ship-To" / final destination
    pub consignee: Address,

    /// Optional intermediate stops / milk run
    pub stops: Vec<LegStop>,

    /// Shipment synopsis
    pub cargo: CargoSummary,

    /// High-level pickup/delivery lifecycle timings
    pub pickup: StepTiming,
    pub delivery: StepTiming,

    /// Extras beyond “drive from A to B” (liftgate, hazardous, temperature control, etc.)
    pub accessorials: Vec<Accessorial>,

    /// Business references (PO, Delivery, Booking, BOL, SSCC, etc.)
    pub references: HashMap<String, String>,

    /// Quoted/awarded rate (flat, per hour/km/container — via UnitRate)
    pub rate: Option<UnitRate>,

    /// Execution updates (optional stream of events)
    pub events: Vec<TransportEvent>,
}

impl From<TransportationRfq> for TransportationRqState {
    fn from(value: TransportationRfq) -> Self {
        TransportationRqState {
            request_id: value.request_id,
            status: TransportStatus::RfqSent,
            mode: value.mode,
            service_level: value.service_level,
            equipment: value.equipment,
            consignor: value.consignor,
            consignee: value.consignee,
            stops: value.stops,
            cargo: value.cargo,
            pickup: StepTiming::requested(value.requested_pickup),
            delivery: StepTiming::requested(value.requested_delivery),
            accessorials: value.accessorials,
            references: value.references,
            rate: None,
            events: Vec::new(),
        }
    }
}

/// Defines where you are in the request→execution flow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TransportStatus {
    /// Built, but not sent
    Draft,
    /// RFQ/Tender sent to providers
    RfqSent,
    /// Provider selected
    Awarded,
    /// transport order/booking confirmed
    Booked,
    /// pickup done and en route
    InTransit,
    /// delivered (actuals available)
    Completed,
    /// Request / Transport cancelled
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Copy)]
pub enum TransportMode {
    Road,
    Air,
    Ocean,
    Rail,
    Multimodal,
}

/// Requested vs planned vs estimated vs actual for one step (pickup or delivery)
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StepTiming {
    /// What the shipper/transport user asked for
    pub requested: Option<TimeWindow>,
    /// What the carrier committed to after award/booking
    pub planned: Option<TimeWindow>,
    /// Rolling ETA during execution (single best estimate)
    pub estimated: Option<DateTime<Utc>>,
    /// Final actual timestamp
    pub actual: Option<DateTime<Utc>>,
}

impl StepTiming {
    pub fn requested(tw: TimeWindow) -> Self {
        StepTiming {
            requested: Some(tw),
            planned: None,
            estimated: None,
            actual: None,
        }
    }

    pub fn set_plan(&mut self, tw: TimeWindow) {
        self.planned = Some(tw);
    }

    pub fn set_estimate(&mut self, date: DateTime<Utc>) {
        self.estimated = Some(date);
    }

    pub fn set_actual(&mut self, date: DateTime<Utc>) {
        self.actual = Some(date);
    }
}

/// A time window (site availability / appointment window)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TimeWindow {
    pub earliest_utc: DateTime<Utc>,
    pub latest_utc: DateTime<Utc>,
}

impl TimeWindow {
    pub fn new(
        earliest_utc: DateTime<Utc>,
        latest_utc: DateTime<Utc>,
    ) -> Result<Self, TransportError> {
        if earliest_utc > latest_utc {
            return Err(TransportError::BadWindow);
        }
        Ok(Self {
            earliest_utc,
            latest_utc,
        })
    }
}

/// An intermediate stop (for multi-pick/drop). Each stop can model both arrival & departure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LegStop {
    pub sequence: u32,
    pub location: Address,
    /// Arrival lifecycle timings at this stop
    pub arrival: Option<StepTiming>,
    /// Departure lifecycle timings at this stop
    pub departure: Option<StepTiming>,
    /// Optional notes / instructions specific to this stop
    pub notes: Option<String>,
}

/// What you’re moving (high-level)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CargoSummary {
    /// Number of pieces ( pieces being cartons / pallets / containers / etc. )
    pub pieces: u32,
    /// Weight in kilogram
    pub gross_weight_kg: f32,
    /// Volume in cubic meters
    pub volume_m3: Option<f32>,
    /// Weather or not the cargo contains dangerous goods (ADR/IMDG/etc.)
    pub dangerous_goods: bool,
    /// Optional short description
    pub commodity_description: Option<String>,
    /// Optional standardized codes (e.g., HS code), free-form map
    pub codes: Option<HashMap<String, String>>,
}

/// Equipment / container / trailer requirements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Equipment {
    /// Examples: "Box Truck 7.5t", "13.6m tautliner", "40HC", "20DV", "Reefer Trailer"
    pub type_code: String,
    /// Temperature range if controlled transport is needed (min, max)
    pub temperature_c: Option<(f32, f32)>,
    /// Free slots for axle/weight/class, door type, etc.
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Accessorial services (used to add cost / constraints on the delivery)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Accessorial {
    /// Liftgate
    TailLift,
    InsidePickup,
    InsideDelivery,
    /// Requires time-slot booking
    Appointment,
    /// Hazardous material handling
    Hazardous,
    /// Temperature managed transport
    TempControl,
    /// Brokerage / Clearance service
    Customs,
    /// Non-Commercial or limited access
    Residential,
    /// Billable waiting time
    Detention,
    /// Special handling / installation
    WhiteGlove,
    /// Any other option, represented as string
    Other(String),
}

/// Execution/visibility event (optional stream)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TransportEvent {
    pub code: TransportEventCode,
    pub timestamp: DateTime<Utc>,
    /// Who reported it (carrier/TSP, telematics, port, visibility provider…)
    pub source: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TransportEventCode {
    PickupPlanned,
    PickupEtaUpdated,
    PickupActual,
    DepartureActual,
    ArrivalEtaUpdated,
    ArrivalActual,
    DeliveryPlanned,
    DeliveryEtaUpdated,
    DeliveryActual,
    ProofOfDeliveryAvailable,
    Exception,
}

/* ====================== Errors ====================== */

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("invalid transition from {from:?} using {action}")]
    InvalidTransition {
        from: TransportStatus,
        action: &'static str,
    },
    #[error("cannot mutate a cancelled request")]
    IsCancelled,
    #[error("cannot mutate a completed request")]
    IsCompleted,
    #[error("time window has earliest > latest")]
    BadWindow,
    #[error("stop with sequence {0} not found")]
    StopNotFound(u32),
}

/* ====================== Transition API ====================== */

impl TransportationRqState {
    /* ---- Creation / RFQ ---- */

    /// Set requested pickup/delivery windows (shipper side).
    pub fn set_requested_windows(
        &mut self,
        pickup: Option<TimeWindow>,
        delivery: Option<TimeWindow>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("set_requested_windows")?;
        if let Some(w) = pickup {
            self.pickup.requested = Some(w);
        }
        if let Some(w) = delivery {
            self.delivery.requested = Some(w);
        }
        Ok(())
    }

    /// Mark the RFQ/tender as sent to carriers.
    pub fn mark_rfq_sent(
        &mut self,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("mark_rfq_sent")?;
        self.status = TransportStatus::RfqSent;
        self.push_event(
            TransportEventCode::PickupPlanned,
            ts,
            source,
            Some("RFQ/tender sent"),
        );
        Ok(())
    }

    /* ---- Award / booking ---- */

    /// Award to a carrier: set price and initial planned windows.
    pub fn award(
        &mut self,
        rate: UnitRate,
        pickup_planned: Option<TimeWindow>,
        delivery_planned: Option<TimeWindow>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("award")?;
        self.status = TransportStatus::Awarded;
        self.rate = Some(rate);
        if let Some(w) = pickup_planned {
            self.pickup.planned = Some(w);
        }
        if let Some(w) = delivery_planned {
            self.delivery.planned = Some(w);
        }
        self.push_event(
            TransportEventCode::PickupPlanned,
            ts,
            source,
            Some("Carrier awarded"),
        );
        self.push_event(TransportEventCode::DeliveryPlanned, ts, "system", None);
        Ok(())
    }

    /// Confirm booking/appointments; refine planned windows.
    pub fn confirm_booking(
        &mut self,
        pickup_planned: Option<TimeWindow>,
        delivery_planned: Option<TimeWindow>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("confirm_booking")?;
        self.status = TransportStatus::Booked;
        if let Some(w) = pickup_planned {
            self.pickup.planned = Some(w);
        }
        if let Some(w) = delivery_planned {
            self.delivery.planned = Some(w);
        }
        self.push_event(
            TransportEventCode::PickupPlanned,
            ts,
            source,
            Some("Booking confirmed"),
        );
        Ok(())
    }

    /* ---- Execution ---- */

    /// Record actual pickup; optionally set first delivery ETA.
    pub fn record_pickup_actual(
        &mut self,
        pickup_actual: DateTime<Utc>,
        first_eta_delivery: Option<DateTime<Utc>>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("record_pickup_actual")?;
        self.pickup.actual = Some(pickup_actual);
        if let Some(eta) = first_eta_delivery {
            self.delivery.estimated = Some(eta);
            self.push_event(
                TransportEventCode::DeliveryEtaUpdated,
                ts,
                "system",
                Some("ETA set at pickup"),
            );
        }
        self.status = TransportStatus::InTransit;
        self.push_event(TransportEventCode::PickupActual, ts, source, None);
        self.push_event(TransportEventCode::DepartureActual, ts, "telematics", None);
        Ok(())
    }

    /// Update rolling ETA to delivery.
    pub fn update_delivery_eta(
        &mut self,
        eta: DateTime<Utc>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
        note: Option<&str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("update_delivery_eta")?;
        self.delivery.estimated = Some(eta);
        self.push_event(TransportEventCode::ArrivalEtaUpdated, ts, source, note);
        Ok(())
    }

    /// Record actual arrival at consignee (gate-in / on-site).
    pub fn record_arrival_actual(
        &mut self,
        arrival_actual: DateTime<Utc>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("record_arrival_actual")?;
        // Keep timestamp in events; header ETA remains until handover.
        let _ = arrival_actual;
        self.push_event(TransportEventCode::ArrivalActual, ts, source, None);
        Ok(())
    }

    /// Record final delivery (handover complete). Optionally add a POD reference.
    pub fn record_delivery_actual(
        &mut self,
        delivery_actual: DateTime<Utc>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
        pod_ref: Option<&str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("record_delivery_actual")?;
        self.delivery.actual = Some(delivery_actual);
        self.delivery.estimated = None;
        self.push_event(TransportEventCode::DeliveryActual, ts, source, None);
        if let Some(pod) = pod_ref {
            self.push_event(
                TransportEventCode::ProofOfDeliveryAvailable,
                ts,
                "carrier",
                Some(pod),
            );
        }
        self.status = TransportStatus::Completed;
        Ok(())
    }

    /// Log a business exception with a note (delay, damage, customs hold, etc.).
    pub fn add_exception(
        &mut self,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
        note: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("add_exception")?;
        self.push_event(
            TransportEventCode::Exception,
            ts,
            source,
            Some(note.as_ref()),
        );
        Ok(())
    }

    /// Append an accessorial discovered during execution.
    pub fn add_accessorial(&mut self, acc: Accessorial) -> Result<(), TransportError> {
        self.ensure_mutable("add_accessorial")?;
        self.accessorials.push(acc);
        Ok(())
    }

    /// Cancel the request (cannot cancel once completed).
    pub fn cancel(
        &mut self,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
        reason: Option<&str>,
    ) -> Result<(), TransportError> {
        if matches!(self.status, TransportStatus::Completed) {
            return Err(TransportError::IsCompleted);
        }
        self.status = TransportStatus::Cancelled;
        self.push_event(TransportEventCode::Exception, ts, source, reason);
        Ok(())
    }

    /* ---- Multi-stop helpers ---- */

    pub fn set_stop_planned(
        &mut self,
        sequence: u32,
        arrival: Option<TimeWindow>,
        departure: Option<TimeWindow>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("set_stop_planned")?;
        let stop = self.find_stop_mut(sequence)?;
        if let Some(w) = arrival {
            stop.arrival.get_or_insert_with(StepTiming::default).planned = Some(w);
        }
        if let Some(w) = departure {
            stop.departure
                .get_or_insert_with(StepTiming::default)
                .planned = Some(w);
        }
        self.push_event(
            TransportEventCode::PickupPlanned,
            ts,
            source,
            Some("Stop plan updated"),
        );
        Ok(())
    }

    pub fn update_stop_eta(
        &mut self,
        sequence: u32,
        arrival_eta: Option<DateTime<Utc>>,
        departure_eta: Option<DateTime<Utc>>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
        note: Option<&str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("update_stop_eta")?;
        let stop = self.find_stop_mut(sequence)?;
        if let Some(eta) = arrival_eta {
            stop.arrival
                .get_or_insert_with(StepTiming::default)
                .estimated = Some(eta);
        }
        if let Some(eta) = departure_eta {
            stop.departure
                .get_or_insert_with(StepTiming::default)
                .estimated = Some(eta);
        }
        self.push_event(TransportEventCode::ArrivalEtaUpdated, ts, source, note);
        Ok(())
    }

    pub fn record_stop_actuals(
        &mut self,
        sequence: u32,
        arrival_actual: Option<DateTime<Utc>>,
        departure_actual: Option<DateTime<Utc>>,
        ts: DateTime<Utc>,
        source: impl AsRef<str>,
    ) -> Result<(), TransportError> {
        self.ensure_mutable("record_stop_actuals")?;
        let stop = self.find_stop_mut(sequence)?;
        let mut events = Vec::new();
        if let Some(a) = arrival_actual {
            stop.arrival.get_or_insert_with(StepTiming::default).actual = Some(a);
            events.push((
                TransportEventCode::ArrivalActual,
                ts,
                source.as_ref().to_string(),
                Some(format!("stop {}", sequence)),
            ));
        }
        if let Some(d) = departure_actual {
            stop.departure
                .get_or_insert_with(StepTiming::default)
                .actual = Some(d);
            events.push((
                TransportEventCode::DepartureActual,
                ts,
                source.as_ref().to_string(),
                Some(format!("stop {}", sequence)),
            ));
        }
        for (code, timestamp, source, note) in events {
            self.push_event(code, timestamp, source, note.as_deref());
        }
        Ok(())
    }

    /* ---- internal helpers ---- */

    fn ensure_mutable(&self, _action: &'static str) -> Result<(), TransportError> {
        if matches!(self.status, TransportStatus::Cancelled) {
            return Err(TransportError::IsCancelled);
        }
        if matches!(self.status, TransportStatus::Completed) {
            return Err(TransportError::IsCompleted);
        }
        Ok(())
    }

    fn push_event(
        &mut self,
        code: TransportEventCode,
        timestamp: DateTime<Utc>,
        source: impl AsRef<str>,
        note: Option<&str>,
    ) {
        self.events.push(TransportEvent {
            code,
            timestamp,
            source: Some(source.as_ref().to_string()),
            note: note.map(|s| s.to_string()),
        });
    }

    fn find_stop_mut(&mut self, seq: u32) -> Result<&mut LegStop, TransportError> {
        self.stops
            .iter_mut()
            .find(|s| s.sequence == seq)
            .ok_or(TransportError::StopNotFound(seq))
    }
}

/* ====================== Compact happy-path test ====================== */

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn addr(name: &str, city: &str, postal: &str) -> Address {
        Address {
            name: name.into(),
            street: vec!["Main Street 1".into()],
            city: city.into(),
            postal_code: postal.into(),
            country_iso: "DE".into(),
        }
    }

    #[test]
    fn road_ltl_happy_path() {
        let mut req = TransportationRqState {
            request_id: "TREQ-1".into(),
            status: TransportStatus::Draft,
            mode: TransportMode::Road,
            service_level: Some("LTL".into()),
            equipment: Some(Equipment {
                type_code: "Box Truck 7.5t".into(),
                temperature_c: None,
                extrinsic: None,
            }),
            consignor: addr("ACME DC Berlin", "Berlin", "10115"),
            consignee: addr("ACME Store Munich", "München", "80331"),
            stops: vec![],
            cargo: CargoSummary {
                pieces: 10,
                gross_weight_kg: 2500.0,
                volume_m3: Some(18.5),
                dangerous_goods: false,
                commodity_description: Some("Retail pallets".into()),
                codes: None,
            },
            pickup: StepTiming::default(),
            delivery: StepTiming::default(),
            accessorials: vec![Accessorial::TailLift, Accessorial::InsideDelivery],
            references: HashMap::from([("PO".into(), "4500012345".into())]),
            rate: None,
            events: vec![],
        };

        // Requested windows
        req.set_requested_windows(
            Some(
                TimeWindow::new(
                    Utc.with_ymd_and_hms(2025, 8, 14, 6, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2025, 8, 14, 10, 0, 0).unwrap(),
                )
                .unwrap(),
            ),
            Some(
                TimeWindow::new(
                    Utc.with_ymd_and_hms(2025, 8, 15, 7, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2025, 8, 15, 15, 0, 0).unwrap(),
                )
                .unwrap(),
            ),
        )
        .unwrap();
        req.mark_rfq_sent(
            Utc.with_ymd_and_hms(2025, 8, 13, 10, 0, 0).unwrap(),
            "buyer",
        )
        .unwrap();

        // Award + booking
        req.award(
            UnitRate {
                rate: Money::euro(89000, 0),
                unit_of_measure: UnitOfMeasure::EA,
                price_basis_quantity: None,
                term_reference: None,
            },
            Some(
                TimeWindow::new(
                    Utc.with_ymd_and_hms(2025, 8, 14, 7, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2025, 8, 14, 8, 0, 0).unwrap(),
                )
                .unwrap(),
            ),
            Some(
                TimeWindow::new(
                    Utc.with_ymd_and_hms(2025, 8, 15, 9, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2025, 8, 15, 12, 0, 0).unwrap(),
                )
                .unwrap(),
            ),
            Utc.with_ymd_and_hms(2025, 8, 13, 11, 0, 0).unwrap(),
            "carrier",
        )
        .unwrap();
        req.confirm_booking(
            Some(
                TimeWindow::new(
                    Utc.with_ymd_and_hms(2025, 8, 14, 7, 30, 0).unwrap(),
                    Utc.with_ymd_and_hms(2025, 8, 14, 8, 0, 0).unwrap(),
                )
                .unwrap(),
            ),
            None,
            Utc.with_ymd_and_hms(2025, 8, 13, 16, 22, 0).unwrap(),
            "carrier",
        )
        .unwrap();

        // Execution
        req.record_pickup_actual(
            Utc.with_ymd_and_hms(2025, 8, 14, 7, 44, 0).unwrap(),
            Some(Utc.with_ymd_and_hms(2025, 8, 15, 10, 20, 0).unwrap()),
            Utc.with_ymd_and_hms(2025, 8, 14, 7, 45, 0).unwrap(),
            "yard scan",
        )
        .unwrap();
        req.update_delivery_eta(
            Utc.with_ymd_and_hms(2025, 8, 15, 10, 18, 0).unwrap(),
            Utc.with_ymd_and_hms(2025, 8, 14, 12, 0, 0).unwrap(),
            "telematics",
            Some("traffic easing"),
        )
        .unwrap();
        req.record_arrival_actual(
            Utc.with_ymd_and_hms(2025, 8, 15, 9, 56, 0).unwrap(),
            Utc.with_ymd_and_hms(2025, 8, 15, 9, 56, 0).unwrap(),
            "carrier",
        )
        .unwrap();
        req.record_delivery_actual(
            Utc.with_ymd_and_hms(2025, 8, 15, 10, 18, 0).unwrap(),
            Utc.with_ymd_and_hms(2025, 8, 15, 10, 18, 0).unwrap(),
            "carrier",
            Some("POD-7745"),
        )
        .unwrap();

        assert_eq!(req.status, TransportStatus::Completed);
        assert!(req.delivery.actual.is_some());
        assert!(req.rate.is_some());
        assert!(!req.events.is_empty());
    }
}
