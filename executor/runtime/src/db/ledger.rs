//! Ledger related functionality
//!
//! A ledger between two parties is persisted in the kv-store and can be modified through the ledger-api.

use std::array::TryFromSliceError;

use ahash::{HashMap, HashMapExt};
use borderless::{
    contracts::ledger::{Currency, LedgerEntry, Money},
    prelude::{ledger::EntryType, TxCtx},
    BorderlessId, ContractId,
};
use borderless_kv_store::{Db, RawRead, RawWrite, RoCursor as _, RoTx};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, LEDGER_SUB_DB};

/// Sub-Key where the meta-information of the ledger is stored
pub const SUB_KEY_LEDGER_META: u64 = u64::MAX;

pub struct Ledger<'a, S: Db> {
    db: &'a S,
}

impl<'a, S: Db> Ledger<'a, S> {
    pub fn new(db: &'a S) -> Self {
        Self { db }
    }

    pub(crate) fn commit_entry(
        &self,
        txn: &mut <S as Db>::RwTx<'_>,
        entry: &LedgerEntry,
        cid: ContractId,
        tx_ctx: &TxCtx,
    ) -> Result<()> {
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        // Read current ledger meta information
        let meta_key = LedgerKey::meta(&entry.creditor, &entry.debitor);
        let meta = match txn.read(&db_ptr, &meta_key)? {
            Some(val) => postcard::from_bytes(&val)?,
            None => LedgerMeta::new(entry.creditor, entry.debitor),
        };
        // update meta information based on the current entry
        let meta = meta.update(entry)?;

        // Write ledger line
        let c_key = LedgerKey::new(entry.creditor, entry.debitor, cid, meta.len, "creditor");
        let d_key = LedgerKey::new(entry.creditor, entry.debitor, cid, meta.len, "debitor");
        let amount_key = LedgerKey::new(entry.creditor, entry.debitor, cid, meta.len, "amount");
        let tax_key = LedgerKey::new(entry.creditor, entry.debitor, cid, meta.len, "tax");
        let currency_key = LedgerKey::new(entry.creditor, entry.debitor, cid, meta.len, "currency");
        let tag_key = LedgerKey::new(entry.creditor, entry.debitor, cid, meta.len, "tag");
        let tx_ctx_key = LedgerKey::new(entry.creditor, entry.debitor, cid, meta.len, "tx_ctx");
        let tx_ctx_bytes = tx_ctx.to_bytes()?;
        txn.write(&db_ptr, &c_key, entry.creditor.as_bytes())?;
        txn.write(&db_ptr, &d_key, entry.debitor.as_bytes())?;
        txn.write(&db_ptr, &amount_key, &entry.amount_milli.to_be_bytes())?;
        txn.write(&db_ptr, &tax_key, &entry.tax_milli.to_be_bytes())?;
        txn.write(&db_ptr, &currency_key, &entry.currency.to_be_bytes())?;
        txn.write(&db_ptr, &tag_key, &entry.tag.as_bytes())?;
        txn.write(&db_ptr, &tx_ctx_key, &tx_ctx_bytes)?;

        // Write meta back
        let meta_bytes = postcard::to_allocvec(&meta)?;
        txn.write(&db_ptr, &meta_key, &meta_bytes)?;
        Ok(())
    }

    /// Opens a specific ledger
    pub fn open(&self, p1: BorderlessId, p2: BorderlessId) -> SelectedLedger<'a, S> {
        SelectedLedger {
            db: self.db,
            p1,
            p2,
        }
    }

    /// Returns a list of all existing ledgers
    pub fn all(&self) -> Result<Vec<LedgerMeta>> {
        let mut out = Vec::new();
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;
        let mask_meta = LedgerKey::mask_meta();
        for (_key, value) in cursor.iter().filter(|(key, _)| {
            // Bit-level-hacking: We try to only match the meta entry
            for (b1, b2) in key.iter().zip(mask_meta.iter()) {
                if b1 | b2 != 1 {
                    return false;
                }
            }
            true
        }) {
            let ledger_meta = postcard::from_bytes(value)?;
            out.push(ledger_meta);
        }
        Ok(out)
    }

    /// Returns a list of balances for all existing ledgers
    pub fn all_balances(&self) -> Result<Vec<Balances>> {
        Ok(self.all()?.into_iter().map(|m| m.to_balances()).collect())
    }
}

pub struct SelectedLedger<'a, S: Db> {
    db: &'a S,
    p1: BorderlessId,
    p2: BorderlessId,
}

impl<'a, S: Db> SelectedLedger<'a, S> {
    /// Returns the length of the ledger
    pub fn meta(&self) -> Result<LedgerMeta> {
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let key = LedgerKey::meta(&self.p1, &self.p2);
        let txn = self.db.begin_ro_txn()?;
        match txn.read(&db_ptr, &key)? {
            Some(val) => {
                let out = postcard::from_bytes(&val)?;
                Ok(out)
            }
            None => Ok(LedgerMeta::new(self.p1, self.p2)),
        }
    }

    /// Returns the balances of the ledger
    pub fn balances(&self) -> Result<Balances> {
        self.meta().map(|m| m.to_balances())
    }
}

/// Meta information about this ledger
#[derive(Serialize, Deserialize)]
pub struct LedgerMeta {
    /// Creditor side
    pub creditor: BorderlessId,
    /// Debitor side
    pub debitor: BorderlessId,
    /// Length of the ledger
    pub len: u64,
    /// Balances by currency ( values are in 1000 units )
    pub balances: HashMap<Currency, i64>,
}

impl LedgerMeta {
    pub fn new(creditor: BorderlessId, debitor: BorderlessId) -> Self {
        LedgerMeta {
            creditor,
            debitor,
            len: 0,
            balances: HashMap::new(),
        }
    }

    /// Updates the ledger meta information with the current entry
    ///
    /// Returns an error, if the ledger-entry does not belong to this ledger.
    pub fn update(mut self, entry: &LedgerEntry) -> Result<Self> {
        let balance = self.balances.entry(entry.currency).or_default();

        // We have to modify the balance, based on the 'direction' of the transfer
        let mul = if (entry.creditor, entry.debitor) == (self.creditor, self.debitor) {
            /* same direction */
            1
        } else if (entry.creditor, entry.debitor) == (self.debitor, self.creditor) {
            /* inverse direction */
            -1
        } else {
            return Err(Error::msg("ledger-entry does not match ledger owners"));
        };
        match entry.kind {
            EntryType::CREATE => {
                *balance += mul * entry.amount_milli;
            }
            EntryType::SETTLE | EntryType::CANCEL => {
                *balance -= mul * entry.amount_milli;
            }
        }
        self.len += 1;
        Ok(self)
    }

    pub fn to_balances(&self) -> Balances {
        let mut balances = HashMap::with_capacity(self.balances.len());
        for (key, value) in self.balances.iter() {
            balances.insert(
                key.symbol().to_string(),
                Money::new(*key, *value).to_string(),
            );
        }
        Balances {
            creditor: self.creditor,
            debitor: self.debitor,
            balances,
        }
    }
}

/// Json-Friendly balances information
#[derive(Serialize)]
pub struct Balances {
    pub creditor: BorderlessId,
    pub debitor: BorderlessId,
    pub balances: HashMap<String, String>,
}

// TODO: Maybe it's better to make this the DB representation;
// while we keep the concept of using json to communicate between host and guest
// -> we could use a LedgerKey system to encode these values:
// [ participant-pair | contract-id | number | value ]
// [     u64          |      u64    |  u64   | u64   ]
// If we use the uuid-compaction, we can store the participant-pair as u64, aswell as the contract-id.
// "Number" would then be the number of the ledger entry, and "value" would be
// an encoded field of the ledger-entry (e.g. by using a hash-func on the field name like "amount_milli").
//
// This way we could efficiently scroll through all ledger entries and only read specific values from it.
// Number = u64 could be used to store meta-info (for a single contract),
// and contract-id = u64::MAX could be used to store meta infos over all contracts
struct LedgerKey([u8; 32]);

impl LedgerKey {
    /// Constructs a new ledger-key for a specific line and column of a participant-tuple and contract-id.
    pub fn new(
        participant_a: BorderlessId,
        participant_b: BorderlessId,
        contract: ContractId,
        line: u64,
        column: &'static str,
    ) -> Self {
        let participant_key = participant_a.merge_compact(&participant_b).to_be_bytes();
        let contract_key = contract.compact().to_be_bytes();
        let line_key = line.to_be_bytes();
        let column_key = xxhash_rust::const_xxh3::xxh3_64(column.as_bytes()).to_be_bytes();
        let mut key = [0; 32];
        key[0..8].copy_from_slice(&participant_key);
        key[8..16].copy_from_slice(&contract_key);
        key[16..24].copy_from_slice(&line_key);
        key[24..32].copy_from_slice(&column_key);
        LedgerKey(key)
    }

    /// Returns the key, where the length of the ledger is saved
    pub fn meta(participant_a: &BorderlessId, participant_b: &BorderlessId) -> Self {
        let participant_key = participant_a.merge_compact(participant_b).to_be_bytes();
        // We want the meta-info to be the absolute last key, so we do some bit-hacking here:
        let mut key = [0xff; 32];
        key[0..8].copy_from_slice(&participant_key);
        LedgerKey(key)
    }

    /// Generates a bit-mask that matches for all meta-keys: `mask_meta | key == 0xff`
    pub fn mask_meta() -> [u8; 32] {
        let mut mask = [0x00; 32];
        mask[0..8].copy_from_slice(&[0xff; 8]);
        mask
    }

    /// Generates a bit-mask that matches all keys where `len == 0`
    pub fn mask_first_entry() -> [u8; 32] {
        let mut mask = [0x00; 32];
        mask[16..24].copy_from_slice(&[0xff; 8]);
        mask
    }

    /// Returns the raw 32-byte representation of the storage key.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Converts the storage key to a hexadecimal string (for logging/debugging).
    pub fn to_hex(&self) -> String {
        use std::fmt::Write;
        self.0.iter().fold(String::new(), |mut output, b| {
            let _ = write!(output, "{b:02x}");
            output
        })
    }

    /// Returns the line of the ledger entry
    ///
    /// The line is the only thing we can really reconstruct from the LedgerKey.
    pub fn get_line(&self) -> u64 {
        let mut out = [0; 8];
        out.copy_from_slice(&self.0[16..24]);
        u64::from_be_bytes(out)
    }
}

impl TryFrom<&[u8]> for LedgerKey {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        let buf = value.try_into()?;
        Ok(LedgerKey(buf))
    }
}

impl AsRef<[u8]> for LedgerKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_is_last_key() {
        let p1 = BorderlessId::generate();
        let p2 = BorderlessId::generate();
        let len_key = LedgerKey::meta(&p1, &p2);
        assert_eq!(len_key.get_line(), SUB_KEY_LEDGER_META);
    }
}
