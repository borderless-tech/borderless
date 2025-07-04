use crate::collections::hashmap::KeyValue;
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

// Immutable proxy used to reference a HashMap key
pub struct Key<'a, K, V> {
    pub(super) cell_ptr: Rc<RefCell<KeyValue<K, V>>>,
    pub(super) _back_ref: PhantomData<&'a V>,
}

impl<'a, K, V> AsRef<K> for Key<'a, K, V> {
    fn as_ref(&self) -> &K {
        let p = unsafe { &*self.cell_ptr.as_ptr() };
        &p.pair.0
    }
}

impl<'a, K, V> Deref for Key<'a, K, V> {
    type Target = K;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

// Immutable proxy used to reference a HashMap value
pub struct Value<'a, K, V> {
    pub(super) cell_ptr: Rc<RefCell<KeyValue<K, V>>>,
    pub(super) _back_ref: PhantomData<&'a V>,
}

impl<'a, K, V> AsRef<V> for Value<'a, K, V> {
    fn as_ref(&self) -> &V {
        let p = unsafe { &*self.cell_ptr.as_ptr() };
        &p.pair.1
    }
}

impl<'a, K, V> Deref for Value<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

// Mutable proxy used to reference a HashMap value
pub struct ValueMut<'a, K, V> {
    pub(super) cell_ptr: Rc<RefCell<KeyValue<K, V>>>,
    pub(super) _back_ref: PhantomData<&'a mut V>, // <- prevents the tree from being borrowed, while a proxy object exists
}

impl<'a, K, V> AsRef<V> for ValueMut<'a, K, V> {
    fn as_ref(&self) -> &V {
        let p = unsafe { &*self.cell_ptr.as_ptr() };
        &p.pair.1
    }
}

impl<'a, K, V> AsMut<V> for ValueMut<'a, K, V> {
    fn as_mut(&mut self) -> &mut V {
        let p = unsafe { &mut *self.cell_ptr.as_ptr() };
        &mut p.pair.1
    }
}

impl<'a, K, V> Deref for ValueMut<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, K, V> DerefMut for ValueMut<'a, K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

pub struct Entry<'a, K, V> {
    pub(super) cell_ptr: Rc<RefCell<KeyValue<K, V>>>,
    pub(super) _back_ref: PhantomData<&'a V>, // <- prevents the tree from being borrowed mutably, while a proxy object exists
}

impl<'a, K, V> AsRef<(K, V)> for Entry<'a, K, V> {
    fn as_ref(&self) -> &(K, V) {
        let p = unsafe { &*self.cell_ptr.as_ptr() };
        &p.pair
    }
}

impl<'a, K, V> Deref for Entry<'a, K, V> {
    type Target = (K, V);

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/*
impl<'a, K, V> Borrow<V> for Entry<'a, K, V> {
    fn borrow(&self) -> &V {
        self.as_ref()
    }
}
*/

impl<'a, K, V> Entry<'a, K, V> {
    pub fn key(&self) -> &K {
        let p = unsafe { &*self.cell_ptr.as_ptr() };
        &p.pair.0
    }

    pub fn value(&self) -> &V {
        let p = unsafe { &*self.cell_ptr.as_ptr() };
        &p.pair.1
    }
}

pub struct ProxyMut<'a, K, V> {
    pub(super) cell_ptr: Rc<RefCell<KeyValue<K, V>>>,
    pub(super) _back_ref: PhantomData<&'a mut V>, // <- prevents the tree from being borrowed, while a proxy object exists
}

impl<'a, K, V> AsRef<V> for ProxyMut<'a, K, V> {
    fn as_ref(&self) -> &V {
        let p = unsafe { &mut *self.cell_ptr.as_ptr() };
        &p.pair.1
    }
}

impl<'a, K, V> AsMut<V> for ProxyMut<'a, K, V> {
    fn as_mut(&mut self) -> &mut V {
        let p = unsafe { &mut *self.cell_ptr.as_ptr() };
        &mut p.pair.1
    }
}

impl<'a, K, V> Deref for ProxyMut<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, K, V> DerefMut for ProxyMut<'a, K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<'a, K, V> Borrow<V> for ProxyMut<'a, K, V> {
    fn borrow(&self) -> &V {
        self.as_ref()
    }
}

impl<'a, K, V> BorrowMut<V> for ProxyMut<'a, K, V> {
    fn borrow_mut(&mut self) -> &mut V {
        self.as_mut()
    }
}
