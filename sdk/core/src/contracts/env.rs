use anyhow::Context;
use borderless_id_types::{BlockIdentifier, TxIdentifier};

use super::{BlockCtx, Role, Sink, TxCtx};
use crate::common::Participant;
use crate::{
    BorderlessId, ContractId,
    __private::{
        read_field, read_register,
        registers::{REGISTER_BLOCK_CTX, REGISTER_EXECUTOR, REGISTER_TX_CTX, REGISTER_WRITER},
        storage_keys::*,
    },
    common::{Description, Metadata},
};

/// Returns the contract-id of the current contract
pub fn contract_id() -> ContractId {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_CONTRACT_ID).expect("contract-id not in metadata")
}

/// Returns the contract participants
pub fn participants() -> Vec<Participant> {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS).expect("participants not in metadata")
}

/// Returns the roles that are assigned in this contract
pub fn roles() -> Vec<Role> {
    todo!("Do we need this function?")
}

/// Returns the available sinks of this contract
pub fn sinks() -> Vec<Sink> {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_SINKS).expect("sinks not in metadata")
}

/// Returns the contract-id of a sink based on its alias
pub fn sink(alias: impl AsRef<str>) -> crate::Result<ContractId> {
    // Search through all sinks
    sinks()
        .into_iter()
        .find(|s| s.has_alias(alias.as_ref()))
        .map(|s| s.contract_id)
        .with_context(|| format!("failed to find sink with alias '{}'", alias.as_ref()))
}

/// Returns the [`Description`] of a contract
pub fn desc() -> Description {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_DESC).expect("description not in metadata")
}

/// Returns the [`Metadata`] of a contract
pub fn meta() -> Metadata {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_META).expect("meta not in metadata")
}

/// Returns the writer of the current transaction
pub fn writer() -> BorderlessId {
    let bytes = read_register(REGISTER_WRITER).expect("caller not present");
    BorderlessId::from_bytes(bytes.try_into().expect("caller must be a borderless-id"))
}

/// Returns the writer of the current transaction
pub(crate) fn executor() -> BorderlessId {
    let bytes = read_register(REGISTER_EXECUTOR).expect("executor not present");
    BorderlessId::from_bytes(bytes.try_into().expect("executor must be a borderless-id"))
}

/// Returns the roles that are assigned to the writer of the current transaction
pub fn writer_roles() -> Vec<String> {
    let writer = writer();
    roles()
        .into_iter()
        .filter(|r| r.participant_id == writer)
        .map(|r| r.role)
        .collect()
}

/// Returns the [`TxCtx`] for the current transaction
pub fn tx_ctx() -> TxCtx {
    let bytes = read_register(REGISTER_TX_CTX).expect("tx-id not present");
    TxCtx::from_bytes(&bytes).expect("invalid data-model in tx-id register")
}

/// Returns the [`TxId`] for the current transaction
pub fn tx_id() -> TxIdentifier {
    let bytes = read_register(REGISTER_TX_CTX).expect("tx-id not present");
    TxCtx::from_bytes(&bytes)
        .expect("invalid data-model in tx-id register")
        .tx_id
}

/// Returns the transaction-index (index inside the block) for the current transaction
pub fn tx_index() -> u64 {
    let bytes = read_register(REGISTER_TX_CTX).expect("tx-id not present");
    TxCtx::from_bytes(&bytes)
        .expect("invalid data-model in tx-id register")
        .index
}

/// Returns the [`BlockCtx`] for the block for the current transaction
pub fn block_ctx() -> BlockCtx {
    let bytes = read_register(REGISTER_BLOCK_CTX).expect("block-id not present");
    BlockCtx::from_bytes(&bytes).expect("invalid data-model in block-ctx register")
}

/// Returns the [`BlockId`] of the block for the current transaction
pub fn block_id() -> BlockIdentifier {
    let bytes = read_register(REGISTER_BLOCK_CTX).expect("block-id not present");
    BlockCtx::from_bytes(&bytes)
        .expect("invalid data-model in block-ctx register")
        .block_id
}

/// Returns the timestamp of the block for the current transaction
pub fn block_timestamp() -> u64 {
    let bytes = read_register(REGISTER_BLOCK_CTX).expect("block-timestamp not present");
    BlockCtx::from_bytes(&bytes)
        .expect("invalid data-model in block-ctx register")
        .timestamp
}
