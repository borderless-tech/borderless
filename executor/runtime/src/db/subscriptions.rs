use crate::{Result, SUBSCRIPTION_REL_SUB_DB};
use borderless::common::{Id, Introduction};
use borderless::events::Topic;
use borderless::{AgentId, Context, Uuid};
use borderless_kv_store::{Db, RawWrite, RoCursor, RoTx, Tx};
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

    //NOTE: in look-up keys the subscriber must be empty
    // The unused delimiters are removed to avoid interferences with the DB cursor
    match (topic.is_empty(), subscriber.is_empty()) {
        (true, true) => format!("{publisher}\n"),
        (false, true) => format!("{publisher}\n{topic}\n"),
        _ => format!("{publisher}\n{topic}\n{subscriber}"),
    }
}

/// Extracts the topic and subscriber from a DB entry
///
/// Returns a tuple, or an error if the deserialization fails
fn extract_entry(key: &[u8], value: &[u8]) -> Result<(Topic, AgentId)> {
    let key = std::str::from_utf8(key).with_context(|| "DB key deserialization failed")?;
    let method = std::str::from_utf8(value).with_context(|| "DB value deserialization failed")?;

    let mut parts = key.splitn(3, '\n');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(p), Some(topic), Some(s)) => {
            // Process subscriber
            let subscriber = AgentId::from_str(s).with_context(|| "Invalid subscriber")?;
            // Process publisher
            let p = Uuid::parse_str(p).with_context(|| "Invalid publisher")?;
            let publisher = Id::try_from(p).with_context(|| "Invalid publisher")?;
            Ok((Topic::new(publisher, topic, method), subscriber))
        }
        _ => Err(crate::Error::msg("Malformed key error")),
    }
}

pub struct SubscriptionHandler<'a, S: Db> {
    db: &'a S,
}

impl<'a, S: Db> SubscriptionHandler<'a, S> {
    pub fn new(db: &'a S) -> Self {
        Self { db }
    }

    /// Loads the subscriptions from a software agent introduction
    pub fn init(&self, txn: &mut <S as Db>::RwTx<'_>, introduction: Introduction) -> Result<()> {
        // Write static subscriptions
        match introduction.id {
            Id::Contract { .. } => {} // Not applicable
            Id::Agent { agent_id } => {
                for s in introduction.subscriptions {
                    self.subscribe_txn(txn, agent_id, s)?
                }
            }
        }
        Ok(())
    }

    /// Subscribes an ['AgentId'] to a topic from a specific publisher
    ///
    /// The changes are automatically commited to DB
    pub fn subscribe(&self, subscriber: AgentId, topic: Topic) -> Result<()> {
        let mut txn = self.db.begin_rw_txn()?;
        self.subscribe_txn(&mut txn, subscriber, topic)?;
        Ok(txn.commit()?)
    }

    /// Subscribes an ['AgentId'] to a topic from a specific publisher
    ///
    /// The user is responsible for commiting the changes to DB
    fn subscribe_txn(
        &self,
        txn: &mut <S as Db>::RwTx<'_>,
        subscriber: AgentId,
        topic: Topic,
    ) -> Result<()> {
        // Setup DB access
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        // Generate DB key
        let key = generate_key(topic.publisher, topic.topic, Some(subscriber));
        txn.write(&db_ptr, &key, &topic.method)?;
        Ok(())
    }

    /// Unsubscribes an ['AgentId'] from a topic
    ///
    /// The changes are automatically commited to DB
    pub fn unsubscribe(&self, subscriber: AgentId, topic: Topic) -> Result<()> {
        let mut txn = self.db.begin_rw_txn()?;
        self.unsubscribe_txn(&mut txn, subscriber, topic)?;
        Ok(txn.commit()?)
    }

    /// Unsubscribes an ['AgentId'] from a topic
    ///
    /// The user is responsible for commiting the changes to DB
    fn unsubscribe_txn(
        &self,
        txn: &mut <S as Db>::RwTx<'_>,
        subscriber: AgentId,
        topic: Topic,
    ) -> Result<()> {
        // Setup DB access
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        // Generate DB key
        let key = generate_key(topic.publisher, topic.topic, Some(subscriber));
        Ok(txn.delete(&db_ptr, &key)?)
    }

    /// Fetches the active subscribers for a full topic (publisher + topic)
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
            let (topic, subscriber) = extract_entry(key, value)?;
            // Push the tuple
            subscribers.push((subscriber, topic.topic));
        }
        // Free up resources
        drop(cursor);
        Ok(subscribers)
    }

    /// Fetches all active subscriptions for the specified ['AgentId']
    pub fn get_subscriptions(&self, target: AgentId) -> Result<Vec<Topic>> {
        // Setup DB cursor
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let mut topics = Vec::new();
        for (key, value) in cursor.iter() {
            let (topic, subscriber) = extract_entry(key, value)?;
            // Ignore subscription not related with target
            if target != subscriber {
                continue;
            }
            // Push the topic
            topics.push(topic);
        }
        // Free up resources
        drop(cursor);
        Ok(topics)
    }

    pub fn unsubscribe_all(&self, txn: &mut <S as Db>::RwTx<'_>, subscriber: Id) -> Result<()> {
        let subscriber = match subscriber {
            Id::Contract { .. } => return Ok(()), // Not applicable
            Id::Agent { agent_id } => agent_id,
        };
        // Fetch active subscriptions
        let subscriptions = self.get_subscriptions(subscriber)?;
        // Unsubscribe from each topic
        for topic in subscriptions {
            self.unsubscribe_txn(txn, subscriber, topic)?;
        }
        Ok(())
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
    use borderless_kv_store::Db;
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
            handler.subscribe(s, topic.clone())?;
        }

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

        // Generate subscriptions
        for i in 0..N {
            let topic = Topic::new(publishers[i], topic.to_string(), "method".to_string());
            // Subscribe to topic
            handler.subscribe(subscribers[i], topic)?;
        }

        // Check that unsubscriptions are successful
        for i in 0..N {
            let s = subscribers[i];
            let p = publishers[i];
            // Unsubscribe from topic
            handler.unsubscribe(s, p, topic.to_string())?;
        }

        // All subscriptions must be gone
        for p in publishers {
            assert!(handler
                .get_topic_subscribers(p, topic.to_string())?
                .is_empty());
        }
        Ok(())
    }

    #[test]
    fn fetch_topic_subscribers() -> Result<()> {
        // Setup dummy DB
        let lmdb = open_tmp_lmdb();
        let handler = SubscriptionHandler::new(&lmdb);

        // Setup: subscribers are sw-agents and publisher is a smart-contract
        let mut subscribers: Vec<AgentId> = std::iter::repeat_with(|| AgentId::generate())
            .take(N)
            .collect();
        let publisher = Id::contract(ContractId::generate());
        let topic = "tennis";

        // Generate subscriptions
        for i in 0..N {
            let topic = Topic::new(publisher, topic.to_string(), "method".to_string());
            // Subscribe to topic
            handler.subscribe(subscribers[i], topic)?;
        }

        // Fetch topic subscribers
        let mut output: Vec<AgentId> = handler
            .get_topic_subscribers(publisher, topic.to_string())?
            .iter()
            .map(|(aid, _)| aid)
            .cloned()
            .collect();
        // Check output
        subscribers.sort();
        output.sort();
        assert_eq!(subscribers, output, "Mismatch in topic subscribers");
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

        // Generate subscriptions
        for i in 0..N {
            let topic = Topic::new(publisher, topics[i % 5].to_string(), "method".to_string());
            // Subscribe to topic
            handler.subscribe(subscribers[i], topic)?;
        }

        // Fetch subscribers
        let mut output: Vec<AgentId> = handler
            .get_topic_subscribers(publisher, String::default())?
            .iter()
            .map(|(aid, _)| aid)
            .cloned()
            .collect();
        // Check output
        subscribers.sort();
        output.sort();
        assert_eq!(subscribers, output, "Mismatch in subscribers");
        Ok(())
    }

    #[test]
    fn fetch_subscriptions() -> Result<()> {
        // Setup dummy DB
        let lmdb = open_tmp_lmdb();
        let handler = SubscriptionHandler::new(&lmdb);

        // Setup: subscriber is a sw-agent and publishers are smart-contracts
        let subscriber = AgentId::generate();
        let topics = vec!["Soccer", "Tennis", "Golf", "Basketball", "Football"];

        let mut full_topic: Vec<String> = Vec::new();
        // Generate subscriptions
        for i in 0..N {
            let p = AgentId::generate();
            let topic = topics[i % 5].to_string();
            full_topic.push(format!("/{}/{}", p, topic.to_ascii_lowercase()));
            // Subscribe to topic
            let topic = Topic::new(Id::agent(p), topic, "method".to_string());
            handler.subscribe(subscriber, topic)?;
        }

        // Fetch subscriptions
        let mut output = handler.get_subscriptions(subscriber)?;
        output.sort();
        full_topic.sort();
        assert_eq!(full_topic, output, "Mismatch in subscriptions");
        Ok(())
    }
}
