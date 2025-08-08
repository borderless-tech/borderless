use crate::{Result, SUBSCRIPTION_REL_SUB_DB};
use borderless::common::{Id, Introduction};
use borderless::events::Topic;
use borderless::{AgentId, Context};
use borderless_kv_store::{Db, RawWrite, RoCursor, RoTx};
use std::str::FromStr;

/// Generates a DB key from a publisher, subscriber and topic
///
/// Current DB relationship is: publisher | topic | subscriber => method_name
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

    // NOTE: when building a look-up key without a topic, do not write
    // additional delimiters as they interfere with our cursor logic
    if topic.is_empty() && subscriber.is_empty() {
        format!("{publisher}\n")
    } else {
        // TODO Forbid creating topics containing the newline character
        format!("{publisher}\n{topic}\n{subscriber}")
    }
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

    pub fn init(&self, txn: &mut <S as Db>::RwTx<'_>, introduction: Introduction) -> Result<()> {
        // Write static subscriptions
        match introduction.id {
            Id::Contract { .. } => {} // Not applicable
            Id::Agent { agent_id } => {
                for s in introduction.subscriptions {
                    self.subscribe(txn, agent_id, s)?
                }
            }
        }
        Ok(())
    }

    pub fn subscribe(
        &self,
        txn: &mut <S as Db>::RwTx<'_>,
        subscriber: AgentId,
        topic: Topic,
    ) -> Result<()> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        // TODO Store the subscription's timestamp for debugging purposes?
        let key = generate_key(topic.publisher, topic.topic, Some(subscriber));
        txn.write(&db_ptr, &key, &topic.method)?;
        Ok(())
    }

    pub fn unsubscribe(
        &self,
        txn: &mut <S as Db>::RwTx<'_>,
        subscriber: AgentId,
        publisher: Id,
        topic: String,
    ) -> Result<bool> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let key = generate_key(publisher, topic, Some(subscriber));
        let deleted = txn.delete(&db_ptr, &key).is_ok();
        Ok(deleted)
    }

    pub fn get_topic_subscribers(
        &self,
        publisher: Id,
        topic: String,
    ) -> Result<Vec<(AgentId, String)>> {
        // Setup DB cursor
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let mut subscribers = Vec::new();

        // Use an efficient look-up key
        let prefix = generate_key(publisher, topic, None);

        for (key, value) in cursor.iter_from(&prefix) {
            // Stop iterating when prefix no longer matches
            if !key.starts_with(prefix.as_bytes()) {
                break;
            }
            // Decode method_name
            let topic =
                String::from_utf8(value.to_vec()).with_context(|| "Failed to deserialize topic")?;
            // Push subscriber to vector
            let (_, subscriber) = extract_key(key)?;
            subscribers.push((subscriber, topic));
        }
        // Free up resources
        drop(cursor);
        Ok(subscribers)
    }

    pub fn get_subscribers(&self, publisher: Id) -> Result<Vec<(AgentId, String)>> {
        self.get_topic_subscribers(publisher, String::default())
    }

    pub fn get_subscriptions(&self, target: AgentId) -> Result<Vec<String>> {
        // Setup DB cursor
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
        Ok(topics)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::subscriptions::SubscriptionHandler;
    use crate::SUBSCRIPTION_REL_SUB_DB;
    use borderless::common::Id;
    use borderless::events::Topic;
    use borderless::{AgentId, ContractId, Result};
    use borderless_kv_store::backend::lmdb::Lmdb;
    use borderless_kv_store::{Db, Tx};
    use tempfile::tempdir;

    const N: usize = 10;

    fn open_tmp_lmdb() -> Lmdb {
        let tmp_dir = tempdir().unwrap();
        let env = Lmdb::new(tmp_dir.path(), 1).unwrap();
        env.create_sub_db(SUBSCRIPTION_REL_SUB_DB).unwrap();
        env
    }

    #[test]
    fn subscription() -> Result<()> {
        // Setup dummy DB
        let lmdb = open_tmp_lmdb();
        let handler = SubscriptionHandler::new(&lmdb);
        let mut txn = lmdb.begin_rw_txn()?;

        // Setup: subscribers are sw-agents and publishers are smart-contracts
        let subscribers: Vec<AgentId> = std::iter::repeat_with(|| AgentId::generate())
            .take(N)
            .collect();
        let publishers: Vec<Id> = std::iter::repeat_with(|| Id::agent(AgentId::generate()))
            .take(N)
            .collect();
        let topic = "MyTopic";

        // Generate subscriptions
        for i in 0..N {
            let s = subscribers[i];
            let p = publishers[i];
            let topic = Topic::new(p, topic.to_string(), "method".to_string());
            handler.subscribe(&mut txn, s, topic.clone())?;
        }

        // Commit changes
        txn.commit()?;

        // Check subscriptions are present
        for i in 0..N {
            let s = subscribers[i];
            let p = publishers[i].to_string();

            let subscriptions = handler.get_subscriptions(s)?;
            assert_eq!(subscriptions.len(), 1);
            let full_topic = format!("/{}/{}", p, topic.to_ascii_lowercase());
            assert_eq!(subscriptions[0], full_topic);
        }
        Ok(())
    }

    #[test]
    fn unsubscription() -> Result<()> {
        // Setup dummy DB
        let lmdb = open_tmp_lmdb();
        let handler = SubscriptionHandler::new(&lmdb);

        // Setup: both subscribers and publishers are sw-agents
        let subscribers: Vec<AgentId> = std::iter::repeat_with(|| AgentId::generate())
            .take(N)
            .collect();
        let publishers: Vec<Id> = std::iter::repeat_with(|| Id::agent(AgentId::generate()))
            .take(N)
            .collect();
        let topic = "MyTopic";

        let mut txn = lmdb.begin_rw_txn()?;
        for i in 0..N {
            let topic = Topic::new(publishers[i], topic.to_string(), "method".to_string());
            // Subscribe to topic
            handler.subscribe(&mut txn, subscribers[i], topic)?;
        }
        txn.commit()?;

        let mut txn = lmdb.begin_rw_txn()?;
        for i in 0..N {
            let s = subscribers[i];
            let p = publishers[i];
            // Unsubscribe and check result is true
            assert!(handler.unsubscribe(&mut txn, s, p, topic.to_string())?);
        }
        Ok(())
    }

    #[test]
    fn fetch_topic_subscribers() -> Result<()> {
        // Setup dummy DB
        let lmdb = open_tmp_lmdb();
        let handler = SubscriptionHandler::new(&lmdb);
        let mut txn = lmdb.begin_rw_txn()?;

        // Setup: subscribers are sw-agents and publisher is a smart-contract
        let mut subscribers: Vec<AgentId> = std::iter::repeat_with(|| AgentId::generate())
            .take(N)
            .collect();
        let publisher = Id::contract(ContractId::generate());
        let topic = "tennis";

        for i in 0..N {
            let topic = Topic::new(publisher, topic.to_string(), "method".to_string());
            // Subscribe to topic
            handler.subscribe(&mut txn, subscribers[i], topic)?;
        }
        txn.abort();
        let mut output = handler.get_topic_subscribers(publisher, topic.to_string())?;
        // Check output
        subscribers.sort();
        output.sort();
        // assert_eq!(subscribers, output, "Mismatch in topic subscribers");
        Ok(())
    }

    #[test]
    fn fetch_subscribers() -> Result<()> {
        // Setup dummy DB
        let lmdb = open_tmp_lmdb();
        let handler = SubscriptionHandler::new(&lmdb);

        // Setup: subscribers are sw-agents and publisher is a smart-contract
        let mut subscribers: Vec<AgentId> = std::iter::repeat_with(|| AgentId::generate())
            .take(N)
            .collect();
        let publisher = Id::contract(ContractId::generate());
        let topics = vec!["Soccer", "Tennis", "Golf", "Basketball", "Football"];

        for i in 0..N {
            let topic = Topic::new(publisher, topics[i % 5].to_string(), "method".to_string());
            // Subscribe to topic
            //handler.subscribe(subscribers[i], topic)?;
        }
        let mut output = handler.get_subscribers(publisher)?;
        // Check output
        subscribers.sort();
        output.sort();
        // assert_eq!(subscribers, output, "Mismatch in subscribers");
        Ok(())
    }

    #[test]
    fn fetch_subscriptions() -> Result<()> {
        // Setup dummy DB
        let lmdb = open_tmp_lmdb();
        let handler = SubscriptionHandler::new(&lmdb);

        // Setup: subscriber is a sw-agent and publishers are smart-contracts
        let subscriber = AgentId::generate();
        let publishers: Vec<Id> = std::iter::repeat_with(|| Id::agent(AgentId::generate()))
            .take(N)
            .collect();
        let topics = vec!["Soccer", "Tennis", "Golf", "Basketball", "Football"];

        for i in 0..N {
            let topic = Topic::new(
                publishers[i],
                topics[i % 5].to_string(),
                "method".to_string(),
            );
            // Subscribe to topic
            // handler.subscribe(subscriber, topic)?;
        }
        // TODO Finish this after discussing if returning the full topic or just the topic
        //let mut output = handler.get_subscriptions(subscriber)?;
        //output.sort();
        //assert_eq!(topics, output, "Mismatch in subscriptions");
        Ok(())
    }
}

/*
 * subscribe(cid, "/order/change", "my_action");
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
