mod cache;
mod iterator;
mod node;
mod proxy;

// use super::iterator::LazyVecItMut;
use crate::__private::storage_traits;
use crate::__private::storage_traits::private::Sealed;
use cache::Cache;
use iterator::LazyVecIt;
use node::Node;
use proxy::{Proxy, ProxyMut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
// use std::ops::{Index, IndexMut};

/*
 * IntMap<u64, Product>;
 *         |      |
 *      sub-key   +-> ( key<u64>, value<Product> )
 *
 * get(key: u64)                    -> read_field(BASE_KEY, key) -> (key, value) -> &(_, value)
 * insert(key: u64, value: Product) -> (key, value) -> write_field(BASE_KEY, key, value)
 *
 * Map<String, Product>
 *         |      |
 *      sub-key   +-> ( key<String>, value<Product> )
 */

pub(crate) const ROOT_KEY: u64 = 0;
pub(crate) const ORDER: usize = 16;

pub struct LazyVec<V> {
    cache: Cache<V>,
}

impl<V: Serialize> Debug for LazyVec<V>
where
    V: Serialize + DeserializeOwned + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.fmt_internal(ROOT_KEY, 0, f).is_ok() {
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<V> LazyVec<V>
where
    V: Serialize + DeserializeOwned + Debug,
{
    fn fmt_internal(&self, node_key: u64, depth: usize, f: &mut Formatter<'_>) -> std::fmt::Result {
        let indent = "  ".repeat(depth);
        // Load node from the DB
        let node = self.cache.read(node_key);

        if f.alternate() {
            writeln!(f, "{}{:#?}", indent, node)?;
        } else {
            writeln!(f, "{}{:?}", indent, node)?;
        }

        for child_key in node.borrow().children.iter() {
            self.fmt_internal(*child_key, depth.saturating_add(1), f)?;
        }

        Ok(())
    }
}

impl<V> LazyVec<V>
where
    V: Serialize + DeserializeOwned + PartialEq,
{
    pub fn contains(&self, value: V) -> bool {
        let mut node = self.cache.read(ROOT_KEY);

        // Traverse down to the first leaf of the tree
        while !node.borrow().is_leaf() {
            // Invariant: internal nodes will always contain at least 1 child
            let child_key = node.borrow().children[0];
            node = self.cache.read(child_key);
        }

        // Traverse through the linked leaf nodes
        loop {
            if node.borrow().values.contains(&value) {
                return true;
            }
            let next = node.borrow().next;
            match next {
                None => return false,
                Some(key) => node = self.cache.read(key),
            }
        }
    }
}

impl<V> Sealed for LazyVec<V> where V: Serialize + DeserializeOwned {}

impl<V> storage_traits::Storeable for LazyVec<V>
where
    V: Serialize + DeserializeOwned,
{
    fn decode(base_key: u64) -> Self {
        LazyVec::open(base_key)
    }

    fn parse_value(value: serde_json::Value, base_key: u64) -> anyhow::Result<Self> {
        let values: Vec<V> = serde_json::from_value(value)?;
        let mut out = Self::new(base_key);
        for v in values {
            out.push(v);
        }
        Ok(out)
    }

    fn commit(self, _base_key: u64) {
        self.cache.commit();
    }
}

impl<V> storage_traits::ToPayload for LazyVec<V>
where
    V: Serialize + DeserializeOwned,
{
    fn to_payload(&self, path: &str) -> anyhow::Result<Option<String>> {
        // As this is a vector, there is no further nesting
        if !path.is_empty() {
            return Ok(None);
        }
        // We build the json output manually to save performance
        let n_items = self.len();
        if n_items == 0 {
            return Ok(Some("[]".to_string()));
        }
        let mut items = self.iter();
        let first = items.next().unwrap(); // We checked empty

        // To pre-allocate the output string, we encode one object and use this as a reference
        let encoded = serde_json::to_string(first.as_ref())?;

        // for N items: N * ITEM_LENGTH + (N-1) (commas) + 2 ('[]'); add some padding just in case
        let mut buf = String::with_capacity(encoded.len() * n_items + n_items + 10);
        buf.push('[');
        buf.push_str(&encoded);
        buf.push(',');
        for item in items {
            let encoded = serde_json::to_string(item.as_ref())?;
            buf.push_str(&encoded);
            buf.push(',');
        }
        // Remove trailing ','
        if n_items > 1 {
            buf.pop();
        }
        buf.push(']');
        Ok(Some(buf))
    }
}

// // TODO
// impl<V> Index<usize> for LazyVec<V>
// where
//     V: Serialize + DeserializeOwned,
// {
//     type Output = Proxy<'a, V>;

//     fn index(&self, index: usize) -> &Self::Output {
//         self.get(index).as_deref().unwrap()
//     }
// }

// impl<V> IndexMut<usize> for LazyVec<V>
// where
//     V: Serialize + DeserializeOwned,
// {
//     fn index_mut(&mut self, index: usize) -> &mut Self::Output {
//         self.get_mut(index).expect("Index out of bounds")
//     }
// }

impl<V> LazyVec<V>
where
    V: Serialize + DeserializeOwned,
{
    fn get_node_rank(&self, key: u64) -> usize {
        self.cache.read(key).borrow().rank()
    }

    fn is_node_empty(&self, key: u64) -> bool {
        let node = self.cache.read(key);
        let node = node.borrow();
        if node.is_leaf() {
            node.values.is_empty()
        } else {
            match node.level {
                1 => node.keys.is_empty(),
                _ => node.keys.len() == 1 && node.keys[0] == 0,
            }
        }
    }

    pub(crate) fn new(base_key: u64) -> Self {
        Self {
            cache: Cache::new(base_key, true),
        }
    }

    pub(crate) fn open(base_key: u64) -> Self {
        Self {
            cache: Cache::new(base_key, false),
        }
    }

    fn get_elements(&self) -> usize {
        self.cache.read(ROOT_KEY).borrow().rank()
    }

    pub fn len(&self) -> usize {
        self.get_elements()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        // Discard the local changes
        self.cache.reset();
        // Loads all the nodes to the cache
        self.load(ROOT_KEY);
        self.cache.clear();
    }

    pub fn get_mut(&mut self, index: usize) -> Option<ProxyMut<'_, V>> {
        match index.cmp(&self.get_elements()) {
            Ordering::Less => Some(self.get_mut_internal(index, ROOT_KEY)),
            _ => None,
        }
    }

    pub fn get(&self, index: usize) -> Option<Proxy<'_, V>> {
        match index.cmp(&self.get_elements()) {
            Ordering::Less => Some(self.get_internal(index, ROOT_KEY)),
            _ => None,
        }
    }

    pub fn push(&mut self, value: V) {
        // INFO Leaves are NOT allowed to grow beyond order when pushing new elements
        if let Some(new_subtree) = self.push_internal(value, ROOT_KEY) {
            self.update_root(new_subtree);
        }
    }

    pub fn pop(&mut self) -> Option<V> {
        if self.is_empty() {
            return None;
        }
        let (value, _) = self.pop_internal(ROOT_KEY);

        // Prune the root node to keep the tree integrity
        self.prune_root();

        Some(value)
    }

    pub fn insert(&mut self, index: usize, element: V) {
        match index.cmp(&self.get_elements()) {
            Ordering::Greater => {
                panic!("Out of bounds");
            }
            Ordering::Equal => {
                // For higher efficiency
                self.push(element);
            }
            Ordering::Less => {
                // INFO Leaves are allowed to grow beyond order when inserting new elements
                if let Some(leaf_key) = self.insert_internal(index, element, ROOT_KEY) {
                    let root = self.cache.read(ROOT_KEY);
                    // Create a whole subtree with the new leaf
                    let subtree_key = self.create_subtree(root.borrow().level(), leaf_key);
                    self.update_root(subtree_key);
                }
            }
        }
    }

    pub fn remove(&mut self, index: usize) -> V {
        match index.cmp(&self.get_elements()) {
            Ordering::Greater => {
                panic!("Out of bounds");
            }
            Ordering::Equal => {
                // For higher efficiency
                match self.pop() {
                    None => panic!("Out of bounds"),
                    Some(value) => value,
                }
            }
            Ordering::Less => {
                let (_empty, val) = self.remove_internal(index, ROOT_KEY);

                // Prune the root node to keep the tree integrity
                self.prune_root();
                val
            }
        }
    }

    fn get_mut_internal(&mut self, index: usize, node_key: u64) -> ProxyMut<'_, V> {
        debug_assert!(index < self.get_elements(), "index out of bounds");
        let node = self.cache.read(node_key);

        if node.borrow().is_leaf() {
            // NOTE: Mark the node as changed, because the user could totally do that.
            self.cache.flag_write(node_key);
            ProxyMut {
                node_ptr: node,
                elem_idx: index,
                _back_ref: PhantomData,
            }
        } else {
            // Traverse to the correct leaf
            let (tgt_index, acc) = Self::select_child(&node.borrow().keys, index);
            let new_index = index.saturating_sub(acc);
            self.get_mut_internal(new_index, node.borrow().children[tgt_index])
        }
    }

    fn get_internal(&self, index: usize, node_key: u64) -> Proxy<'_, V> {
        debug_assert!(index < self.get_elements(), "index out of bounds");
        let node = self.cache.read(node_key);

        if node.borrow().is_leaf() {
            Proxy {
                node_ptr: node,
                elem_idx: index,
                _back_ref: PhantomData,
            }
        } else {
            // Traverse to the correct leaf
            let (tgt_index, acc) = Self::select_child(&node.borrow().keys, index);
            let new_index = index.saturating_sub(acc);
            self.get_internal(new_index, node.borrow().children[tgt_index])
        }
    }

    fn push_internal(&mut self, value: V, node_key: u64) -> Option<u64> {
        let node = self.cache.read(node_key);
        let mut node = node.borrow_mut();

        if node.is_leaf() {
            // Create new leaf node when the order constraint is exceeded
            if node.values.len() >= ORDER {
                let new_leaf = Node::generate_leaf(ORDER, vec![value], node.next);
                // Store newly created leaf into the DB
                let new_key = self.new_key();
                self.cache.write(new_key, new_leaf);
                // Store updated leaf into the DB
                node.next = Some(new_key); // <- mutation here
                self.cache.flag_write(node_key);
                return Some(new_key);
            }
            node.values.push(value); // <- mutation here
        } else {
            // Traverse down to the last child
            let last_child = *node.children.last().unwrap();
            match self.push_internal(value, last_child) {
                Some(key) => {
                    // Create new internal node when the order constraint is exceeded
                    if node.keys.len() >= ORDER {
                        let new_int = Node::generate_internal(ORDER, node.level, key);
                        // Store newly created internal node into the DB
                        let new_key = self.new_key();
                        self.cache.write(new_key, new_int);
                        return Some(new_key);
                    }
                    // The new node contains only 1 element
                    node.keys.push(1); // <- mutation here
                    node.children.push(key); // <- mutation here
                }
                None => {
                    if let Some(last_key) = node.keys.last_mut() {
                        *last_key = last_key.saturating_add(1);
                    }
                }
            }
        }
        self.cache.flag_write(node_key);
        None
    }

    fn pop_internal(&mut self, node_key: u64) -> (V, bool) {
        let node = self.cache.read(node_key);
        let mut node = node.borrow_mut();

        if node.is_leaf() {
            let value = node.values.pop().unwrap(); // <- mutation here
            let empty = node.values.is_empty();
            self.cache.flag_write(node_key);
            (value, empty)
        } else {
            // Traverse down to the last child
            let (value, empty) = self.pop_internal(*node.children.last().unwrap());

            if empty {
                // Delete empty subtree
                node.keys.pop();
                self.cache.remove(node.children.pop().unwrap());

                // Shadow the variable
                let empty = node.keys.is_empty();
                self.cache.flag_write(node_key);

                if !empty {
                    drop(node);
                    // Update the next field of the last leaf to None
                    self.reset_next(node_key);
                }
                (value, empty)
            } else {
                if let Some(last_key) = node.keys.last_mut() {
                    *last_key = last_key.saturating_sub(1);
                }
                self.cache.flag_write(node_key);
                (value, false)
            }
        }
    }

    fn insert_internal(&mut self, index: usize, element: V, node_key: u64) -> Option<u64> {
        let node = self.cache.read(node_key);
        let mut node = node.borrow_mut();
        let mut result = None;

        if node.is_leaf() {
            node.values.insert(index, element);

            // Split leaf when threshold is hit
            if node.values.len() >= ORDER * 2 {
                let new_leaf = Node::generate_leaf(ORDER, node.values.split_off(ORDER), node.next);
                // Store newly created leaf into the DB
                let new_key = self.new_key();
                self.cache.write(new_key, new_leaf);
                node.next = Some(new_key);

                // Propagate upwards the new leaf
                result = Some(new_key);
            }
            self.cache.flag_write(node_key);
            return result;
        }

        // Traverse to the correct leaf
        let (tgt_index, acc) = Self::select_child(&node.keys, index);
        let new_index = index.saturating_sub(acc);

        let child_idx = node.children[tgt_index];
        let leaf = match self.insert_internal(new_index, element, child_idx) {
            Some(leaf) => leaf,
            None => {
                // No leaf splitting happened
                node.keys[tgt_index] = node.keys[tgt_index].saturating_add(1);
                self.cache.flag_write(node_key);
                return None;
            }
        };

        let level = node.level;
        match level {
            1 => {
                // Internal node to leaf handle

                // Update key of recently split leaf
                node.keys[tgt_index] = ORDER;
                // Insert new leaf
                let pos = tgt_index.saturating_add(1);
                node.keys.insert(pos, ORDER);
                node.children.insert(pos, leaf);

                // Triggers propagation mechanism
                if node.keys.len() > ORDER {
                    node.keys.pop();
                    result = node.children.pop();
                }
                self.cache.flag_write(node_key);
            }
            _ => {
                // Internal node to internal node handle
                let start = tgt_index.saturating_add(1);

                drop(node);
                // Trigger propagation to keep elements sorted
                result = self.propagate(node_key, leaf, start);
            }
        }
        result
    }

    fn remove_internal(&mut self, index: usize, node_key: u64) -> (bool, V) {
        let node = self.cache.read(node_key);
        let mut node = node.borrow_mut();

        if node.is_leaf() {
            let val = node.values.remove(index);
            let empty = node.values.is_empty();
            self.cache.flag_write(node_key);
            return (empty, val);
        }

        // Traverse to the correct leaf
        let (tgt_index, acc) = Self::select_child(&node.keys, index);
        let new_index = index.saturating_sub(acc);

        let (empty, value) = self.remove_internal(new_index, node.children[tgt_index]);

        if empty {
            match node.level {
                1 => {
                    // Get rid of empty leaf
                    node.keys.remove(tgt_index);
                    self.cache.remove(node.children.remove(tgt_index));
                }
                _ => {
                    drop(node);
                    // Trigger compacting algorithm
                    self.compact(node_key, tgt_index, true);
                    return (empty, value);
                }
            }
        } else {
            // Update keys in the current node so its indexing system is aligned
            node.keys[tgt_index] = node.keys[tgt_index].saturating_sub(1);
        }
        self.cache.flag_write(node_key);
        (empty, value)
    }

    fn update_root(&mut self, child_key: u64) {
        let former_root = self.cache.read(ROOT_KEY);
        let node = former_root.borrow();

        // Generate a new key for the former root
        let key_old_root = self.new_key();

        // Create the new root node
        let new_root = Node {
            keys: vec![node.rank(), self.get_node_rank(child_key)],
            level: node.level().saturating_add(1),
            children: vec![key_old_root, child_key],
            values: vec![],
            next: None,
        };
        drop(node);
        // Store the former root in a new position in the DB
        self.cache.replace(ROOT_KEY, key_old_root, former_root);
        // Update the new root
        self.cache.write(ROOT_KEY, new_root);
    }

    fn propagate(&mut self, node_key: u64, leaf_key: u64, start: usize) -> Option<u64> {
        let node = self.cache.read(node_key);
        let mut node = node.borrow_mut();
        // NOTE: This is more of an assertion, and less of a case for "unreachable!" (because it is in fact reachable)
        assert!(!node.is_leaf(), "Leaves are not supported");

        let popped_leaf = match node.level {
            1 => {
                // Insert propagated leaf to the first position
                node.children.insert(0, leaf_key);
                node.keys.insert(0, self.get_node_rank(leaf_key));

                // Do not pop the leaf at the tail of the tree if it fits
                // (doing so would trigger a new root creation)
                let last_key = node.children.last().cloned().unwrap();
                let last_child = self.cache.read(last_key);
                let last_child = last_child.borrow();

                if last_child.next.is_none() && node.keys.len() <= ORDER {
                    None
                } else {
                    node.keys.pop();
                    Some(node.children.pop().unwrap())
                }
            }
            _ => {
                let mut current = Some(leaf_key);

                for i in start..node.children.len() {
                    current = self.propagate(node.children[i], current.unwrap(), 0);
                    if current.is_none() {
                        break;
                    }
                }

                // Ensures the tree remains balanced during insertions
                if current.is_some() {
                    // if the current internal node is underpopulated
                    if node.keys.len() < ORDER {
                        let leaf_key = current.take().unwrap();
                        let subtree = self.create_subtree(node.level().saturating_sub(1), leaf_key);
                        node.keys.push(self.get_node_rank(subtree));
                        node.children.push(subtree);
                    }
                }

                // Update the key system after propagation
                for i in 0..node.keys.len() {
                    node.keys[i] = self.get_node_rank(node.children[i]);
                }
                current
            }
        };
        self.cache.flag_write(node_key);
        popped_leaf
    }

    fn compact(&mut self, node_key: u64, start: usize, origin: bool) -> Option<u64> {
        let node = self.cache.read(node_key);
        let mut node = node.borrow_mut();
        // NOTE: This is more of an assertion, and less of a case for "unreachable!" (because it is in fact reachable)
        assert!(!node.is_leaf(), "Leaves are not supported");

        let popped_leaf = match node.level {
            1 => {
                node.keys.remove(0);
                Some(node.children.remove(0))
            }
            _ => {
                let start = start.saturating_add(1);
                let end = node.children.len();

                // Compacting algorithm
                for i in start..end {
                    match self.compact(node.children[i], 0, false) {
                        Some(key) => self.append(node.children[i.saturating_sub(1)], key),
                        None => break,
                    }
                }

                // Fix situation where the rightmost child becomes empty
                if let Some(last_child) = node.children.last() {
                    if self.is_node_empty(*last_child) {
                        node.keys.pop();
                        node.children.pop();
                    }
                }

                match origin {
                    true => None,
                    false => self.compact(node.children[0], 0, false),
                }
            }
        };

        // Update the key system after compacting the tree
        for i in 0..node.keys.len() {
            node.keys[i] = self.get_node_rank(node.children[i]);
        }

        self.cache.flag_write(node_key);
        popped_leaf
    }

    fn prune_root(&mut self) {
        let root = self.cache.read(ROOT_KEY);
        let mut root = root.borrow_mut();

        if !root.is_leaf() {
            // Prune the rightmost subtree if it becomes empty
            if root.keys.last().unwrap() == &0 {
                root.keys.pop();
                root.children.pop();
                // Reflect the root changes to the DB
                self.cache.flag_write(ROOT_KEY);
            }
            // A root with only one subtree is useless
            if root.keys.len() == 1 {
                let child_key = root.children[0];
                let child = self.cache.read(child_key);
                // Replace root with its only child
                self.cache.replace(child_key, ROOT_KEY, child);
            }
        }
    }

    fn append(&mut self, node_key: u64, leaf_key: u64) {
        let node = self.cache.read(node_key);
        let mut node = node.borrow_mut();
        // NOTE: This is more of an assertion, and less of a case for "unreachable!" (because it is in fact reachable)
        assert!(!node.is_leaf(), "Leaves are not supported");

        match node.level {
            1 => {
                node.children.push(leaf_key);
                node.keys.push(self.get_node_rank(leaf_key));
            }
            _ => {
                // Invariant: internal nodes will never be empty
                self.append(*node.children.last().unwrap(), leaf_key);
            }
        }

        // Update the key system after appending the leaf
        for i in 0..node.keys.len() {
            node.keys[i] = self.get_node_rank(node.children[i]);
        }
        self.cache.flag_write(node_key);
    }

    fn create_subtree(&mut self, tgt_level: usize, node_key: u64) -> u64 {
        let mut current_key = node_key;
        // Create a whole subtree with the new leaf up until the root level
        for level in 1..=tgt_level {
            // Read current node
            let current_node = self.cache.read(current_key);
            let current_node = current_node.borrow();
            // Create a new internal node
            let new_int = Node {
                keys: vec![current_node.rank()],
                children: vec![current_key],
                level,
                values: vec![],
                next: None,
            };
            // Store the newly created internal node into the DB
            let new_key = self.new_key();
            self.cache.write(new_key, new_int);
            // Update the control key
            current_key = new_key;
        }
        // Return the top level new internal node key
        current_key
    }

    fn select_child(keys: &[usize], index: usize) -> (usize, usize) {
        let len = keys.len();

        let mut accumulative: usize = 0;
        for (i, key) in keys.iter().enumerate().take(len) {
            if index < accumulative.saturating_add(*key) {
                return (i, accumulative);
            }
            accumulative = accumulative.saturating_add(*key);
        }
        // NOTE: This is a case for unreachable!, as the loop should always return, if everything is correct
        unreachable!("Insert_internal() and remove_internal() always use a valid index")
    }

    pub fn exists(&self) -> bool {
        self.cache.exists()
    }

    fn new_key(&mut self) -> u64 {
        self.cache.new_key()
    }

    fn reset_next(&mut self, start: u64) {
        let mut node = self.cache.read(start);
        let mut leaf_key = u64::MAX;
        // Traverse down to the last leaf of the tree
        while !node.borrow().is_leaf() {
            // Invariant: internal nodes will always contain at least 1 child
            let last_child = *node.borrow().children.last().unwrap();
            node = self.cache.read(last_child);
            leaf_key = last_child;
        }

        // Set the last leaf next to None
        node.borrow_mut().next = None;
        self.cache.flag_write(leaf_key);
    }

    // Fetches all the nodes from the DB, loading them in the cache
    fn load(&mut self, key: u64) {
        let node = self.cache.read(key);
        for child in node.borrow().children.iter() {
            self.load(*child);
        }
    }

    pub fn iter(&self) -> LazyVecIt<V> {
        LazyVecIt::new(self)
    }

    // TODO Who needs mutable iterators anyway..
    // pub fn iter_mut(&mut self) -> LazyVecItMut<V, ORDER, BASE_KEY> {
    //     LazyVecItMut::new(self)
    // }
}
