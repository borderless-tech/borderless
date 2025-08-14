use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use chrono::{DateTime, Utc};
pub use currency_4217::{Currency, Money};

/// Address definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Minimal contact info for supervisors/contractors (maps to cXML `<Contact>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContactInfo {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<Address>,
}

/// Who did the work (cXML `<Contractor>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Contractor {
    pub identifier: Option<ContractorIdentifier>,
    pub contact: Option<ContactInfo>,
}

/// Contractor identifier (cXML `<ContractorIdentifier domain=...>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContractorIdentifier {
    pub domain: ContractorIdentifierDomain,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ContractorIdentifierDomain {
    /// cXML: "buyerReferenceID"
    BuyerReferenceID,
    /// cXML: "supplierReferenceID"
    SupplierReferenceID,
    /// Any other agreed domain string
    Custom(String),
}

/// Quantity + UOM that the price is based on (cXML `<PriceBasisQuantity>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PriceBasisQuantity {
    pub quantity: u32,
    pub unit_of_measure: UnitOfMeasure,
}

/// Flexible Unit-of-Measure that supports common known codes AND arbitrary strings.
/// - Known codes serialize to a canonical short code (see `rename`).
/// - On input, we also accept popular aliases (UNECE codes and ERP shorthands).
/// - Unknown strings deserialize to `UnitOfMeasure::Other("...")` instead of failing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UnitOfMeasure {
    Known(KnownUom),
    Other(String),
}

/// The most-used UoMs for commerce & hardware, with common aliases.
///
/// Canonical serialized codes (via `rename`) are chosen to be friendly & recognizable,
/// while aliases include UNECE Rec 20 codes and widely used ERP shorthands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum KnownUom {
    // ---- Count / packaging ---------------------------------------------------
    /// Piece / Each
    #[serde(rename = "PCE", alias = "EA", alias = "C62")]
    Piece,
    /// Package
    #[serde(rename = "PKG", alias = "XPK", alias = "PK")]
    Package,
    /// Set
    #[serde(rename = "SET")]
    Set,
    /// Box
    #[serde(rename = "BOX", alias = "BX")]
    Box,
    /// Bag
    #[serde(rename = "BAG", alias = "BG")]
    Bag,
    /// Roll
    #[serde(rename = "ROL", alias = "RL")]
    Roll,
    /// Pair
    #[serde(rename = "PAI", alias = "PR")]
    Pair,

    // ---- Mass ----------------------------------------------------------------
    /// Kilogram
    #[serde(rename = "KG", alias = "KGM")]
    Kilogram,
    /// Gram
    #[serde(rename = "G", alias = "GRM")]
    Gram,
    /// Tonne (metric ton)
    #[serde(rename = "T", alias = "TNE")]
    Tonne,

    // ---- Volume --------------------------------------------------------------
    /// Litre
    #[serde(rename = "LTR", alias = "L")]
    Litre,
    /// Millilitre
    #[serde(rename = "ML", alias = "MLT")]
    Millilitre,
    /// Cubic meter
    #[serde(rename = "CBM", alias = "M3", alias = "MTQ")]
    CubicMeter,

    // ---- Length / area -------------------------------------------------------
    /// Meter
    #[serde(rename = "MTR", alias = "M")]
    Meter,
    /// Centimeter
    #[serde(rename = "CM", alias = "CMT")]
    Centimeter,
    /// Millimeter
    #[serde(rename = "MM", alias = "MMT")]
    Millimeter,
    /// Square meter
    #[serde(rename = "SQM", alias = "M2", alias = "MTK")]
    SquareMeter,

    // ---- Time ----------------------------------------------------------------
    /// Hour
    #[serde(rename = "HUR", alias = "H")]
    Hour,
    /// Minute
    #[serde(rename = "MIN")]
    Minute,
    /// Second
    #[serde(rename = "SEC", alias = "S")]
    Second,
    /// Day
    #[serde(rename = "DAY")]
    Day,
    /// Month
    #[serde(rename = "MON")]
    Month,
    /// Year
    #[serde(rename = "YR", alias = "ANN")]
    Year,
}

/// Service pricing per time/measure unit (cXML `<UnitRate>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Identifies the meaning of a `UnitRate` (cXML `<TermReference>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TermReference {
    pub term_name: String,
    pub term: String,
}

/// Time period for a service (cXML `<Period>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Period {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Mirrors cXML `<SpendDetail>` → may contain Travel, Fee, and/or Labor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SpendDetail {
    pub travel_detail: Option<TravelDetail>,
    pub fee_detail: Option<FeeDetail>,
    pub labor_detail: Option<LaborDetail>,
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Labor service details (subset of cXML `<LaborDetail>`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Optional “travel” style cost bucket (kept simple; extend as needed)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TravelDetail {
    pub amount: Option<Money>,
    pub description: Option<String>,
    pub period: Option<Period>,
    pub extrinsic: Option<HashMap<String, String>>,
}

/// Optional “fee” style charge
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FeeDetail {
    pub amount: Money,
    pub description: Option<String>,
    pub extrinsic: Option<HashMap<String, String>>,
}
