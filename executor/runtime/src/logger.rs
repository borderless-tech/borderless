use std::cmp::min;

use anyhow::Result;
use borderless_kv_store::*;
use borderless_sdk::{
    internal::storage_keys::{StorageKey, BASE_KEY_LOGS},
    log::{LogLevel, LogLine},
    ContractId,
};
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

use crate::CONTRACT_SUB_DB;

/// Storage key, where the meta-information about the buffer is saved
const SUB_KEY_META: u64 = u64::MAX;

/// We keep a maximum of 32k log-lines ( which should be sufficient for debugging )
const MAX_LOG_BUFFER_SIZE: u64 = 32 * 1024;

#[derive(Serialize, Deserialize, Default)]
struct BufferMeta {
    start: u64,
    end: u64,
    /// Absolute index at which the last flush started.
    last_flush_start: u64,
    /// Number of log lines flushed in the last flush.
    last_flush_count: u64,
}

/// Logger instance that is created over a key-value storage for a given contract-id
///
/// The logger is essentially a ring-buffer with a fixed size, that uses a specific key-space.
pub struct Logger<'a, S: Db> {
    db: &'a S,
    cid: ContractId,
}

impl<'a, S: Db> Logger<'a, S> {
    pub fn new(db: &'a S, cid: ContractId) -> Self {
        Self { db, cid }
    }

    /// Flushes the given log lines into the ring-buffer.
    ///
    /// This function writes a batch of log lines into the underlying key-value storage. It performs the following steps:
    ///
    /// 1. Reads the current buffer metadata, which includes the logical start and end indices of the stored log lines.
    /// 2. Determines if adding the new log lines would exceed the fixed capacity (`MAX_LOG_BUFFER_SIZE`). If so,
    ///    it advances the start index to overwrite the oldest entries.
    /// 3. Records the flush metadata (`last_flush_start` and `last_flush_count`) to track the range of log lines added in this flush.
    /// 4. Writes the new log lines to storage using modulo arithmetic to map the logical indices to physical storage keys.
    /// 5. Updates and persists the modified metadata.
    ///
    /// # Arguments
    ///
    /// * `lines` - A slice of `LogLine` objects to be flushed into the buffer.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the flush is successful.
    ///
    /// # Errors
    ///
    /// Returns an error if any database operation fails or if serialization/deserialization of log lines or metadata fails.
    pub fn flush_lines(&self, lines: &[LogLine]) -> Result<()> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let mut txn = self.db.begin_rw_txn()?;

        // Retrieve meta info, or initialize it if not present.
        let meta_key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, SUB_KEY_META);
        let mut meta = match txn.read(&db_ptr, &meta_key)? {
            Some(bytes) => postcard::from_bytes(bytes)?,
            None => {
                // Initialize with flush info set to 0.
                let meta = BufferMeta::default();
                let bytes = postcard::to_allocvec(&meta)?;
                txn.write(&db_ptr, &meta_key, &bytes)?;
                meta
            }
        };

        let new_line_count = lines.len() as u64;
        let current_count = meta.end - meta.start;

        // If adding new lines would overflow the ring buffer, adjust the start index.
        if current_count + new_line_count > MAX_LOG_BUFFER_SIZE {
            let drop_count = current_count + new_line_count - MAX_LOG_BUFFER_SIZE;
            meta.start += drop_count;
        }

        // Record the flush information: where the flush starts and how many lines are flushed.
        meta.last_flush_start = meta.end;
        meta.last_flush_count = new_line_count;

        // Write each new log line using modulo arithmetic to wrap-around.
        for (i, line) in lines.iter().enumerate() {
            let index = (meta.end + i as u64) % MAX_LOG_BUFFER_SIZE;
            let key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, index);
            let bytes = postcard::to_allocvec(line)?;
            txn.write(&db_ptr, &key, &bytes)?;
        }

        // Update meta with the new end.
        meta.end += new_line_count;
        let meta_bytes = postcard::to_allocvec(&meta)?;
        txn.write(&db_ptr, &meta_key, &meta_bytes)?;

        txn.commit()?;
        Ok(())
    }

    /// Retrieves the full log from the buffer in chronological order.
    pub fn get_full_log(&self) -> Result<Vec<LogLine>> {
        self.get_log_lines(0, MAX_LOG_BUFFER_SIZE)
    }

    /// Retrieves a range of log lines from the buffer in chronological order.
    ///
    /// # Arguments
    ///
    /// * `start_offset` - The number of log lines to skip from the oldest entry.
    /// * `count` - The maximum number of log lines to retrieve.
    ///
    /// For example, to get the 100 oldest log lines, call with start_offset = 0 and count = 100.
    pub fn get_log_lines(&self, start_offset: u64, count: u64) -> Result<Vec<LogLine>> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let meta_key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, SUB_KEY_META);
        // Read meta info; if missing, assume an empty buffer.
        let meta = match txn.read(&db_ptr, &meta_key)? {
            Some(bytes) => postcard::from_bytes(bytes)?,
            None => BufferMeta::default(),
        };

        let total_count = meta.end - meta.start;
        // If the requested start offset is beyond the current log count, return an empty Vec.
        if start_offset >= total_count {
            return Ok(Vec::new());
        }
        // Determine the absolute indices in the logical log buffer.
        let range_start = meta.start + start_offset;
        let range_end = min(range_start + count, meta.end);

        let mut logs = Vec::new();
        // Iterate over the specified range and fetch each log line.
        for i in range_start..range_end {
            // Compute the physical index using modulo arithmetic.
            let index = i % MAX_LOG_BUFFER_SIZE;
            let key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, index);
            if let Some(bytes) = txn.read(&db_ptr, &key)? {
                let log_line: LogLine = postcard::from_bytes(bytes)?;
                logs.push(log_line);
            }
        }
        Ok(logs)
    }

    /// Retrieves the log lines that were flushed in the last call to `flush_lines`.
    pub fn get_last_log(&self) -> Result<Vec<LogLine>> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let meta_key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, SUB_KEY_META);

        let meta: BufferMeta = match txn.read(&db_ptr, &meta_key)? {
            Some(bytes) => postcard::from_bytes(bytes)?,
            None => return Ok(Vec::new()),
        };

        let mut logs = Vec::new();
        let flush_start = meta.last_flush_start;
        let flush_count = meta.last_flush_count;

        // Iterate over the range corresponding to the last flush.
        for i in flush_start..(flush_start + flush_count) {
            // Compute the physical index using modulo arithmetic.
            let index = i % MAX_LOG_BUFFER_SIZE;
            let key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, index);
            if let Some(bytes) = txn.read(&db_ptr, &key)? {
                let log_line: LogLine = postcard::from_bytes(bytes)?;
                logs.push(log_line);
            }
        }
        Ok(logs)
    }

    /// Returns the total number of log lines ever flushed.
    ///
    /// Note that this number is the absolute index of the last flushed log line,
    /// so if logs have been overwritten in the ring-buffer, the current log count
    /// (meta.end - meta.start) may be lower.
    pub fn total_log_lines(&self) -> Result<u64> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let meta_key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, SUB_KEY_META);
        // If meta is missing, we assume no logs have been flushed yet.
        let meta = match txn.read(&db_ptr, &meta_key)? {
            Some(bytes) => postcard::from_bytes(bytes)?,
            None => BufferMeta::default(),
        };
        Ok(meta.end)
    }

    /// Retrieves log lines for the given page and the total number of pages.
    ///
    /// # Arguments
    ///
    /// * `page` - Zero-based page index.
    /// * `per_page` - The number of log lines per page.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - A Vec of LogLine for the requested page.
    /// - The total number of pages.
    pub fn get_logs_paginated(&self, page: u64, per_page: u64) -> Result<(Vec<LogLine>, u64)> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let meta_key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, SUB_KEY_META);

        // Retrieve meta information. If not found, assume an empty buffer.
        let meta = match txn.read(&db_ptr, &meta_key)? {
            Some(bytes) => postcard::from_bytes(bytes)?,
            None => BufferMeta {
                start: 0,
                end: 0,
                last_flush_start: 0,
                last_flush_count: 0,
            },
        };

        // Calculate the total number of log lines currently in the ring-buffer.
        let total_count = meta.end - meta.start;
        // Calculate total pages using ceiling division.
        let total_pages = if total_count == 0 {
            0
        } else {
            (total_count + per_page - 1) / per_page
        };

        // Calculate the logical start and end indices for the requested page.
        let page_start = meta.start + page * per_page;
        // If the start index is beyond the end of the stored logs, return an empty Vec.
        if page_start >= meta.end {
            return Ok((Vec::new(), total_pages));
        }
        let page_end = std::cmp::min(meta.start + (page + 1) * per_page, meta.end);

        // Retrieve the logs for the calculated range.
        let mut logs = Vec::new();
        for i in page_start..page_end {
            // Map the logical index to the physical index in the ring-buffer.
            let physical_index = i % MAX_LOG_BUFFER_SIZE;
            let key = StorageKey::system_key(&self.cid, BASE_KEY_LOGS, physical_index);
            if let Some(bytes) = txn.read(&db_ptr, &key)? {
                let log_line: LogLine = postcard::from_bytes(bytes)?;
                logs.push(log_line);
            }
        }
        Ok((logs, total_pages))
    }
}

/// Just prints a log line to stdout
///
/// Ignores the timestamp
pub fn print_log_line(line: LogLine) {
    let msg = line.msg;
    match line.level {
        LogLevel::Trace => trace!("{msg}"),
        LogLevel::Debug => debug!("{msg}"),
        LogLevel::Info => info!("{msg}"),
        LogLevel::Warn => warn!("{msg}"),
        LogLevel::Error => error!("{msg}"),
    }
}
