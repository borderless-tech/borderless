use core::fmt;
use core::str::FromStr;

use borderless_id_types::BorderlessId;

use crate::{Error, Participant, Result};

/// ISO 4217 currencies that account for the vast majority of global FX turnover (BIS 2022).
///
/// * Each variant name is the three letter ISO code.
/// * The discriminant is the ISO numeric code (`repr(u32)`).
/// * `Display` prints the common symbol (or symbol‑like shorthand).
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Currency {
    USD = 840,
    EUR = 978,
    JPY = 392,
    GBP = 826,
    CNY = 156,
    AUD = 36,
    CAD = 124,
    CHF = 756,
    HKD = 344,
    SGD = 702,
    SEK = 752,
    KRW = 410,
    NOK = 578,
    NZD = 554,
    INR = 356,
    MXN = 484,
    TWD = 901,
    ZAR = 710,
    BRL = 986,
    DKK = 208,
    PLN = 985,
    THB = 764,
    ILS = 376,
    IDR = 360,
    CZK = 203,
}

impl Currency {
    /// All supported currencies (in a fixed order).
    pub const ALL: [Currency; 25] = [
        Currency::USD,
        Currency::EUR,
        Currency::JPY,
        Currency::GBP,
        Currency::CNY,
        Currency::AUD,
        Currency::CAD,
        Currency::CHF,
        Currency::HKD,
        Currency::SGD,
        Currency::SEK,
        Currency::KRW,
        Currency::NOK,
        Currency::NZD,
        Currency::INR,
        Currency::MXN,
        Currency::TWD,
        Currency::ZAR,
        Currency::BRL,
        Currency::DKK,
        Currency::PLN,
        Currency::THB,
        Currency::ILS,
        Currency::IDR,
        Currency::CZK,
    ];

    /// Returns the commonly used symbol for the currency.
    pub const fn symbol(self) -> &'static str {
        match self {
            Currency::USD => "$",
            Currency::EUR => "€",
            Currency::JPY => "¥",
            Currency::GBP => "£",
            Currency::CNY => "CN¥",
            Currency::AUD => "A$",
            Currency::CAD => "C$",
            Currency::CHF => "CHF",
            Currency::HKD => "HK$",
            Currency::SGD => "S$",
            Currency::SEK => "kr",
            Currency::KRW => "₩",
            Currency::NOK => "kr",
            Currency::NZD => "NZ$",
            Currency::INR => "₹",
            Currency::MXN => "Mex$",
            Currency::TWD => "NT$",
            Currency::ZAR => "R",
            Currency::BRL => "R$",
            Currency::DKK => "kr",
            Currency::PLN => "zł",
            Currency::THB => "฿",
            Currency::ILS => "₪",
            Currency::IDR => "Rp",
            Currency::CZK => "Kč",
        }
    }

    /// Returns the full English name of the currency.
    pub const fn full_name(self) -> &'static str {
        match self {
            Currency::USD => "United States dollar",
            Currency::EUR => "Euro",
            Currency::JPY => "Japanese yen",
            Currency::GBP => "Pound sterling",
            Currency::CNY => "Chinese yuan (renminbi)",
            Currency::AUD => "Australian dollar",
            Currency::CAD => "Canadian dollar",
            Currency::CHF => "Swiss franc",
            Currency::HKD => "Hong Kong dollar",
            Currency::SGD => "Singapore dollar",
            Currency::SEK => "Swedish krona",
            Currency::KRW => "South Korean won",
            Currency::NOK => "Norwegian krone",
            Currency::NZD => "New Zealand dollar",
            Currency::INR => "Indian rupee",
            Currency::MXN => "Mexican peso",
            Currency::TWD => "New Taiwan dollar",
            Currency::ZAR => "South African rand",
            Currency::BRL => "Brazilian real",
            Currency::DKK => "Danish krone",
            Currency::PLN => "Polish złoty",
            Currency::THB => "Thai baht",
            Currency::ILS => "Israeli new shekel",
            Currency::IDR => "Indonesian rupiah",
            Currency::CZK => "Czech koruna",
        }
    }
}

#[derive(Debug)]
pub struct CurrencyError;
impl fmt::Display for CurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid ISO-4217 currency code")
    }
}
impl std::error::Error for CurrencyError {}

impl TryFrom<u32> for Currency {
    type Error = CurrencyError;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            840 => Ok(Currency::USD),
            978 => Ok(Currency::EUR),
            392 => Ok(Currency::JPY),
            826 => Ok(Currency::GBP),
            156 => Ok(Currency::CNY),
            36 => Ok(Currency::AUD),
            124 => Ok(Currency::CAD),
            756 => Ok(Currency::CHF),
            344 => Ok(Currency::HKD),
            702 => Ok(Currency::SGD),
            752 => Ok(Currency::SEK),
            410 => Ok(Currency::KRW),
            578 => Ok(Currency::NOK),
            554 => Ok(Currency::NZD),
            356 => Ok(Currency::INR),
            484 => Ok(Currency::MXN),
            901 => Ok(Currency::TWD),
            710 => Ok(Currency::ZAR),
            986 => Ok(Currency::BRL),
            208 => Ok(Currency::DKK),
            985 => Ok(Currency::PLN),
            764 => Ok(Currency::THB),
            376 => Ok(Currency::ILS),
            360 => Ok(Currency::IDR),
            203 => Ok(Currency::CZK),
            _ => Err(CurrencyError),
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}

/// Monetary value stored as an *integer number of thousandths* (1/1000) of the currency’s major unit.
///
/// Using thousandths lets us represent all ISO 4217 currencies (the largest fraction
/// in normal use is the Bahraini dinar’s 3 decimal places) without loss.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Money {
    /// Number of thousandths of a unit (can be negative).
    amount_milli: i64,
    currency: Currency,
}

impl Money {
    /// Creates a new `Money` from an *integer* amount of thousandths.
    pub const fn from_thousandths(amount_milli: i64, currency: Currency) -> Self {
        Self {
            amount_milli,
            currency,
        }
    }

    /// Returns the currency of this value.
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the stored amount in thousandths.
    pub const fn amount_thousandths(&self) -> i64 {
        self.amount_milli
    }

    /// Returns the decimal value as `f64` (possible loss of precision for very large values).
    pub fn amount(&self) -> f64 {
        self.amount_milli as f64 / 1000.0
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.amount_milli.abs();
        let integral = abs / 1000;
        let fractional = (abs % 1000) as u32;

        // Build fractional string, trimming trailing zeros.
        if fractional == 0 {
            if self.amount_milli < 0 {
                write!(f, "-{} {}", integral, self.currency)
            } else {
                write!(f, "{} {}", integral, self.currency)
            }
        } else {
            let mut frac_str = format!("{:03}", fractional);
            while frac_str.ends_with('0') {
                frac_str.pop();
            }
            if self.amount_milli < 0 {
                write!(f, "-{}.{} {}", integral, frac_str, self.currency)
            } else {
                write!(f, "{}.{} {}", integral, frac_str, self.currency)
            }
        }
    }
}

/// Simple error type for `Money::from_str`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseMoneyError {
    InvalidFormat,
    UnknownCurrency,
    Overflow,
}

impl fmt::Display for ParseMoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseMoneyError::InvalidFormat => write!(f, "invalid money format"),
            ParseMoneyError::UnknownCurrency => write!(f, "unknown currency symbol"),
            ParseMoneyError::Overflow => write!(f, "amount overflow"),
        }
    }
}
impl std::error::Error for ParseMoneyError {}

impl FromStr for Money {
    type Err = ParseMoneyError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let s = input.trim();

        // Try to find a trailing currency symbol (optionally preceded by space)
        for &cur in Currency::ALL.iter() {
            let sym = cur.symbol();
            if s.ends_with(sym) {
                // slice off the symbol and any whitespace before it.
                let number_part = s[..s.len() - sym.len()].trim_end();

                // Replace comma with dot to unify decimal separator.
                let unified = number_part.replace(',', ".");
                if unified.is_empty() {
                    println!("--");
                    return Err(ParseMoneyError::InvalidFormat);
                }

                // Handle optional negative sign.
                let negative = unified.starts_with('-');
                let number_core = if negative { &unified[1..] } else { &unified };

                // Split on optional decimal point.
                let mut parts = number_core.split('.');
                let int_part_str = parts.next().unwrap();
                let frac_part_str = parts.next();
                if parts.next().is_some() {
                    // more than one dot
                    println!("too many dots");
                    return Err(ParseMoneyError::InvalidFormat);
                }

                let integral: i64 = int_part_str
                    .parse()
                    .map_err(|_| ParseMoneyError::InvalidFormat)?;

                let mut milli: i64 = integral
                    .checked_mul(1000)
                    .ok_or(ParseMoneyError::Overflow)?;

                if let Some(frac) = frac_part_str {
                    if frac.len() > 3 {
                        println!("fracs: {}", frac.len());
                        return Err(ParseMoneyError::InvalidFormat);
                    }
                    // Pad to 3 digits on the right by adding zeros.
                    let mut padded = frac.to_owned();
                    while padded.len() < 3 {
                        padded.push('0');
                    }
                    let frac_val: i64 =
                        padded.parse().map_err(|_| ParseMoneyError::InvalidFormat)?;
                    milli = milli
                        .checked_add(frac_val)
                        .ok_or(ParseMoneyError::Overflow)?;
                }

                if negative {
                    milli = -milli;
                }

                return Ok(Money {
                    amount_milli: milli,
                    currency: cur,
                });
            }
        }
        Err(ParseMoneyError::UnknownCurrency)
    }
}

#[derive(Debug)]
pub struct EntryTypeErr;
impl fmt::Display for EntryTypeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid entry-type")
    }
}
impl std::error::Error for EntryTypeErr {}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EntryType {
    /// A new debt is created
    CREATE = 0,
    /// An existing debt is settled and thus removed
    SETTLE = 1,
    /// Cancels / removes an existing debt
    CANCEL = 2,
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

pub struct LedgerEntry {
    pub creditor: BorderlessId,
    pub debitor: BorderlessId,
    pub amount_milli: i64,
    pub tax_milli: i64,
    pub currency: Currency,
    pub kind: EntryType,
    pub tag: String,
}

// 2 * 16 byte for the ids
// 2 *  8 byte for the amount + tax
// 2 *  4 byte for currency + kind
pub const LEDGER_ENTRY_MIN_LEN: usize = 32 + 16 + 8;

impl LedgerEntry {
    pub fn get_money(&self) -> Money {
        Money {
            amount_milli: self.amount_milli,
            currency: self.currency,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(LEDGER_ENTRY_MIN_LEN + self.tag.len());
        bytes.extend(self.creditor.as_bytes());
        bytes.extend(self.debitor.as_bytes());
        bytes.extend(self.amount_milli.to_be_bytes());
        bytes.extend(self.tax_milli.to_be_bytes());
        bytes.extend((self.currency as u32).to_be_bytes());
        bytes.extend((self.kind as u32).to_be_bytes());
        bytes.extend(self.tag.as_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        LedgerEntry::check_buffer(bytes)?;
        // 16 byte buffer
        let mut buf = [0; 16];
        buf.copy_from_slice(&bytes[0..16]);
        let creditor = BorderlessId::from_bytes(buf);
        buf.copy_from_slice(&bytes[16..32]);
        let debitor = BorderlessId::from_bytes(buf);
        // 8 byte buffer
        let mut buf = [0; 8];
        buf.copy_from_slice(&bytes[32..40]);
        let amount_milli = i64::from_be_bytes(buf);
        buf.copy_from_slice(&bytes[40..48]);
        let tax_milli = i64::from_be_bytes(buf);
        // 4 byte buffer
        let mut buf = [0; 4];
        buf.copy_from_slice(&bytes[48..52]);
        let currency = Currency::try_from(u32::from_be_bytes(buf))?;
        buf.copy_from_slice(&bytes[52..56]);
        let kind = EntryType::try_from(u32::from_be_bytes(buf))?;
        let tag = String::from_utf8_lossy(&bytes[56..]);
        Ok(LedgerEntry {
            creditor,
            debitor,
            amount_milli,
            tax_milli,
            currency,
            kind,
            tag: tag.into_owned(),
        })
    }

    pub fn check_buffer(bytes: &[u8]) -> Result<()> {
        if bytes.len() < LEDGER_ENTRY_MIN_LEN {
            return Err(Error::msg("slice is too short for a ledger-entry"));
        }
        Ok(())
    }

    // POC how we can generate a view over a byte buffer
    pub unsafe fn view_kind(bytes: &[u8]) -> Result<EntryType> {
        let mut buf = [0; 4];
        buf.copy_from_slice(&bytes.get_unchecked(52..56));
        let kind = EntryType::try_from(u32::from_be_bytes(buf)).unwrap();
        Ok(kind)
    }
}

pub struct EntryBuilder<C, D> {
    creditor: C,
    debitor: D,
    amount_milli: Option<i64>,
    tax_milli: Option<i64>,
    currency: Option<Currency>,
    kind: EntryType,
    tag: Option<String>,
}

impl<C, D> EntryBuilder<C, D>
where
    C: Participant,
    D: Participant,
{
    pub fn with_amount(self, money: Money) -> Result<Self> {
        if self
            .currency
            .map(|c| c != money.currency)
            .unwrap_or_default()
        {
            return Err(Error::msg("Amount and Tax must use the same currency"));
        }
        Ok(Self {
            creditor: self.creditor,
            debitor: self.debitor,
            amount_milli: Some(money.amount_milli),
            tax_milli: self.tax_milli,
            currency: Some(money.currency),
            kind: self.kind,
            tag: self.tag,
        })
    }

    pub fn with_tax(self, tax: Money) -> Result<Self> {
        if self.currency.map(|c| c != tax.currency).unwrap_or_default() {
            return Err(Error::msg("Amount and Tax must use the same currency"));
        }
        Ok(Self {
            creditor: self.creditor,
            debitor: self.debitor,
            amount_milli: self.amount_milli,
            tax_milli: Some(tax.amount_milli),
            currency: Some(tax.currency),
            kind: self.kind,
            tag: self.tag,
        })
    }

    pub fn with_tag(self, tag: impl AsRef<str>) -> Self {
        Self {
            creditor: self.creditor,
            debitor: self.debitor,
            amount_milli: self.amount_milli,
            tax_milli: self.tax_milli,
            currency: self.currency,
            kind: self.kind,
            tag: Some(tag.as_ref().to_string()),
        }
    }
    pub fn build(self) -> Result<LedgerEntry> {
        let creditor = C::get_participant(self.creditor)?;
        let debitor = D::get_participant(self.debitor)?;
        let tag = self.tag.unwrap_or_default();
        if self.amount_milli.is_none() {
            return Err(Error::msg("missing amount"));
        }
        if self.currency.is_none() {
            return Err(Error::msg("missing currency"));
        }
        let tax_milli = self.tax_milli.unwrap_or_default();

        let ledger_entry = LedgerEntry {
            debitor,
            creditor,
            amount_milli: self.amount_milli.unwrap(),
            tax_milli,
            currency: self.currency.unwrap(),
            kind: self.kind,
            tag,
        };
        Ok(ledger_entry)
    }

    pub fn execute(self) -> Result<()> {
        let entry = self.build()?;
        Ok(())
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
        amount_milli: None,
        tax_milli: None,
        currency: None,
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
        amount_milli: None,
        tax_milli: None,
        currency: None,
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
        amount_milli: None,
        tax_milli: None,
        currency: None,
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

    #[test]
    fn usd_symbol() {
        assert_eq!(Currency::USD.symbol(), "$");
    }

    #[test]
    fn eur_full_name() {
        assert_eq!(Currency::EUR.full_name(), "Euro");
    }

    #[test]
    fn repr_value_matches_iso() {
        assert_eq!(Currency::JPY as u32, 392);
    }

    #[test]
    fn parse_eur_comma() {
        let m: Money = "10,23 €".parse().unwrap();
        assert_eq!(m.amount_thousandths(), 10230);
        assert_eq!(m.currency(), Currency::EUR);
    }

    #[test]
    fn parse_usd_dot() {
        let m: Money = "-99.1$".parse().unwrap();
        assert_eq!(m.amount_thousandths(), -99100);
        assert_eq!(m.currency(), Currency::USD);
    }

    #[test]
    fn display_round_trip() {
        let original = "123.456 NZ$";
        let m: Money = original.parse().unwrap();
        let round = m.to_string();
        assert_eq!(round, "123.456 NZ$");
    }

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
            .with_amount("100 €".parse()?)?
            .with_tax("19 €".parse()?)?
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
            .with_amount("100 €".parse()?)?
            .with_tax("19 €".parse()?)?
            .build()?;

        let e2 = transfer(debitor.id, creditor.id)
            .with_amount("100 €".parse()?)?
            .with_tax("19 €".parse()?)?
            .build()?;

        let e3 = transfer(debitor.alias, creditor.alias)
            .with_amount("100 €".parse()?)?
            .with_tax("19 €".parse()?)?
            .build()?;

        assert_eq!(e1.to_bytes(), e2.to_bytes());
        assert_eq!(e2.to_bytes(), e3.to_bytes());
        assert_eq!(e1.to_bytes(), e3.to_bytes());
        Ok(())
    }

    #[test]
    fn encode_decode() -> Result<()> {
        let (creditor, debitor) = prepare_participants();
        let entry = transfer(debitor, creditor)
            .with_amount("100 €".parse()?)?
            .with_tax("19 €".parse()?)?
            .with_tag("test-transfer")
            .build()?;
        let bytes = entry.to_bytes();
        let decoded = LedgerEntry::from_bytes(&bytes)?;
        assert_eq!(decoded.amount_milli, 100_000);
        assert_eq!(decoded.tax_milli, 19_000);
        assert_eq!(decoded.currency, Currency::EUR);
        assert_eq!(decoded.tag, "test-transfer");
        Ok(())
    }
}
