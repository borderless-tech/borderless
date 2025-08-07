use core::fmt;
use core::str::FromStr;

use borderless_id_types::BorderlessId;
use serde::{Deserialize, Serialize};

use crate::{Error, Participant, Result, __private::create_ledger_entry};

/// ISO 4217 currencies that account for the vast majority of global FX turnover (BIS 2022).
///
/// * Each variant name is the three letter ISO code.
/// * The discriminant is the ISO numeric code (`repr(u32)`).
/// * `Display` prints the common symbol (or symbol like shorthand).
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            Currency::NOK => "NKr",
            Currency::NZD => "NZ$",
            Currency::INR => "₹",
            Currency::MXN => "Mex$",
            Currency::TWD => "NT$",
            Currency::ZAR => "R",
            Currency::BRL => "R$",
            Currency::DKK => "DKK",
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

    pub const fn fracs(&self) -> u8 {
        match self {
            Currency::JPY | Currency::KRW => 0, // no decimals
            _ => 2,                             // all others use two decimals
        }
    }

    pub fn to_be_bytes(&self) -> [u8; 4] {
        (*self as u32).to_be_bytes()
    }

    pub fn from_be_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.try_into().ok()?;
        let num = u32::from_be_bytes(b);
        num.try_into().ok()
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Number of thousandths of a unit (can be negative).
    amount_milli: i64,
    currency: Currency,
}

impl Money {
    /// Creates a new Money struct for the given currency.
    ///
    /// The fractions are automatically converted based on the currency.
    pub fn new(currency: Currency, amount: i64, fracs: u32) -> Self {
        // This multiplier will be 10 for frac=2, 1 for frac=3 and 1000 for frac=0
        let mul = 10i64.pow(3u32.saturating_sub(currency.fracs() as u32));
        let amount_milli = amount.signum() * (amount.abs() * 1000 + mul * fracs as i64);
        Money {
            amount_milli,
            currency,
        }
    }

    /// Creates a new `Money` struct with currency set to `EUR` ( euro )
    pub fn euro(euros: i64, cents: u32) -> Self {
        Money::new(Currency::EUR, euros, cents)
    }

    /// Creates a new `Money` struct with currency set to `USD` ( US dollar )
    pub fn usd(dollars: i64, cents: u32) -> Self {
        Money::new(Currency::USD, dollars, cents)
    }

    /// Creates a new `Money` struct with currency set to `GBP` ( british pounds )
    pub fn pound(pounds: i64, pence: u32) -> Self {
        Money::new(Currency::GBP, pounds, pence)
    }

    /// Creates a new `Money` struct with currency set to `JPY` ( japanese yen )
    pub fn yen(yen: i64) -> Self {
        Money::new(Currency::JPY, yen, 0)
    }

    /// Creates a new `Money` struct with currency set to `CNY` ( chinese yuan )
    pub fn yuan(yuan: i64, fen: u32) -> Self {
        Money::new(Currency::CNY, yuan, fen)
    }

    /// Creates a new `Money` struct with currency set to `CHR` ( swiss franc )
    pub fn chf(francs: i64, rappen: u32) -> Self {
        Money::new(Currency::CHF, francs, rappen)
    }

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
            let remove_fracs = 3u8.saturating_sub(self.currency().fracs());
            for _ in 0..remove_fracs {
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

enum ParserState {
    Prefix,
    Number,
    NumberFrac,
    CurSym,
}

impl FromStr for Money {
    type Err = ParseMoneyError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // Okay, write a small parser from scratch
        // start with either number or "-"
        let mut state = ParserState::Prefix;
        let mut mul = 1;
        let mut num = String::new();
        let mut frac = String::new();
        let mut sym = String::new();
        for c in input.chars() {
            // Ignore whitespace completely
            if c.is_ascii_whitespace() {
                continue;
            }
            match state {
                ParserState::Prefix => {
                    if c == '-' {
                        mul = -1; // multiply by -1 to make a negative number
                        state = ParserState::Number;
                    } else if c.is_numeric() {
                        state = ParserState::Number;
                        num.push(c);
                    } else {
                        return Err(ParseMoneyError::InvalidFormat);
                    }
                }
                ParserState::Number => {
                    if c.is_numeric() {
                        num.push(c);
                    } else if c == ',' || c == '.' {
                        state = ParserState::NumberFrac;
                    } else {
                        sym.push(c);
                        state = ParserState::CurSym;
                    }
                }
                ParserState::NumberFrac => {
                    if c.is_numeric() {
                        frac.push(c);
                    } else {
                        sym.push(c);
                        state = ParserState::CurSym;
                    }
                }
                ParserState::CurSym => {
                    sym.push(c);
                }
            }
        }
        let num = i64::from_str_radix(&num, 10).map_err(|_| ParseMoneyError::InvalidFormat)?;
        let frac = if frac.is_empty() {
            0
        } else {
            // We have to account for the correct number of fractions
            let n = i64::from_str_radix(&frac, 10).map_err(|_| ParseMoneyError::InvalidFormat)?;
            if frac.len() > 3 {
                return Err(ParseMoneyError::InvalidFormat);
            }
            n * 10_i64.pow(3 - frac.len() as u32)
        };
        debug_assert!(frac < 1000);
        let amount_milli = mul * (1000 * num + frac);

        for cur in Currency::ALL {
            if sym == cur.symbol() {
                return Ok(Money {
                    amount_milli,
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
    use rand::random_range;

    use crate::{
        __private::storage_keys::{BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS},
        common::Participant,
    };

    use super::*;

    fn random_money() -> Money {
        let currency = Currency::ALL[random_range(0..Currency::ALL.len())];
        // Check fraction and shorten the amount so that it does not represent invalid values for that currency
        // E.g. cur=€ and amount_milli=1003 would not be valid, as 0.103 € is not representable.
        let mut amount_milli: i64 = random_range(-1_000_000_000..1_000_000_0);
        let fracs = 3u32.saturating_sub(currency.fracs() as u32);
        let mul = 10i64.pow(fracs);
        amount_milli /= mul; // Integer division - removes fractions
        amount_milli *= mul; // Multiply again, so e.g. (1003 / 10) * 10 = 100

        Money {
            amount_milli,
            currency,
        }
    }

    #[test]
    fn cur_u32_roundtrip() {
        for cur in Currency::ALL {
            let num = cur as u32;
            let back = Currency::try_from(num).unwrap();
            assert_eq!(cur, back);
        }
    }

    #[test]
    fn display_parse_roundtrip() -> Result<()> {
        for _ in 0..1_000 {
            let m = random_money();
            let s = m.to_string();
            let m2: Money = s.parse()?;
            assert_eq!(m, m2, "{s}");
        }
        Ok(())
    }

    #[test]
    fn parse_dot_comma() -> Result<()> {
        for _ in 0..1_000 {
            let m = random_money();
            let s1 = m.to_string();
            let s2 = s1.replace(".", ",");
            let m1: Money = s1.parse()?;
            let m2: Money = s2.parse()?;
            assert_eq!(m1, m2);
        }
        Ok(())
    }

    #[test]
    fn ignore_whitespace() -> Result<()> {
        let spaces = [
            "100 €",
            "100€",
            "  100€ ",
            " 100 €",
            "1 00, 0 0 €",
            "100.00 €",
            " 100.00 €",
            " 100 .00 €",
        ];
        for s in spaces {
            let m: Money = s.parse()?;
            assert_eq!(m.amount(), 100.0);
            assert_eq!(m.currency, Currency::EUR);
        }
        Ok(())
    }

    #[test]
    fn print_fractions() -> Result<()> {
        let m = Money::euro(10, 32);
        assert_eq!(m.to_string(), "10.32 €");
        assert_eq!("10,32€".parse::<Money>()?, m);
        let m = Money::euro(10, 0);
        assert_eq!(m.to_string(), "10 €");
        // These are all valid representations of 10€
        assert_eq!("10,00 €".parse::<Money>()?, m);
        assert_eq!("10.00 €".parse::<Money>()?, m);
        assert_eq!("10,0 €".parse::<Money>()?, m);
        assert_eq!("10.0 €".parse::<Money>()?, m);
        assert_eq!("10 €".parse::<Money>()?, m);
        assert_eq!("10€".parse::<Money>()?, m);
        Ok(())
    }

    #[test]
    fn currency_constructor_euro() {
        let euro = Money::euro(100, 10);
        assert_eq!(euro.amount_milli, 100100);
        assert_eq!(euro.to_string(), "100.10 €");
        assert_eq!(euro, Money::new(Currency::EUR, 100, 10));
        let euro = Money::euro(-100, 10);
        assert_eq!(euro.amount_milli, -100100);
        assert_eq!(euro.to_string(), "-100.10 €");
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
