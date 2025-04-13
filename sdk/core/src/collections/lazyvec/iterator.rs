use super::proxy::Proxy;
use serde::{Deserialize, Serialize};

use super::lazyvec::BPlusTree;

/// Immutable B+Tree Iterator
pub struct BPlusTreeIt<'a, V, const ORDER: usize, const BASE_KEY: u64> {
    tree: &'a BPlusTree<V, ORDER, BASE_KEY>,
    global_idx: usize,
}

impl<'a, V, const ORDER: usize, const BASE_KEY: u64> Iterator
    for BPlusTreeIt<'a, V, ORDER, BASE_KEY>
where
    V: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone,
{
    type Item = Proxy<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.tree.get(self.global_idx);
        self.global_idx = self.global_idx.saturating_add(1);
        out
    }
}

impl<'a, V, const ORDER: usize, const BASE_KEY: u64> BPlusTreeIt<'a, V, ORDER, BASE_KEY>
where
    V: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone,
{
    pub(crate) fn new(tree: &'a BPlusTree<V, ORDER, BASE_KEY>) -> Self {
        BPlusTreeIt {
            tree,
            global_idx: 0,
        }
    }
}

// /// Mutable B+Tree Iterator
// pub struct BPlusTreeItMut<'a, V, const ORDER: usize, const BASE_KEY: u64> {
//     tree: &'a mut BPlusTree<V, ORDER, BASE_KEY>,
//     global_idx: usize,
// }

// impl<'a, V, const ORDER: usize, const BASE_KEY: u64> Iterator
//     for BPlusTreeItMut<'a, V, ORDER, BASE_KEY>
// where
//     V: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone,
// {
//     type Item = ProxyMut<'a, V>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let out = self.tree.get_mut(self.global_idx);
//         self.global_idx = self.global_idx.saturating_add(1);
//         out
//     }
// }

// impl<'a, V, const ORDER: usize, const BASE_KEY: u64> BPlusTreeItMut<'a, V, ORDER, BASE_KEY>
// where
//     V: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone,
// {
//     pub(crate) fn new(tree: &'a mut BPlusTree<V, ORDER, BASE_KEY>) -> Self {
//         BPlusTreeItMut {
//             tree,
//             global_idx: 0,
//         }
//     }
// }
