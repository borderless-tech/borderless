use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::*;

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
    /// Provider placed his bid
    BidSent,
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
