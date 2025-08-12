//! ISO 4217 compliant definition of `Currency` and `Money`

use core::fmt;
use core::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// ISO 4217 currencies that account for the vast majority of global FX turnover (BIS 2022).
///
/// * Each variant name is the three letter ISO code.
/// * The discriminant is the ISO numeric code (`repr(u32)`).
/// * `Display` prints the common symbol (or symbol like shorthand).
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
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

// TODO: Let's simply use "String" as the base representation here
/// Monetary value stored as an *integer number of thousandths* (1/1000) of the currency’s major unit.
///
/// Using thousandths lets us represent all ISO 4217 currencies (the largest fraction in normal use is the Bahraini dinar’s 3 decimal places) without loss.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct Money {
    /// Amount in thousandths of a unit (can be negative).
    pub amount_milli: i64,
    pub currency: Currency,
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

    pub fn from_milli(currency: Currency, amount_milli: i64) -> Self {
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
            let remove_fracs = 3u8.saturating_sub(self.currency.fracs());
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random_range;

    // Emulate having anyhow
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// Generates a random amount in a random currency
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
}
