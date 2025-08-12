use chrono::{DateTime, NaiveDate, Utc};
use currency_4217::Money;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The full request datatype for a purchase order
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderRequest {
    /// Header of the order request
    ///
    /// This contains metadata about the order such as the `order_id`,
    /// the address where the order must be shipped to and additional information.
    pub header: OrderRequestHeader,
    /// Items of the order
    ///
    /// These are the concrete items that were purchased in this order
    pub items: Vec<ItemOut>,
}

/// Header of an order-request
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrderRequestHeader {
    /// Identifier of this order
    pub order_id: String,
    /// Date of the order
    pub order_date: DateTime<Utc>,
    /// Specifies weather or not this was a new order, or a modification to an existing one
    pub order_type: OrderType,
    /// Sum of all item prices plus shipping cost
    pub total: Money,
    /// Shipping Address
    pub ship_to: Address,
    /// Billing Address ( may be identical to shipping address )
    pub bill_to: Address,
    /// Shipping cost
    pub shipping: Option<Money>,
    /// Tax
    pub tax: Option<Money>,
    /// Optional comments on this order
    pub comments: Option<String>,
}

/// A single item position in the purchase order
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemOut {
    /// The line-number in the list of items that are purchased.
    pub line_number: u32,
    /// Total quantity of that item
    pub quantity: u32,
    /// Optional requested date of delivery, if possible
    pub requested_delivery_date: Option<NaiveDate>,
    /// Item that was purchased
    pub item: Item,
}

/// Datatype for a single item - bundles item-id and item-detail
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    /// ID of the requested item
    pub item_id: ItemID,
    /// Additional information like unit-price, description, unit of measure etc.
    pub detail: ItemDetail,
}

/// Supplier and buyer part/item ID
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemID {
    /// If known, the buyer can supply his/her own ID for this item
    ///
    /// If specified, this information can be used to map the item into the buyers IT-Systems
    pub buyer_part_id: Option<String>,
    /// Item-ID from the supplier
    pub supplier_part_id: String,
}

/// Details of a single position in the purchase order
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemDetail {
    /// For goods: price per unit
    pub unit_price: Option<Money>,

    /// Human-readable description
    pub description: String,

    /// For goods: unit of measure. For services, prefer `unit_rate` (cXML deprecates UnitPrice+UOM for services).
    pub unit_of_measure: Option<UnitOfMeasure>,

    pub classification: Option<Classification>,
    pub manufacturer: Option<ManufacturerInfo>,
    pub extrinsic: Option<HashMap<String, String>>,

    /// Services: cXML-style service pricing (preferred over `unit_price` for services)
    pub unit_rate: Option<UnitRate>,

    /// Services: detailed info (labor/fee/travel), like in cXML `<SpendDetail>`
    pub spend_detail: Option<SpendDetail>,
}

/// Address definition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Address {
    /// Name of the company / person that resides at the address
    pub name: String,
    /// Street information
    pub street: Vec<String>,
    /// City
    pub city: String,
    /// Postal code as string
    pub postal_code: String,
    /// Country string according to ISO 3166
    pub country_iso: String,
}

/// Information about the manufacturer
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ManufacturerInfo {
    pub part_id: String,
    pub name: String,
}

/// Classification (UNSPSC, ECLASS, …)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Classification {
    pub domain: ClassificationDomain,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ClassificationDomain {
    UNSPSC,
    ECLASS,
    Custom(String),
}

/// Defines if the order was "new", an "update" or should be "delete"d
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum OrderType {
    New,
    Update,
    Delete,
}

/// Service pricing per time/measure unit (cXML `<UnitRate>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnitRate {
    /// Money amount of the rate
    pub rate: Money,
    /// Unit the service is provided in (e.g., hours)
    pub unit_of_measure: UnitOfMeasure,
    /// Optional price basis quantity (e.g., rate applies per 8 hours)
    pub price_basis_quantity: Option<PriceBasisQuantity>,
    /// Optional rate code/context (e.g., payCode=Overtime)
    pub term_reference: Option<TermReference>,
}

/// Quantity + UOM that the price is based on (cXML `<PriceBasisQuantity>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PriceBasisQuantity {
    pub quantity: u32,
    pub unit_of_measure: UnitOfMeasure,
}

/// Identifies the meaning of a `UnitRate` (cXML `<TermReference>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TermReference {
    pub term_name: String,
    pub term: String,
}

/// Time period for a service (cXML `<Period>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Period {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Mirrors cXML `<SpendDetail>` → may contain Travel, Fee, and/or Labor
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpendDetail {
    pub travel_detail: Option<TravelDetail>,
    pub fee_detail: Option<FeeDetail>,
    pub labor_detail: Option<LaborDetail>,
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Labor service details (subset of cXML `<LaborDetail>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LaborDetail {
    /// Supplier quote/proposal reference (cXML `supplierReferenceCode` attribute)
    pub supplier_reference_code: Option<String>,
    /// The applicable rate (often required for labor)
    pub unit_rate: Option<UnitRate>,
    /// Period the labor occurred
    pub period: Option<Period>,
    /// The contractor performing the work
    pub contractor: Option<Contractor>,
    /// Free-text description of the job
    pub job_description: Option<String>,
    /// Person supervising the contractor
    pub supervisor: Option<ContactInfo>,
    /// Where the work is performed
    pub work_location: Option<Address>,
    /// Extra machine-readable fields
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Optional “fee” style charge
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FeeDetail {
    pub amount: Money,
    pub description: Option<String>,
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Optional “travel” style cost bucket (kept simple; extend as needed)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TravelDetail {
    pub amount: Option<Money>,
    pub description: Option<String>,
    pub period: Option<Period>,
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Who did the work (cXML `<Contractor>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Contractor {
    pub identifier: Option<ContractorIdentifier>,
    pub contact: Option<ContactInfo>,
}

/// Contractor identifier (cXML `<ContractorIdentifier domain=...>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractorIdentifier {
    pub domain: ContractorIdentifierDomain,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ContractorIdentifierDomain {
    /// cXML: "buyerReferenceID"
    BuyerReferenceID,
    /// cXML: "supplierReferenceID"
    SupplierReferenceID,
    /// Any other agreed domain string
    Custom(String),
}

/// Minimal contact info for supervisors/contractors (maps to cXML `<Contact>`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContactInfo {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<Address>,
}

/// ISO Units-of-Measure-Codes or a freely defined measure
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum UnitOfMeasure {
    /// Each
    EA,
    /// Kilogram
    KG,
    /// Liter
    LTR,
    /// Meter
    MTR,
    /// Hour
    HUR,
    /// Day
    DAY,
    /// Month
    MON,
    /// A custom unit
    Custom(String),
}

/// A transport service request/tender/booking (A→B or multi-stop)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransportationRequest {
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

/// Where you are in the request→execution flow
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum TransportStatus {
    Draft,     // built but not sent
    RfqSent,   // RFQ/tender sent to providers
    Awarded,   // provider selected
    Booked,    // transport order/booking confirmed
    InTransit, // pickup done and en route
    Completed, // delivered (actuals available)
    Cancelled,
}

/// Road/Air/Ocean/Rail or mixed
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum TransportMode {
    Road,
    Air,
    Ocean,
    Rail,
    Multimodal,
}

/// Requested vs planned vs estimated vs actual for one step (pickup or delivery)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

/// A time window (site availability / appointment window)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimeWindow {
    pub earliest_utc: DateTime<Utc>,
    pub latest_utc: DateTime<Utc>,
}

/// An intermediate stop (for multi-pick/drop). Each stop can model both arrival & departure.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Equipment {
    /// Examples: "Box Truck 7.5t", "13.6m tautliner", "40HC", "20DV", "Reefer Trailer"
    pub type_code: String,
    /// Temperature range if controlled transport is needed (min, max)
    pub temperature_c: Option<(f32, f32)>,
    /// Free slots for axle/weight/class, door type, etc.
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Accessorial services (used to add cost / constraints on the delivery)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransportEvent {
    pub code: TransportEventCode,
    pub timestamp: DateTime<Utc>,
    /// Who reported it (carrier/TSP, telematics, port, visibility provider…)
    pub source: Option<String>,
    pub note: Option<String>,
}

/// A compact event vocabulary
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
