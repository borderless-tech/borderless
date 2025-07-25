use crate::Result;
use crate::SUBSCRIPTION_REL_SUB_DB;
use borderless::common::Id;
use borderless::{AgentId, Context};
use borderless_kv_store::{Db, RawWrite, RoCursor, RoTx, Tx};
use std::str::FromStr;

/// Generates a DB key from an AgentId and an unprefixed topic
///
/// Current DB relationship is: topic | subscriber => publisher
fn generate_key(subscriber: AgentId, topic: String) -> String {
    // TODO: Handle lowercase + trailing slash etc.
    let id = subscriber.to_string().to_ascii_lowercase();
    format!("{topic}{id}")
}

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

        let publisher = match publisher {
            Id::Contract { contract_id } => contract_id.to_string().to_ascii_lowercase(),
            Id::Agent { agent_id } => agent_id.to_string().to_ascii_lowercase(),
        };
        let key = generate_key(subscriber, topic);
        // Apply changes to DB
        txn.write(&db_ptr, &key, &publisher)?;
        txn.commit()?;
        Ok(())
    }

    pub fn unsubscribe(&self, subscriber: AgentId, topic: String) -> Result<()> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let mut txn = self.db.begin_rw_txn()?;

        let key = generate_key(subscriber, topic);
        // Apply changes to DB
        txn.delete(&db_ptr, &key)?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_topic_subscribers(&self, publisher: String, topic: String) -> Result<Vec<AgentId>> {
        // Access to DB
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let mut subscribers = Vec::new();
        // TODO Get Sized bytes
        let topic_prefix = topic.as_bytes();

        for (key, value) in cursor.iter_from(&topic_prefix) {
            // Stop iterating when prefix no longer matches
            if !key.starts_with(topic_prefix) {
                break;
            }
            // TODO Read subscriber from key
            // Parse AgentId from encoded bytes
            let s = std::str::from_utf8(value).with_context(|| "Deserialization failed")?;
            let publisher =
                AgentId::from_str(s).with_context(|| "AgentId deserialization error")?;

            // TODO If publisher match, then add subscriber to vector
            // Push subscriber to vector
            //subscribers.push(agent);
        }
        // Free up resources
        drop(cursor);
        drop(txn);
        Ok(subscribers)
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
