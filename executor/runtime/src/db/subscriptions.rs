use crate::Result;
use crate::SUBSCRIPTION_REL_SUB_DB;
use borderless::common::Id;
use borderless::{AgentId, Context};
use borderless_kv_store::{Db, RawWrite, RoCursor, RoTx, Tx};
use std::str::FromStr;

/// Generates a DB key from a publisher, subscriber and topic
///
/// Current DB relationship is: publisher | topic | subscriber => ()
///
/// For generating a subscribers look-up key, leave the subscriber field as None
fn generate_key(publisher: Id, topic: String, subscriber: Option<AgentId>) -> String {
    // Publishers can be either Contracts or Agents
    let publisher = match publisher {
        Id::Contract { contract_id } => contract_id.to_string().to_ascii_lowercase(),
        Id::Agent { agent_id } => agent_id.to_string().to_ascii_lowercase(),
    };
    // Subscribers are only Agents
    let subscriber = subscriber
        .map(|agent| agent.to_string().to_ascii_lowercase())
        .unwrap_or_default();
    // Remove leading and trailing slashes
    let topic = topic.trim_matches('/').to_ascii_lowercase();
    // TODO Forbid creating topics containing the newline character
    format!("{publisher}\n{topic}\n{subscriber}")
}

/// Extracts the full topic (publisher + topic) and subscriber from a DB key
///
/// Returns a tuple, or an error if the deserialization fails
fn extract_key(key: &[u8]) -> Result<(String, AgentId)> {
    let key = std::str::from_utf8(key).with_context(|| "DB key deserialization failed")?;

    let mut parts = key.splitn(3, '\n');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(publisher), Some(topic), Some(s)) => {
            let subscriber =
                AgentId::from_str(s).with_context(|| "AgentId deserialization error")?;
            let full_topic = format!("/{publisher}/{topic}");
            Ok((full_topic, subscriber))
        }
        _ => todo!("Use crate error"),
    }
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

        let key = generate_key(publisher, topic, Some(subscriber));
        // Apply changes to DB
        txn.write(&db_ptr, &key, &[])?; // Store a placeholder as value
        txn.commit()?;
        Ok(())
    }

    pub fn unsubscribe(&self, subscriber: AgentId, publisher: Id, topic: String) -> Result<bool> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let mut txn = self.db.begin_rw_txn()?;

        let key = generate_key(publisher, topic, Some(subscriber));
        // Apply changes to DB
        let deleted = txn.delete(&db_ptr, &key).is_ok();
        txn.commit()?;
        Ok(deleted)
    }

    pub fn get_topic_subscribers(&self, publisher: Id, topic: String) -> Result<Vec<AgentId>> {
        // Access to DB
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let mut subscribers = Vec::new();

        // Use an efficient look-up key
        let prefix = generate_key(publisher, topic, None);

        for (key, _) in cursor.iter_from(&prefix.as_bytes()) {
            // Stop iterating when prefix no longer matches
            if !key.starts_with(prefix.as_bytes()) {
                break;
            }
            // Push subscriber to vector
            let (_, subscriber) = extract_key(key)?;
            subscribers.push(subscriber);
        }
        // Free up resources
        drop(cursor);
        drop(txn);
        Ok(subscribers)
    }

    pub fn get_subscribers(&self, publisher: Id) -> Result<Vec<AgentId>> {
        self.get_topic_subscribers(publisher, String::default())
    }

    pub fn get_subscriptions(&self, target: AgentId) -> Result<Vec<String>> {
        // Access to DB
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let mut topics = Vec::new();

        // TODO Avoid iterating all the keys?
        for (key, _) in cursor.iter() {
            let (full_topic, subscriber) = extract_key(key)?;
            // Ignore subscription not related with target
            if target != subscriber {
                continue;
            }
            // Push the full topic
            // TODO Return topic or full topic?
            topics.push(full_topic);
        }
        // Free up resources
        drop(cursor);
        drop(txn);
        Ok(topics)
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
