use crate::collections::hashmap::KeyValue;
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

pub struct Proxy<'a, K, V> {
    pub(super) cell_ptr: Rc<RefCell<KeyValue<K, V>>>,
    pub(super) _back_ref: PhantomData<&'a V>, // <- prevents the tree from being borrowed mutably, while a proxy object exists
}

impl<'a, K, V> AsRef<V> for Proxy<'a, K, V> {
    fn as_ref(&self) -> &V {
        // TODO - check if this causes UB !
        let p = unsafe { &*self.cell_ptr.as_ptr() };
        &p.value
    }
}

impl<'a, K, V> Deref for Proxy<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, K, V> Borrow<V> for Proxy<'a, K, V> {
    fn borrow(&self) -> &V {
        self.as_ref()
    }
}

impl<'a, K, V> Proxy<'a, K, V> {
    pub fn key(&self) -> &K {
        let kv = unsafe { &*self.cell_ptr.as_ptr() };
        &kv.key
    }

    pub fn value(&self) -> &V {
        let kv = unsafe { &*self.cell_ptr.as_ptr() };
        &kv.value
    }
}

pub struct ProxyMut<'a, K, V> {
    pub(super) cell_ptr: Rc<RefCell<KeyValue<K, V>>>,
    pub(super) _back_ref: PhantomData<&'a mut V>, // <- prevents the tree from being borrowed, while a proxy object exists
}

impl<'a, K, V> AsRef<V> for ProxyMut<'a, K, V> {
    fn as_ref(&self) -> &V {
        // TODO - check if this causes UB !
        let p = unsafe { &mut *self.cell_ptr.as_ptr() };
        &p.value
    }
}

impl<'a, K, V> AsMut<V> for ProxyMut<'a, K, V> {
    fn as_mut(&mut self) -> &mut V {
        // TODO - check if this causes UB !
        let p = unsafe { &mut *self.cell_ptr.as_ptr() };
        &mut p.value
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
