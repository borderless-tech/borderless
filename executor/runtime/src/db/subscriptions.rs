use crate::Result;
use crate::SUBSCRIPTION_REL_SUB_DB;
use borderless::common::Id;
use borderless::{AgentId, Context};
use borderless_kv_store::{Db, RawWrite, RoCursor, RoTx, Tx};
use std::str::FromStr;

/// Generates a DB key from a publisher, subscriber and topic
///
/// Current DB relationship is: publisher | topic | subscriber => ()
fn generate_key(publisher: Id, subscriber: AgentId, topic: String) -> String {
    // TODO: Handle lowercase + trailing slash etc.
    let publisher = match publisher {
        Id::Contract { contract_id } => contract_id.to_string().to_ascii_lowercase(),
        Id::Agent { agent_id } => agent_id.to_string().to_ascii_lowercase(),
    };
    let subscriber = subscriber.to_string().to_ascii_lowercase();

    // TODO NUL in ASCII is rare in text (is it a valid delimiter?)
    format!("{publisher}\0{topic}\0{subscriber}")
}

/// Generates a DB key from a publisher and topic
///
/// Designed for efficient look-ups of a topic's subscribers
fn generate_topic_key(publisher: Id, topic: String) -> String {
    let publisher = match publisher {
        Id::Contract { contract_id } => contract_id.to_string().to_ascii_lowercase(),
        Id::Agent { agent_id } => agent_id.to_string().to_ascii_lowercase(),
    };
    format!("{publisher}\0{topic}")
}

/// Extracts the subscriber from a DB key
///
/// Returns an AgentId, or an error if the deserialization fails
fn extract_subscriber(key: &[u8]) -> Result<AgentId> {
    // Extract subscriber from key
    let key = std::str::from_utf8(key).with_context(|| "DB key deserialization failed")?;
    let s = key.rsplit('\0').next().unwrap();
    Ok(AgentId::from_str(s).with_context(|| "AgentId deserialization error")?)
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

        let key = generate_key(publisher, subscriber, topic);
        // Apply changes to DB
        txn.write(&db_ptr, &key, &[])?; // Store a placeholder as value
        txn.commit()?;
        Ok(())
    }

    pub fn unsubscribe(&self, subscriber: AgentId, publisher: Id, topic: String) -> Result<()> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let mut txn = self.db.begin_rw_txn()?;

        let key = generate_key(publisher, subscriber, topic);
        // Apply changes to DB
        txn.delete(&db_ptr, &key)?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_topic_subscribers(&self, publisher: Id, topic: String) -> Result<Vec<AgentId>> {
        // Access to DB
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let mut subscribers = Vec::new();

        // Use an efficient look-up key
        let prefix = generate_topic_key(publisher, topic);

        for (key, _) in cursor.iter_from(&prefix.as_bytes()) {
            // Stop iterating when prefix no longer matches
            if !key.starts_with(prefix.as_bytes()) {
                break;
            }
            // Push subscriber to vector
            subscribers.push(extract_subscriber(key)?);
        }
        // Free up resources
        drop(cursor);
        drop(txn);
        Ok(subscribers)
    }

    pub fn get_subscribers(&self, publisher: Id) -> Result<Vec<AgentId>> {
        self.get_topic_subscribers(publisher, String::default())
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
