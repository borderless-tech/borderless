use super::proxy::Proxy;
use serde::{Deserialize, Serialize};

use super::lazyvec::LazyVec;

/// Immutable B+Tree Iterator
pub struct LazyVecIt<'a, V, const ORDER: usize, const BASE_KEY: u64> {
    tree: &'a LazyVec<V, ORDER, BASE_KEY>,
    global_idx: usize,
}

impl<'a, V, const ORDER: usize, const BASE_KEY: u64> Iterator for LazyVecIt<'a, V, ORDER, BASE_KEY>
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

impl<'a, V, const ORDER: usize, const BASE_KEY: u64> LazyVecIt<'a, V, ORDER, BASE_KEY>
where
    V: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone,
{
    pub(crate) fn new(tree: &'a LazyVec<V, ORDER, BASE_KEY>) -> Self {
        LazyVecIt {
            tree,
            global_idx: 0,
        }
    }
}

// /// Mutable B+Tree Iterator
// pub struct LazyVecItMut<'a, V, const ORDER: usize, const BASE_KEY: u64> {
//     tree: &'a mut LazyVec<V, ORDER, BASE_KEY>,
//     global_idx: usize,
// }

// impl<'a, V, const ORDER: usize, const BASE_KEY: u64> Iterator
//     for LazyVecItMut<'a, V, ORDER, BASE_KEY>
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

// impl<'a, V, const ORDER: usize, const BASE_KEY: u64> LazyVecItMut<'a, V, ORDER, BASE_KEY>
// where
//     V: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone,
// {
//     pub(crate) fn new(tree: &'a mut LazyVec<V, ORDER, BASE_KEY>) -> Self {
//         LazyVecItMut {
//             tree,
//             global_idx: 0,
//         }
//     }
// }
