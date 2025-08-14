use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// ISO Units-of-Measure-Codes or a freely defined measure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
