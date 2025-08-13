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

/// Monetary value stored as an *integer number of thousandths* (1/1000) of the currency’s major unit.
///
/// Using thousandths lets us represent all ISO 4217 currencies (the largest fraction in normal use is the Bahraini dinar’s 3 decimal places) without loss.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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

/// Formats an amount with the given number of fractions.
///
/// Similar to the `fmt::Display` implementation, but without the currency symbol
#[cfg(feature = "serde")]
fn fmt_amount_with_fracs(amount_milli: i64, fracs: u8) -> String {
    let abs = amount_milli.abs();
    let integral = abs / 1000;
    let fractional = (abs % 1000) as u32;

    // Build fractional string, trimming trailing zeros.
    if fractional == 0 {
        if amount_milli < 0 {
            format!("-{}", integral)
        } else {
            format!("{}", integral)
        }
    } else {
        let mut frac_str = format!("{:03}", fractional);
        let remove_fracs = 3u8.saturating_sub(fracs);
        for _ in 0..remove_fracs {
            frac_str.pop();
        }
        if amount_milli < 0 {
            format!("-{}.{}", integral, frac_str)
        } else {
            format!("{}.{}", integral, frac_str)
        }
    }
}

/// Parses the input string into `amount_milli`, the currency symbol and the number of fracs
fn parse_amount_to_milli(input: &str) -> Result<(i64, String, usize), ParseMoneyError> {
    // normalize whitespace (incl. non-breaking) and trim
    let mut s = input.replace('\u{00A0}', " "); // NBSP → space
    s.retain(|c| c != '\t' && c != '\r' && c != '\n' && !c.is_whitespace());

    if s.is_empty() {
        return Err(ParseMoneyError::InvalidFormat);
    }

    // sign
    let (neg, rest) = if let Some(r) = s.strip_prefix('-') {
        (true, r.trim_start())
    } else {
        (false, s.as_str())
    };

    // find the rightmost '.' or ',' → decimal separator
    let last_dot = rest.rfind('.');
    let last_comma = rest.rfind(',');
    let dec_idx = match (last_dot, last_comma) {
        (Some(d), Some(c)) => Some(d.max(c)),
        (Some(d), None) => Some(d),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };

    // split number (left) and trailing currency symbol (right)
    // we’ll scan digits/sep from the left; once a non [0-9., _' ] appears, that and the rest is the symbol
    let mut num_end = rest.len();
    for (i, ch) in rest.char_indices() {
        let is_group = ch == '.' || ch == ',' || ch == ' ' || ch == '_' || ch == '’' || ch == '\'';
        if ch.is_ascii_digit() || is_group {
            continue;
        } else {
            num_end = i;
            break;
        }
    }
    let (num_part, sym_part) = rest.split_at(num_end);
    let sym = sym_part.trim().to_string();

    // split whole/fraction around the chosen decimal separator (if any)
    let (whole_raw, frac_raw) = if let Some(idx) = dec_idx {
        // only treat that occurrence as decimal if it's inside num_part
        if idx < num_end {
            (&num_part[..idx], &num_part[idx + 1..])
        } else {
            (num_part, "")
        }
    } else {
        (num_part, "")
    };

    // strip grouping from the whole part
    let mut whole_clean = String::with_capacity(whole_raw.len());
    for ch in whole_raw.chars() {
        if ch.is_ascii_digit() {
            whole_clean.push(ch);
        }
        // ignore group separators: . , space, NBSP, _, ’, '
        else if ch == '.' || ch == ',' || ch == ' ' || ch == '_' || ch == '’' || ch == '\'' {
            continue;
        } else {
            return Err(ParseMoneyError::InvalidFormat);
        }
    }
    if whole_clean.is_empty() {
        return Err(ParseMoneyError::InvalidFormat);
    }

    // take only the leading digits of the fractional part; grouping not expected after decimal
    let mut frac_digits = String::new();
    for ch in frac_raw.chars() {
        if ch.is_ascii_digit() {
            frac_digits.push(ch);
        } else if ch == ' ' {
            continue;
        }
        // allow stray space after decimal
        else {
            break;
        } // currency symbol or anything else found; stop
    }
    if frac_digits.len() > 3 {
        return Err(ParseMoneyError::InvalidFormat);
    }

    // build milli
    let whole_i: i128 = whole_clean
        .parse()
        .map_err(|_| ParseMoneyError::InvalidFormat)?;
    let frac_pad = {
        let mut f = frac_digits;
        while f.len() < 3 {
            f.push('0');
        }
        f
    };
    let frac_i: i128 = frac_pad
        .parse()
        .map_err(|_| ParseMoneyError::InvalidFormat)?;
    let mut milli = whole_i
        .checked_mul(1000)
        .and_then(|x| x.checked_add(frac_i))
        .ok_or(ParseMoneyError::Overflow)?;
    if neg {
        milli = -milli;
    }

    // return frac_len as actually provided (before padding)
    let frac_len = dec_idx
        .filter(|&i| i < num_end)
        .map(|_| frac_raw.chars().take_while(|c| c.is_ascii_digit()).count())
        .unwrap_or(0);

    Ok((milli as i64, sym, frac_len))
}

impl FromStr for Money {
    type Err = ParseMoneyError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (amount_milli, sym, frac_len) = parse_amount_to_milli(input)?;

        for cur in Currency::ALL {
            if sym == cur.symbol() {
                // Check, if the frac_len is smaller or equal to what the currency allows
                if frac_len > cur.fracs() as usize {
                    return Err(ParseMoneyError::InvalidFormat);
                }
                // Otherwise, just return the money struct
                return Ok(Money {
                    amount_milli,
                    currency: cur,
                });
            }
        }
        Err(ParseMoneyError::UnknownCurrency)
    }
}

#[cfg(feature = "serde")]
mod serialize_money {
    use super::*;
    use serde::{Deserializer, Serializer};

    /// Human readable definition of [`Money`]
    #[derive(Serialize, Deserialize)]
    struct MoneyHuman {
        amount: String,
        currency: Currency,
    }

    /// Binary format for [`Money`]
    ///
    /// NOTE: This is identical to [`Money`] itself, but has a different serialize/deserialize implementation !
    #[derive(Serialize, Deserialize)]
    struct MoneyBin {
        amount_milli: i64,
        currency: Currency,
    }

    impl Serialize for Money {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if serializer.is_human_readable() {
                let f = self.currency.fracs();
                let amount = fmt_amount_with_fracs(self.amount_milli, f);
                MoneyHuman {
                    amount,
                    currency: self.currency,
                }
                .serialize(serializer)
            } else {
                MoneyBin {
                    amount_milli: self.amount_milli,
                    currency: self.currency,
                }
                .serialize(serializer)
            }
        }
    }

    impl<'de> Deserialize<'de> for Money {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            if deserializer.is_human_readable() {
                let m = MoneyHuman::deserialize(deserializer)?;
                let fracs = m.currency.fracs();
                let (amount_milli, _, n_fracs) =
                    parse_amount_to_milli(&m.amount).map_err(serde::de::Error::custom)?;
                if n_fracs > fracs as usize {
                    return Err(serde::de::Error::custom(format!(
                        "invalid amount of fractions for currency {}",
                        m.currency
                    )));
                }
                Ok(Money {
                    amount_milli,
                    currency: m.currency,
                })
            } else {
                let m = MoneyBin::deserialize(deserializer)?;
                Ok(Money {
                    amount_milli: m.amount_milli,
                    currency: m.currency,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random_range;

    // Emulate having anyhow
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// Generates a random amount in a random currency
    pub fn random_money() -> Money {
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
    #[test]
    fn parse_accepts_dot_and_comma_and_whitespace() {
        let cases = [
            ("100€", 100_000, "€", 0),
            ("100.5 €", 100_500, "€", 1),
            (" 100 , 50 € ", 100_500, "€", 2),
            ("-1,005€", -1_005, "€", 3),
            ("10.00€", 10_000, "€", 2),
        ];
        for (inp, want_milli, want_sym, want_fracs) in cases {
            let (milli, sym, fracs) = parse_amount_to_milli(inp).unwrap();
            println!("{inp}");
            assert_eq!(
                (milli, sym, fracs),
                (want_milli, want_sym.to_string(), want_fracs)
            );
        }
    }

    #[test]
    fn parse_rejects_more_than_three_fractional_digits() {
        // Parser itself rejects >3 digits regardless of currency
        let e = parse_amount_to_milli("1.2345€").unwrap_err();
        assert_eq!(e, ParseMoneyError::InvalidFormat);
    }

    #[test]
    fn parse_handles_sign_and_large_values() {
        let (milli, sym, fracs) = parse_amount_to_milli("-123456789.001$").unwrap();
        assert_eq!(sym, "$");
        assert_eq!(fracs, 3);
        assert_eq!(milli, -123_456_789_001);
    }

    // ---- Money::from_str (uses parse + currency check) ----
    #[test]
    fn from_str_respects_currency_fracs() {
        // EUR allows 2 → OK
        let m: Money = "1.23 €".parse().unwrap();
        assert_eq!(m.amount_milli, 1_230);

        // EUR with 3 → error (even though parse_amount_to_milli would accept 3)
        let e = "1.234€".parse::<Money>().unwrap_err();
        assert_eq!(e, ParseMoneyError::InvalidFormat);

        // JPY with decimals → error
        let e = "1.0 ¥".parse::<Money>().unwrap_err();
        assert_eq!(e, ParseMoneyError::InvalidFormat);
    }

    #[test]
    fn from_str_unknown_currency() {
        let e = "10.00 ¤".parse::<Money>().unwrap_err();
        assert_eq!(e, ParseMoneyError::UnknownCurrency);
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::tests::random_money;
    use super::*;
    use serde_json as json;

    // Emulate having anyhow
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    #[test]
    fn json_serialize_eur() -> Result<()> {
        let m = Money::euro(12, 34); // €12.34 → 12_340 milli
        let s = json::to_string(&m)?;
        // Human readable: amount should be a STRING with 2 fracs for EUR
        assert_eq!(s, r#"{"amount":"12.34","currency":"EUR"}"#);
        Ok(())
    }

    #[test]
    fn json_serialize_jpy() -> Result<()> {
        let m = Money::yen(123); // JPY has 0 fractional digits
        let s = json::to_string(&m)?;
        assert_eq!(s, r#"{"amount":"123","currency":"JPY"}"#);
        Ok(())
    }

    #[test]
    fn json_roundtrip_eur() -> Result<()> {
        for _ in 0..1000 {
            let m = random_money();
            let s = json::to_string(&m)?;
            let back: Money = json::from_str(&s)?;
            assert_eq!(m, back);
        }
        Ok(())
    }

    #[test]
    fn json_deserialize_accepts_comma_and_dot() -> Result<()> {
        // Your parser accepts both '.' and ',' — serde path goes through parse_amount_to_milli
        let j1 = r#"{ "amount": "10.5", "currency": "EUR" }"#;
        let j2 = r#"{ "amount": "10,5", "currency": "EUR" }"#;
        let m1: Money = json::from_str(j1)?;
        let m2: Money = json::from_str(j2)?;
        assert_eq!(m1, m2);
        assert_eq!(m1.amount_milli, 10_500);
        Ok(())
    }

    #[test]
    fn json_deserialize_rejects_too_many_fracs_for_currency() {
        // EUR has 2 fracs → "1.234" should be rejected
        let j = r#"{ "amount": "1.234", "currency": "EUR" }"#;
        let res = json::from_str::<Money>(j);
        assert!(res.is_err());
        let err = res.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid amount of fractions") || msg.contains("invalid money format"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn json_deserialize_rejects_numeric_amount() {
        // MoneyHuman.amount is String; numbers should be rejected
        let j = r#"{ "amount": 12.34, "currency": "EUR" }"#;
        let err = json::from_str::<Money>(j).unwrap_err();
        assert!(err.to_string().contains("invalid type") || err.to_string().contains("string"));
    }

    #[test]
    fn postcard_roundtrip_usd() -> Result<()> {
        // Binary path must keep amount_milli intact and not format strings
        let m = Money::usd(5, 1); // $5.01 → 5_010 milli
        let bytes = postcard::to_allocvec(&m)?;
        let back: Money = postcard::from_bytes(&bytes)?;
        assert_eq!(m, back);
        assert_eq!(back.amount_milli, 5_010);
        Ok(())
    }

    #[test]
    fn postcard_roundtrip() -> Result<()> {
        for _ in 0..1000 {
            let m = random_money();
            let bytes = postcard::to_allocvec(&m)?;
            let back: Money = postcard::from_bytes(&bytes)?;
            assert_eq!(m, back);
        }
        Ok(())
    }

    #[test]
    fn json_and_binary_are_different_shapes() -> Result<()> {
        let m = Money::euro(1, 0);
        // JSON should contain "amount" as a string
        let js = json::to_string(&m)?;
        assert!(js.contains(r#""amount":"1""#));

        // Binary length should be small (no string) — this is a smoke test:
        // Currency is a repr(u32) in the code, but serialized here via derive;
        // still, the binary should *not* include "1" or any dots/commas.
        let bin = postcard::to_allocvec(&m)?;
        assert!(!std::str::from_utf8(&bin).unwrap_or("").contains("1"));
        Ok(())
    }

    #[test]
    fn fmt_amount_matches_currency_fracs() {
        // EUR → 2 fracs
        let s = fmt_amount_with_fracs(12_340, Currency::EUR.fracs());
        assert_eq!(s, "12.34");

        // JPY → 0 fracs
        let s = fmt_amount_with_fracs(12_000, Currency::JPY.fracs());
        assert_eq!(s, "12");

        // KRW (0 fracs) negative
        let s = fmt_amount_with_fracs(-987_000, Currency::KRW.fracs());
        assert_eq!(s, "-987");

        // Trailing zeros trimmed correctly at 2 fracs
        let s = fmt_amount_with_fracs(10_000, Currency::EUR.fracs());
        assert_eq!(s, "10");
    }
}
