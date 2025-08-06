//! Ledger related functionality
//!
//! A ledger between two parties is persisted in the kv-store and can be modified through the ledger-api.

use std::array::TryFromSliceError;

use ahash::{HashMap, HashMapExt};
use borderless::{
    contracts::ledger::{Currency, LedgerEntry, Money},
    http::{queries::Pagination, PaginatedElements},
    prelude::{ledger::EntryType, TxCtx},
    BorderlessId, Context, ContractId,
};
use borderless_kv_store::{Db, RawRead, RawWrite, RoCursor as _, RoTx};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, LEDGER_SUB_DB};

use crate::log_shim::debug;

/// Ledger controller of the database
pub struct Ledger<'a, S: Db> {
    db: &'a S,
}

impl<'a, S: Db> Ledger<'a, S> {
    pub fn new(db: &'a S) -> Self {
        Self { db }
    }

    /// Commits a new ledger-entry in the given transaction
    pub(crate) fn commit_entry(
        &self,
        txn: &mut <S as Db>::RwTx<'_>,
        entry: &LedgerEntry,
        cid: ContractId,
        tx_ctx: &TxCtx,
    ) -> Result<()> {
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        // Read current ledger meta information
        let ledger_id = entry.creditor.merge_compact(&entry.debitor);
        let meta_key = LedgerKey::meta(ledger_id);
        let mut meta = match txn.read(&db_ptr, &meta_key)? {
            Some(val) => postcard::from_bytes(&val)?,
            None => LedgerMeta::new(entry.creditor, entry.debitor),
        };

        // Write ledger line
        let c_key = LedgerKey::new(ledger_id, meta.len, "creditor");
        let d_key = LedgerKey::new(ledger_id, meta.len, "debitor");
        let amount_key = LedgerKey::new(ledger_id, meta.len, "amount");
        let tax_key = LedgerKey::new(ledger_id, meta.len, "tax");
        let currency_key = LedgerKey::new(ledger_id, meta.len, "currency");
        let kind_key = LedgerKey::new(ledger_id, meta.len, "kind");
        let tag_key = LedgerKey::new(ledger_id, meta.len, "tag");
        let cid_key = LedgerKey::new(ledger_id, meta.len, "contract_id");
        let tx_ctx_key = LedgerKey::new(ledger_id, meta.len, "tx_ctx");
        let tx_ctx_bytes = postcard::to_allocvec(&tx_ctx)?;
        txn.write(&db_ptr, &c_key, entry.creditor.as_bytes())?;
        txn.write(&db_ptr, &d_key, entry.debitor.as_bytes())?;
        txn.write(&db_ptr, &amount_key, &entry.amount_milli.to_be_bytes())?;
        txn.write(&db_ptr, &tax_key, &entry.tax_milli.to_be_bytes())?;
        txn.write(&db_ptr, &currency_key, &entry.currency.to_be_bytes())?;
        txn.write(&db_ptr, &kind_key, &entry.kind.to_be_bytes())?;
        txn.write(&db_ptr, &tag_key, &entry.tag.as_bytes())?;
        txn.write(&db_ptr, &cid_key, &cid.as_bytes())?;
        txn.write(&db_ptr, &tx_ctx_key, &tx_ctx_bytes)?;

        // update meta information based on the current entry
        meta.update(entry)?;

        // Write meta back
        let meta_bytes = postcard::to_allocvec(&meta)?;
        txn.write(&db_ptr, &meta_key, &meta_bytes)?;
        debug!(
            "commited ledger entry: {entry}, ledger-id={ledger_id}, len={}",
            meta.len
        );
        Ok(())
    }

    /// Opens a ledger for a pair of borderless-ids
    pub fn open(&self, p1: BorderlessId, p2: BorderlessId) -> SelectedLedger<'a, S> {
        let ledger_id = p1.merge_compact(&p2);
        SelectedLedger {
            db: self.db,
            ledger_id,
        }
    }

    /// Selects a specific ledger based on its ID
    pub fn select(&self, ledger_id: u64) -> SelectedLedger<'a, S> {
        SelectedLedger {
            db: self.db,
            ledger_id,
        }
    }

    /// Returns a list of all existing ledgers
    pub fn all(&self) -> Result<Vec<LedgerMeta>> {
        let mut out = Vec::new();
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        // NOTE: We always iterate over the entire key-space.
        // As this is all super low level, it is quite efficient,
        // but on paper it does not scale very well.
        // In the far or near future we have to migrate this to something different.
        for (key, value) in cursor.iter() {
            let key = LedgerKey::from_slice(key);
            if key.is_meta() {
                let ledger_meta = postcard::from_bytes(value)?;
                out.push(ledger_meta);
            }
        }
        Ok(out)
    }

    /// Returns a list of all existing ledgers
    pub fn all_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PaginatedElements<LedgerMetaDto>> {
        let mut elements = Vec::new();
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let range = pagination.to_range();
        let mut idx = 0;

        // NOTE: We always iterate over the entire key-space.
        // As this is all super low level, it is quite efficient,
        // but on paper it does not scale very well.
        // In the far or near future we have to migrate this to something different.
        for (key, value) in cursor.iter() {
            let key = LedgerKey::from_slice(key);
            if !key.is_meta() {
                continue;
            }
            if range.start <= idx && idx < range.end {
                let ledger_meta: LedgerMeta = postcard::from_bytes(value)?;
                elements.push(ledger_meta.into_dto());
            }
            idx += 1;
        }
        let paginated = PaginatedElements {
            elements,
            total_elements: idx,
            pagination,
        };
        Ok(paginated)
    }

    pub fn all_ids(&self) -> Result<Vec<LedgerIds>> {
        let mut out = Vec::new();
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;
        let mut last_ledger_id = 0;
        for (key, _) in cursor.iter() {
            let key = LedgerKey::from_slice(key);
            let ledger_id = key.ledger_id();
            if ledger_id == last_ledger_id {
                continue;
            }
            last_ledger_id = ledger_id;
            // Read creditor and debitor
            let elem = self.get_ledger_id(&txn, &db_ptr, ledger_id, key.line())?;
            out.push(elem);
        }
        Ok(out)
    }

    pub fn all_ids_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PaginatedElements<LedgerIds>> {
        let mut elements = Vec::new();
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;
        let mut last_ledger_id = 0;

        let range = pagination.to_range();
        let mut idx = 0;

        // NOTE: We always iterate over the entire key-space.
        // As this is all super low level, it is quite efficient,
        // but on paper it does not scale very well.
        // In the far or near future we have to migrate this to something different.
        for (key, _) in cursor.iter() {
            let key = LedgerKey::from_slice(key);
            let ledger_id = key.ledger_id();
            if ledger_id == last_ledger_id {
                continue;
            }
            last_ledger_id = ledger_id;

            if range.start <= idx && idx < range.end {
                // Read creditor and debitor
                let elem = self.get_ledger_id(&txn, &db_ptr, ledger_id, key.line())?;
                elements.push(elem);
            }
            idx += 1;
        }
        let paginated = PaginatedElements {
            elements,
            total_elements: idx,
            pagination,
        };
        Ok(paginated)
    }

    /// Helper function to obtain the `LedgerIds` from a single line.
    /// If the line does not exists, it returns an error.
    fn get_ledger_id(
        &self,
        txn: &<S as Db>::RoTx<'_>,
        db_ptr: &<S as Db>::Handle,
        ledger_id: u64,
        line: u64,
    ) -> Result<LedgerIds> {
        // Read creditor and debitor
        let c_key = LedgerKey::new(ledger_id, line, "creditor");
        let d_key = LedgerKey::new(ledger_id, line, "debitor");
        let creditor = txn
            .read(db_ptr, &c_key)?
            .and_then(|b| BorderlessId::from_slice(b).ok())
            .context("missing creditor")?;
        let debitor = txn
            .read(db_ptr, &d_key)?
            .and_then(|b| BorderlessId::from_slice(b).ok())
            .context("missing debitor")?;
        Ok(LedgerIds {
            creditor,
            debitor,
            ledger_id,
        })
    }
}

/// Represents a selected ledger
pub struct SelectedLedger<'a, S: Db> {
    db: &'a S,
    ledger_id: u64,
}

impl<'a, S: Db> SelectedLedger<'a, S> {
    /// Returns the length of the ledger (if it exists)
    pub fn meta(&self) -> Result<Option<LedgerMeta>> {
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let key = LedgerKey::meta(self.ledger_id);
        let txn = self.db.begin_ro_txn()?;
        match txn.read(&db_ptr, &key)? {
            Some(val) => {
                let out = postcard::from_bytes(&val)?;
                Ok(Some(out))
            }
            None => Ok(None),
        }
    }

    /// Returns the length of the ledger (if it exists)
    pub fn meta_for_contract(&self, cid: ContractId) -> Result<Option<LedgerMetaDto>> {
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;

        // Read ledger meta
        let key = LedgerKey::meta(self.ledger_id);
        let mut meta: LedgerMeta = match txn
            .read(&db_ptr, &key)?
            .and_then(|b| postcard::from_bytes(b).ok())
        {
            Some(m) => m,
            None => return Ok(None),
        };
        // Reset it, to only keep the creditor -> debitor info
        meta.reset_balance();

        let mut line = 0;
        loop {
            // If there are no more lines to read, then break
            match self.check_line(&txn, &db_ptr, line as u64, cid)? {
                Some(true) => { /* execute the logic below */ }
                Some(false) => {
                    line += 1;
                    continue;
                }
                None => break,
            }
            let (entry, entry_cid, _tx_ctx) = self
                .get(&txn, &db_ptr, line as u64)?
                .context("line must exist")?;
            debug_assert_eq!(entry_cid, cid);
            // Update the ledger-meta based on the new entry
            meta.update(&entry)?;
            line += 1;
        }
        let mut dto = meta.into_dto();
        dto.contract_id = Some(cid);
        Ok(Some(dto))
    }

    /// Returns a paginated list of ledger entries (for all contracts)
    pub fn get_entries_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<PaginatedElements<LedgerEntryDto>> {
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;

        // Read length via meta
        let meta_key = LedgerKey::meta(self.ledger_id);
        let total_elements = match txn
            .read(&db_ptr, &meta_key)?
            .and_then(|b| postcard::from_bytes::<LedgerMeta>(b).ok())
        {
            Some(meta) => meta.len as usize,
            None => return Ok(PaginatedElements::empty(pagination)),
        };

        let mut elements = Vec::new();
        if !pagination.reverse {
            for idx in pagination.to_range() {
                match self.get(&txn, &db_ptr, idx as u64)? {
                    Some((entry, cid, tx_ctx)) => {
                        elements.push(LedgerEntryDto::new(entry, cid, tx_ctx));
                    }
                    None => break,
                }
            }
        } else {
            let range = pagination.to_range();
            let mut idx = total_elements.saturating_sub(range.start);
            while idx > 0 {
                // NOTE: We start with idx == total_elements if range.start == 0;
                // So we decrease in advance. Otherwise the idx > 0 would result in us leaving out the last element.
                idx -= 1;
                let (entry, cid, tx_ctx) = self
                    .get(&txn, &db_ptr, idx as u64)?
                    .context("entry idx < max_len must exist")?;
                elements.push(LedgerEntryDto::new(entry, cid, tx_ctx));
                if range.end - range.start <= elements.len() {
                    break;
                }
            }
        }
        Ok(PaginatedElements {
            elements,
            total_elements,
            pagination,
        })
    }

    pub fn get_contract_paginated(
        &self,
        cid: ContractId,
        pagination: Pagination,
    ) -> Result<PaginatedElements<LedgerEntryDto>> {
        let db_ptr = self.db.open_sub_db(LEDGER_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;

        let mut elements = Vec::new();
        let range = pagination.to_range();
        let mut idx = 0;

        if !pagination.reverse {
            // Go forward and ignore all entries, where the contract-id does not match
            let mut line = 0;
            loop {
                // If there are no more lines to read, then break
                match self.check_line(&txn, &db_ptr, line as u64, cid)? {
                    Some(true) => { /* execute the logic below */ }
                    Some(false) => {
                        line += 1;
                        continue;
                    }
                    None => break,
                }
                // Take as many items as fit in the page
                if range.start <= idx && idx < range.end {
                    let (entry, cid, tx_ctx) = self
                        .get(&txn, &db_ptr, line as u64)?
                        .context("line must exist")?;
                    elements.push(LedgerEntryDto::new(entry, cid, tx_ctx));
                }
                // Keep counting, since we don't know the 'total_elements' in advance
                idx += 1;
                line += 1;
            }
        } else {
            // Go backward and ignore all entries, where the contract-id does not match
            let meta_key = LedgerKey::meta(self.ledger_id);
            let all_entries = match txn
                .read(&db_ptr, &meta_key)?
                .and_then(|b| postcard::from_bytes::<LedgerMeta>(b).ok())
            {
                Some(meta) => meta.len as usize,
                None => return Ok(PaginatedElements::empty(pagination)),
            };

            let mut line = all_entries;
            while line > 0 {
                // NOTE: We start with line == total_elements
                // So we decrease in advance. Otherwise the line > 0 would result in us leaving out the last element.
                line -= 1;
                // if it returns 'false' we want to continue, as this is a line not matching to our contract-id
                if !self
                    .check_line(&txn, &db_ptr, line as u64, cid)?
                    .unwrap_or_default()
                {
                    continue;
                }
                if range.start <= idx && idx < range.end {
                    let (entry, cid, tx_ctx) = self
                        .get(&txn, &db_ptr, line as u64)?
                        .context("line must exist")?;
                    elements.push(LedgerEntryDto::new(entry, cid, tx_ctx));
                }
                idx += 1;
            }
        }
        Ok(PaginatedElements {
            elements,
            total_elements: idx,
            pagination,
        })
    }

    /// Reads a single column in an existing db-txn
    fn read_column<T>(
        &self,
        txn: &<S as Db>::RoTx<'_>,
        db_ptr: &<S as Db>::Handle,
        line: u64,
        column: &'static str,
        transformer: impl Fn(&[u8]) -> Option<T>,
    ) -> Result<Option<T>> {
        let key = LedgerKey::new(self.ledger_id, line, column);
        let out = txn.read(db_ptr, &key)?.and_then(transformer);
        Ok(out)
    }

    /// Checks if a ledger entry exists and matches a specific contract-id
    ///
    /// Returns `Some(true)` if the line matches the contract-id and `Some(false)` if not.
    /// If no line is found for the given index, `None` is returned.
    fn check_line(
        &self,
        txn: &<S as Db>::RoTx<'_>,
        db_ptr: &<S as Db>::Handle,
        line: u64,
        target_cid: ContractId,
    ) -> Result<Option<bool>> {
        let contract_id = match self.read_column(txn, db_ptr, line, "contract_id", |b| {
            ContractId::from_slice(b).ok()
        })? {
            Some(c) => c,
            None => {
                // Early return here - if this value exists, all the others must exist too
                return Ok(None);
            }
        };
        // If we only want to match a single contract-id, we can do it like this:
        Ok(Some(target_cid == contract_id))
    }

    /// Reads a line from the ledger
    fn get(
        &self,
        txn: &<S as Db>::RoTx<'_>,
        db_ptr: &<S as Db>::Handle,
        line: u64,
    ) -> Result<Option<(LedgerEntry, ContractId, TxCtx)>> {
        let contract_id = match self.read_column(txn, db_ptr, line, "contract_id", |b| {
            ContractId::from_slice(b).ok()
        })? {
            Some(c) => c,
            None => {
                // Early return here - if this value exists, all the others must exist too
                return Ok(None);
            }
        };
        // Read all other values
        let creditor = self
            .read_column(txn, db_ptr, line, "creditor", |b| {
                BorderlessId::from_slice(b).ok()
            })?
            .context("missing creditor")?;
        let debitor = self
            .read_column(txn, db_ptr, line, "debitor", |b| {
                BorderlessId::from_slice(b).ok()
            })?
            .context("missing debitor")?;
        let amount_milli = self
            .read_column(txn, db_ptr, line, "amount", i64_from_slice)?
            .context("missing field amount")?;
        let tax_milli = self
            .read_column(txn, db_ptr, line, "tax", i64_from_slice)?
            .context("missing field tax")?;
        let currency = self
            .read_column(txn, db_ptr, line, "currency", Currency::from_be_bytes)?
            .context("missing field currency")?;
        let kind = self
            .read_column(txn, db_ptr, line, "kind", EntryType::from_be_bytes)?
            .context("missing field tag")?;
        let tag = self
            .read_column(txn, db_ptr, line, "tag", |b| {
                Some(String::from_utf8_lossy(b).to_string())
            })?
            .context("missing field tag")?;
        let tx_ctx = self
            .read_column(txn, db_ptr, line, "tx_ctx", |b| {
                postcard::from_bytes(b).ok()
            })?
            .context("missing field tx-ctx")?;
        let entry = LedgerEntry {
            creditor,
            debitor,
            amount_milli,
            tax_milli,
            currency,
            kind,
            tag: tag.to_string(),
        };
        Ok(Some((entry, contract_id, tx_ctx)))
    }
}

fn i64_from_slice(slice: &[u8]) -> Option<i64> {
    let b = slice.try_into().ok()?;
    Some(i64::from_be_bytes(b))
}

/// A ledger-entry meant to be consumed by APIs
#[derive(Serialize)]
pub struct LedgerEntryDto {
    pub creditor: BorderlessId,
    pub debitor: BorderlessId,
    pub amount: String,
    pub tax: String,
    pub kind: String,
    pub tag: String,
    pub contract_id: ContractId,
    pub tx_ctx: TxCtx,
}

impl LedgerEntryDto {
    pub fn new(entry: LedgerEntry, contract_id: ContractId, tx_ctx: TxCtx) -> LedgerEntryDto {
        let amount = Money::new(entry.currency, entry.amount_milli).to_string();
        let tax = Money::new(entry.currency, entry.tax_milli).to_string();
        LedgerEntryDto {
            creditor: entry.creditor,
            debitor: entry.debitor,
            amount,
            tax,
            kind: entry.kind.to_string(),
            tag: entry.tag,
            contract_id,
            tx_ctx,
        }
    }
}

#[derive(Serialize)]
pub struct LedgerIds {
    pub creditor: BorderlessId,
    pub debitor: BorderlessId,
    pub ledger_id: u64,
}

/// Meta information about this ledger
#[derive(Debug, Serialize, Deserialize)]
pub struct LedgerMeta {
    /// Creditor side
    pub creditor: BorderlessId,
    /// Debitor side
    pub debitor: BorderlessId,
    /// Length of the ledger
    pub len: u64,
    /// Balances by currency ( values are in 1000 units, so 1€ = 1000 )
    pub balances: HashMap<Currency, i64>,
}

/// Meta information about this ledger (DTO for JSON-APIs)
///
/// Contains the ledger-id, which is useful for later queries,
/// + converts the amount
#[derive(Serialize)]
pub struct LedgerMetaDto {
    /// ID that is used for this ledger
    pub ledger_id: u64,
    /// Creditor side
    pub creditor: BorderlessId,
    /// Debitor side
    pub debitor: BorderlessId,
    /// Length of the ledger
    pub len: u64,
    /// Balances by currency ( values are normalized, so 1€ = 1.00 €)
    pub balances: HashMap<Currency, f64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// (Optional) Contract-ID, if the query was for a single contract-id only.
    pub contract_id: Option<ContractId>,
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

    pub fn into_dto(self) -> LedgerMetaDto {
        let ledger_id = self.creditor.merge_compact(&self.debitor);
        let balances = self
            .balances
            .into_iter()
            .map(|(k, v)| (k, v as f64 / 1000.0))
            .collect();
        LedgerMetaDto {
            ledger_id,
            creditor: self.creditor,
            debitor: self.debitor,
            len: self.len,
            balances,
            contract_id: None,
        }
    }

    /// Resets the balances and length - useful when creating a temporary balance from the original `LedgerMeta`
    pub fn reset_balance(&mut self) {
        self.balances.clear();
        self.len = 0;
    }

    /// Updates the ledger meta information with the current entry
    ///
    /// Returns an error, if the ledger-entry does not belong to this ledger.
    pub fn update(&mut self, entry: &LedgerEntry) -> Result<()> {
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
        Ok(())
    }
}

/// A 24-bit ledger key constructed from a pair of borderless-ids, a line-index and a 'column' name.
///
/// The column name is used to have different keys for different values.
/// To be able to access the ledger with maximum speed, we do not save a single struct in one line
/// and use a serialization format. Instead we save each value individually by its 'column name'.
/// This allows us to scan through the key-range very efficiently.
///
/// The encoding of the ledger-key is as follows:
/// ```text
/// [ participant-pair | number | value ]
/// [     u64          |  u64   | u64   ]
/// ```
struct LedgerKey([u8; 24]);

impl LedgerKey {
    /// Constructs a new ledger-key for a specific ledger-id, line and column
    ///
    /// The ledger-id is the calculated from the participant-ids by using `BorderlessId::merge_compact`
    pub fn new(ledger_id: u64, line_idx: u64, column: &'static str) -> Self {
        let participant_key = ledger_id.to_be_bytes();
        let line_key = line_idx.to_be_bytes();
        let column_key = xxhash_rust::const_xxh3::xxh3_64(column.as_bytes()).to_be_bytes();
        let mut key = [0; 24];
        key[0..8].copy_from_slice(&participant_key);
        key[8..16].copy_from_slice(&line_key);
        key[16..24].copy_from_slice(&column_key);
        LedgerKey(key)
    }

    /// Returns the key, where the length of the ledger is saved
    pub fn meta(ledger_id: u64) -> Self {
        let participant_key = ledger_id.to_be_bytes();
        // We want the meta-info to be the absolute last key, so we do some bit-hacking here:
        let mut key = [0xff; 24];
        key[0..8].copy_from_slice(&participant_key);
        LedgerKey(key)
    }

    /// Creates a ledger-key from a slice (useful when iterating over the kv-store)
    pub fn from_slice(slice: &[u8]) -> Self {
        let mut key = [0; 24];
        key[..].copy_from_slice(slice);
        LedgerKey(key)
    }

    /// Reconstructs the 'line' (index) from the ledger-key
    pub fn line(&self) -> u64 {
        let mut out = [0; 8];
        out.copy_from_slice(&self.0[8..16]);
        u64::from_be_bytes(out)
    }

    /// Reconstructs the ledger-id from the ledger-key
    pub fn ledger_id(&self) -> u64 {
        let mut out = [0; 8];
        out.copy_from_slice(&self.0[0..8]);
        u64::from_be_bytes(out)
    }

    pub fn is_meta(&self) -> bool {
        self.line() == u64::MAX
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
}
