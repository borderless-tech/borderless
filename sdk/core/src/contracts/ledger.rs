use core::fmt;

use borderless_id_types::BorderlessId;
use serde::{Deserialize, Serialize};

use crate::{Error, Participant, Result, __private::create_ledger_entry};

pub use currency_4217::{Currency, Money};

#[derive(Debug)]
pub struct EntryTypeErr;
impl fmt::Display for EntryTypeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid entry-type")
    }
}
impl std::error::Error for EntryTypeErr {}

/// A ledger entry on the guest side
#[derive(Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub creditor: BorderlessId,
    pub debitor: BorderlessId,
    pub amount_milli: i64,
    pub tax_milli: i64,
    pub currency: Currency,
    pub kind: EntryType,
    pub tag: String,
}

impl fmt::Display for LedgerEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {}->{}, amount={}, tax={}, cur={}, tag={}",
            self.kind,
            self.debitor,
            self.creditor,
            self.amount_milli,
            self.tax_milli,
            self.currency,
            self.tag
        )
    }
}

impl LedgerEntry {
    pub fn get_money(&self) -> Money {
        Money {
            amount_milli: self.amount_milli,
            currency: self.currency,
        }
    }

    pub fn to_bytes(&self) -> std::result::Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }

    pub fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub fn execute(self) -> Result<()> {
        create_ledger_entry(self)
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryType {
    /// A new debt is created
    CREATE = 0,
    /// An existing debt is settled and thus removed
    SETTLE = 1,
    /// Cancels / removes an existing debt
    CANCEL = 2,
}

impl EntryType {
    pub fn to_be_bytes(&self) -> [u8; 4] {
        (*self as u32).to_be_bytes()
    }

    pub fn from_be_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.try_into().ok()?;
        let num = u32::from_be_bytes(b);
        num.try_into().ok()
    }
}

impl TryFrom<u32> for EntryType {
    type Error = EntryTypeErr;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(EntryType::CREATE),
            1 => Ok(EntryType::SETTLE),
            2 => Ok(EntryType::CANCEL),
            _ => Err(EntryTypeErr),
        }
    }
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::CREATE => f.write_str("CREATE"),
            EntryType::SETTLE => f.write_str("SETTLE"),
            EntryType::CANCEL => f.write_str("CANCEL"),
        }
    }
}

pub struct EntryBuilder<C, D> {
    creditor: C,
    debitor: D,
    amount: Option<Money>,
    tax: Option<Money>,
    kind: EntryType,
    tag: Option<String>,
}

impl<C, D> EntryBuilder<C, D>
where
    C: Participant,
    D: Participant,
{
    pub fn with_amount(self, money: Money) -> Self {
        Self {
            creditor: self.creditor,
            debitor: self.debitor,
            amount: Some(money),
            tax: self.tax,
            kind: self.kind,
            tag: self.tag,
        }
    }

    pub fn with_tax(self, tax: Money) -> Self {
        Self {
            creditor: self.creditor,
            debitor: self.debitor,
            amount: self.amount,
            tax: Some(tax),
            kind: self.kind,
            tag: self.tag,
        }
    }

    pub fn with_tag(self, tag: impl AsRef<str>) -> Self {
        Self {
            creditor: self.creditor,
            debitor: self.debitor,
            amount: self.amount,
            tax: self.tax,
            kind: self.kind,
            tag: Some(tag.as_ref().to_string()),
        }
    }
    pub fn build(self) -> Result<LedgerEntry> {
        let creditor = C::get_participant(self.creditor)?;
        let debitor = D::get_participant(self.debitor)?;
        let tag = self.tag.unwrap_or_default();

        let (amount, tax_milli) = match (self.amount, self.tax) {
            (Some(a), Some(t)) => {
                if a.currency != t.currency {
                    return Err(Error::msg("amount and tax must use same currency"));
                }
                (a, t.amount_milli)
            }
            (Some(a), None) => (a, 0),
            (None, _) => return Err(Error::msg("missing amount")),
        };

        let ledger_entry = LedgerEntry {
            debitor,
            creditor,
            amount_milli: amount.amount_milli,
            tax_milli,
            currency: amount.currency,
            kind: self.kind,
            tag,
        };
        Ok(ledger_entry)
    }

    pub fn execute(self) -> Result<()> {
        let entry = self.build()?;
        create_ledger_entry(entry)
    }
}

pub fn transfer<C, D>(from: D, to: C) -> EntryBuilder<C, D>
where
    C: Participant,
    D: Participant,
{
    EntryBuilder {
        creditor: to,
        debitor: from,
        amount: None,
        tax: None,
        kind: EntryType::CREATE,
        tag: None,
    }
}

pub fn settle_debt<C, D>(from: D, to: C) -> EntryBuilder<C, D>
where
    C: Participant,
    D: Participant,
{
    EntryBuilder {
        creditor: to,
        debitor: from,
        amount: None,
        tax: None,
        kind: EntryType::SETTLE,
        tag: None,
    }
}

pub fn cancellation<C, D>(from: D, to: C) -> EntryBuilder<C, D>
where
    C: Participant,
    D: Participant,
{
    EntryBuilder {
        creditor: to,
        debitor: from,
        amount: None,
        tax: None,
        kind: EntryType::CANCEL,
        tag: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        __private::storage_keys::{BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS},
        common::Participant,
    };

    use super::*;

    fn prepare_participants() -> (Participant, Participant) {
        use crate::__private::env::off_chain::storage_write;

        let debitor = Participant {
            id: BorderlessId::generate(),
            alias: "buyer".to_string(),
            roles: Vec::new(),
        };
        let creditor = Participant {
            id: BorderlessId::generate(),
            alias: "seller".to_string(),
            roles: Vec::new(),
        };
        let participants = vec![debitor.clone(), creditor.clone()];
        let bytes = postcard::to_allocvec(&participants).unwrap();

        // Prepare participants:
        storage_write(BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS, &bytes);

        (creditor, debitor)
    }

    #[test]
    fn create_ledger_entry() -> Result<()> {
        let (creditor, debitor) = prepare_participants();
        let entry = transfer(debitor, creditor)
            .with_amount("100 €".parse()?)
            .with_tax("19 €".parse()?)
            .with_tag("test-transfer")
            .build()?;
        assert_eq!(entry.amount_milli, 100_000);
        assert_eq!(entry.tax_milli, 19_000);
        assert_eq!(entry.currency, Currency::EUR);
        assert_eq!(entry.tag, "test-transfer");
        Ok(())
    }

    #[test]
    fn participant_logic() -> Result<()> {
        let (creditor, debitor) = prepare_participants();
        let e1 = transfer(&debitor, &creditor)
            .with_amount("100 €".parse()?)
            .with_tax("19 €".parse()?)
            .build()?;

        let e2 = transfer(debitor.id, creditor.id)
            .with_amount("100 €".parse()?)
            .with_tax("19 €".parse()?)
            .build()?;

        let e3 = transfer(debitor.alias, creditor.alias)
            .with_amount("100 €".parse()?)
            .with_tax("19 €".parse()?)
            .build()?;

        assert_eq!(e1.to_bytes()?, e2.to_bytes()?);
        assert_eq!(e2.to_bytes()?, e3.to_bytes()?);
        assert_eq!(e1.to_bytes()?, e3.to_bytes()?);
        Ok(())
    }

    #[test]
    fn encode_decode() -> Result<()> {
        let (creditor, debitor) = prepare_participants();
        let entry = transfer(debitor, creditor)
            .with_amount("100 €".parse()?)
            .with_tax("19 €".parse()?)
            .with_tag("test-transfer")
            .build()?;
        let bytes = entry.to_bytes()?;
        let decoded = LedgerEntry::from_bytes(&bytes)?;
        assert_eq!(decoded.amount_milli, 100_000);
        assert_eq!(decoded.tax_milli, 19_000);
        assert_eq!(decoded.currency, Currency::EUR);
        assert_eq!(decoded.tag, "test-transfer");
        Ok(())
    }
}
