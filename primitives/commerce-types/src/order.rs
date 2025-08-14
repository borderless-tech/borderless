use crate::core::*;
use chrono::{DateTime, NaiveDate, Utc};
use currency_4217::Money;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The full request datatype for a purchase order
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    /// ID of the requested item
    pub item_id: ItemID,
    /// Additional information like unit-price, description, unit of measure etc.
    pub detail: ItemDetail,
}

/// Supplier and buyer part/item ID
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ItemID {
    /// If known, the buyer can supply his/her own ID for this item
    ///
    /// If specified, this information can be used to map the item into the buyers IT-Systems
    pub buyer_part_id: Option<String>,
    /// Item-ID from the supplier
    pub supplier_part_id: String,
}

/// Details of a single position in the purchase order
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Information about the manufacturer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ManufacturerInfo {
    pub part_id: String,
    pub name: String,
}

/// Classification (UNSPSC, ECLASS, …)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Classification {
    pub domain: ClassificationDomain,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ClassificationDomain {
    UNSPSC,
    ECLASS,
    Custom(String),
}

/// Defines if the order was "new", an "update" or should be "delete"d
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OrderType {
    New,
    Update,
    Delete,
}
