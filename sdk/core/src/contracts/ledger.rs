#![allow(unused)]
use std::collections::HashMap;

use borderless_id_types::BorderlessId;

// TODO: I don't like this design.
//
// Couldn't we do something that let's us do EUR( 32_50 ) ?
// Or use parse trait ... "32,50€".parse()?
pub enum Currency {
    /// Albanian Lek
    ALL = 8,
    /// Azerbaijani Manat
    AZN = 944,
    /// Belarusian Ruble
    BYN = 933,
    /// Bosnian Convertible Mark
    BAM = 977,
    /// British Pound Sterling
    GBP = 826,
    /// Bulgarian Lev
    BGN = 975,
    /// Czech Koruna
    CZK = 203,
    /// Danish Krone
    DKK = 208,
    /// Euro
    EUR = 978,
    /// Hungarian Forint
    HUF = 348,
    /// Icelandic Krona
    ISK = 352,
    /// Norwegian Krone
    NOK = 578,
    /// Polish Zloty
    PLN = 985,
    /// Romanian Leu
    RON = 946,
    /// Russian Ruble
    RUB = 643,
    /// Serbian Dinar
    RSD = 941,
    /// Swiss Franc
    CHF = 756,
    /// Turkish Lira
    TRY = 949,
    /// Ukrainian Hryvnia
    UAH = 980,
    /// US Dollar
    USD = 840,
}

impl Currency {
    pub fn currency_name(&self) -> &'static str {
        match self {
            Currency::ALL => "Albanian Lek",
            Currency::AZN => "Azerbaijani Manat",
            Currency::BYN => "Belarusian Ruble",
            Currency::BAM => "Bosnian Convertible Mark",
            Currency::GBP => "British Pound Sterling",
            Currency::BGN => "Bulgarian Lev",
            Currency::CZK => "Czech Koruna",
            Currency::DKK => "Danish Krone",
            Currency::EUR => "Euro",
            Currency::HUF => "Hungarian Forint",
            Currency::ISK => "Icelandic Krona",
            Currency::NOK => "Norwegian Krone",
            Currency::PLN => "Polish Zloty",
            Currency::RON => "Romanian Leu",
            Currency::RUB => "Russian Ruble",
            Currency::RSD => "Serbian Dinar",
            Currency::CHF => "Swiss Franc",
            Currency::TRY => "Turkish Lira",
            Currency::UAH => "Ukrainian Hryvnia",
            Currency::USD => "US Dollar",
        }
    }

    pub const fn symbol(&self) -> &'static str {
        match self {
            Currency::ALL => "Lek",
            Currency::AZN => "₼",
            Currency::BYN => "Br",
            Currency::BAM => "KM",
            Currency::GBP => "£",
            Currency::BGN => "лв",
            Currency::CZK => "Kč",
            Currency::DKK => "kr",
            Currency::EUR => "€",
            Currency::HUF => "Ft",
            Currency::ISK => "kr",
            Currency::NOK => "kr",
            Currency::PLN => "zł",
            Currency::RON => "lei",
            Currency::RUB => "₽",
            Currency::RSD => "Дин.",
            Currency::CHF => "CHF",
            Currency::TRY => "₺",
            Currency::UAH => "₴",
            Currency::USD => "$",
        }
    }
}

pub trait Account: Copy {
    /// Settles the debt of some debitor
    fn settle_debt(&self, debitor: Self) -> TransferBuilder<Self> {
        TransferBuilder {
            party_a: *self,
            party_b: debitor,
        }
    }

    /// Transfer a fixed amount of some currency to someone else
    fn transfer(&self, other: Self) -> TransferBuilder<Self> {
        TransferBuilder {
            party_a: other,
            party_b: *self,
        }
    }
}
pub struct TransferBuilder<A: Account> {
    party_a: A,
    party_b: A,
}

impl<A: Account> TransferBuilder<A> {
    pub fn amount(self, amount: u64, currency: Currency) {
        todo!("create transfer between the two parties")
    }
}

// TODO: We could use a structure similar to a lazyvec here;
// Basically a list, that remembers a state
// (e.g. each time a transfer is happening, the state is updated, but we nonetheless keep the list).
// this would allow us to keep track of all transfers, while accessing the last item in an instant

// NOTE: This is not only a ledger, but also the datamodel of a single transfer :D
//
// -> Let's create a ledger structure as described above
/// Simple ledger between two parties
pub struct Ledger {
    /// First party of the ledger
    party_a: BorderlessId,

    /// Second party of the ledger
    party_b: BorderlessId,

    /// Currency, that the ledger is in
    currency: Currency,

    /// Actual balance between `a` and `b`, specified in the smallest unit of the currency
    ///
    /// E.g. for `Currency::EUR` the smalles unit is a `cent`, therefore an amount of `3249` would
    /// mean `32,49€`.
    /// Note: Since this is a balance, it means that `a` has a credit of `32,49€` while `b` has the same amount as debit.
    balance: i64,
}

/// List of ledgers in a smart-contract
///
/// There can be one ledger for each pair of participants in the contract.
pub struct Ledgers {
    // NOTE: The idea is to "merge" the two ids into one thing, so the ledger can be accessed
    // by the tuple of borderless-ids (the merge operation is commutative)
    inner: HashMap<[u8; 16], Ledger>,
}
