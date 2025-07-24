use crate::SUBSCRIPTION_REL_SUB_DB;
use anyhow::Result;
use borderless::common::Id;
use borderless::AgentId;
use borderless_kv_store::{Db, RawWrite, Tx};
use wasmtime::component::__internal::anyhow;

pub struct SubscriptionHandler<'a, S: Db> {
    db: &'a S,
}

impl<'a, S: Db> SubscriptionHandler<'a, S> {
    pub fn new(db: &'a S) -> Self {
        Self { db }
    }
    pub fn subscribe(&self, subscriber: AgentId, publisher: Id, topic: String) -> Result<()> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let mut txn = self.db.begin_rw_txn()?;

        // Current DB relationship = topic | receiver => publisher
        // TODO: Handle lowercase + trailing slash etc.
        let subscriber = subscriber.to_string().to_ascii_lowercase();
        let publisher = match publisher {
            Id::Contract { contract_id } => contract_id.to_string().to_ascii_lowercase(),
            Id::Agent { agent_id } => agent_id.to_string().to_ascii_lowercase(),
        };
        let key = format!("{publisher}{topic}");

        // Apply changes to DB
        txn.write(&db_ptr, &key, &subscriber)?;
        txn.commit()?;
        Ok(())
    }

    pub fn unsubscribe(&self, publisher: Id, topic: String) -> Result<()> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let mut txn = self.db.begin_rw_txn()?;

        // TODO Create auxiliary function DRY?
        let publisher = match publisher {
            Id::Contract { contract_id } => contract_id.to_string().to_ascii_lowercase(),
            Id::Agent { agent_id } => agent_id.to_string().to_ascii_lowercase(),
        };
        let key = format!("{publisher}{topic}");

        // Apply changes to DB
        txn.delete(&db_ptr, &key)?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_subscribers_of_topic(topic: String) -> Vec<AgentId> {
        todo!()
    }

    pub fn get_subscribers(id: Id) -> Vec<(AgentId, String /* topic-string */)> {
        todo!()
    }

    pub fn get_subscriptions(aid: AgentId) -> Vec<String> {
        todo!()
    }
}

/*
 * subscibe(cid, "/order/change", "my_action");
 *
 * Message { topic, value } <<<< Bake in the contract-id or agent-id of the emitter of the message
 * -> lookup subscribers -> agent-id
 * -> "my_action" -> CallAction { method_name: "my_action", value }
 * -> call action with agent-id and action-struct
 *
 * fn convert_messages(msgs: Vec<Message>, db: &'a DB) -> Vec<(AgentId, CallAction)> { ... }
 *
 *
 *
 * A message from contract cc963345-4cd9-8f30-b215-2cdffee3d189 on topic /foo/baaa should become:
 *
 * /cc963345-4cd9-8f30-b215-2cdffee3d189/foo/baa
 * (this gives you the option of splitting out the ID)
 *
 * -> do a lowercase conversion, so e.g. MY-TOPIC and My-topic would become my-topic
 */
