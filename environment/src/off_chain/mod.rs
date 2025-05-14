use std::collections::HashMap;

/// The off_chain environment
pub struct EnvInstance {
    /// Simulates a Database
    hashmap: HashMap<Vec<u8>, Vec<u8>>,
}
