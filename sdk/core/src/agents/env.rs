use crate::__private::registers::{REGISTER_BLOCK_CTX, REGISTER_EXECUTOR, REGISTER_TX_CTX};
use crate::__private::storage_keys::{
    BASE_KEY_METADATA, META_SUB_KEY_DESC, META_SUB_KEY_ID, META_SUB_KEY_META,
};
use crate::__private::{read_field, read_register};
use crate::common::{Description, Metadata};
use crate::contracts::env::participants;
use crate::contracts::{BlockCtx, TxCtx};
use borderless_id_types::{aid_prefix, AgentId, BlockIdentifier, BorderlessId, TxIdentifier, Uuid};

/// Checks whether the current running program is a sw-agent
pub fn is_agent() -> bool {
    let id: Uuid = read_field(BASE_KEY_METADATA, META_SUB_KEY_ID).expect("id not in metadata");
    aid_prefix(id.as_bytes())
}

/// Returns the Agent-id of the current sw-agent
pub fn agent_id() -> AgentId {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_ID).expect("agent-id not in metadata")
}

/// Returns the [`Description`] of a sw-agent
pub fn desc() -> Description {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_DESC).expect("description not in metadata")
}

/// Returns the [`Metadata`] of a sw-agent
pub fn meta() -> Metadata {
    read_field(BASE_KEY_METADATA, META_SUB_KEY_META).expect("meta not in metadata")
}

/// Returns the executor of the sw-agent
pub fn executor() -> BorderlessId {
    let bytes = read_register(REGISTER_EXECUTOR).expect("executor not present");
    BorderlessId::from_bytes(bytes.try_into().expect("executor must be a borderless-id"))
}

/// Returns the roles that are assigned to the writer of the current transaction
pub fn writer_roles() -> Vec<String> {
    let writer = crate::contracts::env::writer();
    let participants = participants();

    participants
        .into_iter()
        .filter(|p| p.id == writer)
        .flat_map(|p| p.roles)
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
