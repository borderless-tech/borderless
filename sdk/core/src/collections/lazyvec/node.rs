use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

#[derive(Clone, Serialize, Deserialize)]
pub struct Node<V> {
    pub(crate) keys: Vec<usize>,   // Used by Internal nodes
    pub(crate) children: Vec<u64>, // DB keys of the child nodes (used by Internal nodes)
    pub(crate) level: usize,       // 0 -> Leaf, otherwise -> Internal
    pub(crate) values: Vec<V>,     // Used by Leaves
    pub(crate) next: Option<u64>,  // DB key of the next leave (used by Leaves)
}

impl<V: Debug> Debug for Node<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // NOTE: The f.alternate() is the 'extended-debug' option {:#?} instead of {:?}
        if f.alternate() {
            if self.is_leaf() {
                write!(
                    f,
                    "Leaf {{ values: {:?}, next: {:?} }}",
                    self.values, self.next
                )
            } else {
                write!(
                    f,
                    "Internal {{ keys: {:?}, children: {:?}, level: {} }}",
                    self.keys, self.children, self.level
                )
            }
        } else if self.is_leaf() {
            write!(f, "Leaf {{ {:?} }}", self.values)
        } else {
            write!(f, "Internal {{ {:?} }}", self.keys)
        }
    }
}

impl<V> Node<V> {
    pub(crate) fn rank(&self) -> usize {
        if self.is_leaf() {
            self.values.len()
        } else {
            self.keys.iter().sum()
        }
    }

    pub(crate) fn is_leaf(&self) -> bool {
        self.level == 0
    }

    pub(crate) fn level(&self) -> usize {
        self.level
    }

    pub(crate) fn generate_internal(order: usize, level: usize, node_key: u64) -> Self {
        let mut keys = Vec::with_capacity(order);
        let mut children = Vec::with_capacity(order);
        keys.push(1); // Only the new node is added
        children.push(node_key);

        Node {
            keys,
            children,
            level,
            values: vec![],
            next: None,
        }
    }
}

impl<V> Node<V>
where
    V: Serialize + for<'de> Deserialize<'de> + Clone,
{
    pub(crate) fn empty_leaf(order: usize) -> Self {
        Node {
            keys: vec![],
            children: vec![],
            level: 0,
            values: Vec::with_capacity(order),
            next: None,
        }
    }

    pub(crate) fn generate_leaf(order: usize, values: Vec<V>, next: Option<u64>) -> Self {
        let mut new_values = Vec::with_capacity(order);
        new_values.extend(values);

        Node {
            keys: vec![],
            children: vec![],
            level: 0,
            values: new_values,
            next,
        }
    }
}