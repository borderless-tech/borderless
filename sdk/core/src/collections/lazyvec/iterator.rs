use super::proxy::Proxy;
use super::LazyVec;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Immutable B+Tree Iterator
pub struct LazyVecIt<'a, V> {
    tree: &'a LazyVec<V>,
    global_idx: usize,
}

impl<'a, V> Iterator for LazyVecIt<'a, V>
where
    V: Serialize + DeserializeOwned + Clone,
{
    type Item = Proxy<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.tree.get(self.global_idx);
        self.global_idx = self.global_idx.saturating_add(1);
        out
    }
}

impl<'a, V> LazyVecIt<'a, V>
where
    V: Serialize + DeserializeOwned + Clone,
{
    pub(crate) fn new(tree: &'a LazyVec<V>) -> Self {
        LazyVecIt {
            tree,
            global_idx: 0,
        }
    }
}

// /// Mutable B+Tree Iterator
// pub struct LazyVecItMut<'a, V> {
//     tree: &'a mut LazyVec<V>,
//     global_idx: usize,
// }

// impl<'a, V> Iterator for LazyVecItMut<'a, V>
// where
//     V: Serialize + DeserializeOwned + Clone,
// {
//     type Item = ProxyMut<'a, V>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let out = self.tree.get_mut(self.global_idx);
//         self.global_idx = self.global_idx.saturating_add(1);
//         out
//     }
// }

// impl<'a, V> LazyVecItMut<'a, V>
// where
//     V: Serialize + DeserializeOwned + Clone,
// {
//     pub(crate) fn new(tree: &'a mut LazyVec<V>) -> Self {
//         LazyVecItMut {
//             tree,
//             global_idx: 0,
//         }
//     }
// }
