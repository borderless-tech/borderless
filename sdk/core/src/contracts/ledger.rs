#![allow(unused)]
use std::collections::HashMap;

use anyhow::anyhow;
use borderless_id_types::BorderlessId;
use serde::{Deserialize, Serialize};

use crate::__private::{read_field, storage_keys::BASE_KEY_MASK_LEDGER, write_field};

use super::TxCtx;

// TODO: I don't like this design.
//
// Couldn't we do something that let's us do EUR( 32_50 ) ?
// Or use parse trait ... "32,50€".parse()?
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl Account for BorderlessId {
    fn settle_debt(&self, debitor: Self) -> TransferBuilder<Self> {
        TransferBuilder {
            party_a: *self,
            party_b: debitor,
        }
    }

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

impl TransferBuilder<BorderlessId> {
    pub fn amount(self, amount: i64, currency: Currency) -> crate::Result<()> {
        Ledger::open(self.party_a, self.party_b).push(amount, currency)
    }
}

pub struct Ledger {
    party_a: BorderlessId,
    party_b: BorderlessId,
    base_key: u64,
}

const SUB_KEY_STATE: u64 = 0;
const SUB_KEY_LEN: u64 = u64::MAX;

impl Ledger {
    pub fn open(party_a: BorderlessId, party_b: BorderlessId) -> Self {
        let base_key = party_a.merge_compact(&party_b) & BASE_KEY_MASK_LEDGER;
        Ledger {
            party_a,
            party_b,
            base_key,
        }
    }

    pub fn state(&self) -> LedgerState {
        if let Some(state) = read_field(self.base_key, SUB_KEY_STATE) {
            state
        } else {
            // Return default value
            LedgerState {
                party_a: self.party_a,
                party_b: self.party_b,
                currency: Currency::EUR,
                balance: 0,
            }
        }
    }

    pub fn len(&self) -> u64 {
        read_field(self.base_key, SUB_KEY_LEN).unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // TODO: This could be its own error type tbh
    pub fn push(&mut self, amount: i64, currency: Currency) -> crate::Result<()> {
        let prev = self.state();
        if prev.currency != currency {
            return Err(anyhow!("ledger currency mismatch"));
        }

        // Record the new state
        let new_state = LedgerState {
            party_a: prev.party_a,
            party_b: prev.party_b,
            currency,
            balance: prev.balance + amount,
        };
        write_field(self.base_key, SUB_KEY_STATE, &new_state);

        // Record the transfer plus the metadata
        let transfer = LedgerStateMeta {
            transfer: LedgerState {
                party_a: prev.party_a,
                party_b: prev.party_b,
                currency,
                balance: amount,
            },
            tx_ctx: super::env::tx_ctx(),
            block_ts: super::env::block_timestamp(),
        };
        let sub_key = self.len() + 1;
        write_field(self.base_key, sub_key, &transfer);

        // Adjust len ( the sub_key for the record is the new len )
        write_field(self.base_key, SUB_KEY_LEN, &sub_key);
        Ok(())
    }

    pub fn get_transfer(&self, idx: usize) -> Option<LedgerStateMeta> {
        todo!()
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
#[derive(Serialize, Deserialize)]
pub struct LedgerState {
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

#[derive(Serialize, Deserialize)]
pub struct LedgerStateMeta {
    transfer: LedgerState,
    tx_ctx: TxCtx,
    block_ts: u64,
}
