pub struct SubscriptionHandler<'a, S: Db> {
    db: &'a S,
}

impl SubscriptionHandler {
    pub fn subscribe(subscriber: AgentId, publisher: Id, unprefixed_topic: String) {
        let topic = format!("/{publisher}/{unprefixed_topic}"); // TODO: Handle lowercase + trailing slash etc.
        todo!()
    }

    pub fn unsubscribe(unsubscriber: AgentId, publisher: Id, unprefixed_topic: String) {
        todo!()
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
