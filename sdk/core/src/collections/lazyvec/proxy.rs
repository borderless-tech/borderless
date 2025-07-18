use super::node::Node;

use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

pub struct Proxy<'a, V> {
    pub(super) node_ptr: Rc<RefCell<Node<V>>>,
    pub(super) elem_idx: usize,
    pub(super) _back_ref: PhantomData<&'a V>, // <- prevents the tree from being borrowed mutably, while a proxy object exists
}

impl<'a, V> AsRef<V> for Proxy<'a, V> {
    fn as_ref(&self) -> &V {
        // TODO - check if this causes UB !
        let p = unsafe { &*self.node_ptr.as_ptr() };
        &p.values[self.elem_idx]
    }
}

impl<'a, V> Deref for Proxy<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, V> Borrow<V> for Proxy<'a, V> {
    fn borrow(&self) -> &V {
        self.as_ref()
    }
}

pub struct ProxyMut<'a, V> {
    pub(super) node_ptr: Rc<RefCell<Node<V>>>,
    pub(super) elem_idx: usize,
    pub(super) _back_ref: PhantomData<&'a mut V>, // <- prevents the tree from being borrowed, while a proxy object exists
}

impl<'a, V> AsRef<V> for ProxyMut<'a, V> {
    fn as_ref(&self) -> &V {
        // TODO - check if this causes UB !
        let p = unsafe { &mut *self.node_ptr.as_ptr() };
        &p.values[self.elem_idx]
    }
}

impl<'a, V> AsMut<V> for ProxyMut<'a, V> {
    fn as_mut(&mut self) -> &mut V {
        // TODO - check if this causes UB !
        let p = unsafe { &mut *self.node_ptr.as_ptr() };
        &mut p.values[self.elem_idx]
    }
}

impl<'a, V> Deref for ProxyMut<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, V> DerefMut for ProxyMut<'a, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<'a, V> Borrow<V> for ProxyMut<'a, V> {
    fn borrow(&self) -> &V {
        self.as_ref()
    }
}

impl<'a, V> BorrowMut<V> for ProxyMut<'a, V> {
    fn borrow_mut(&mut self) -> &mut V {
        self.as_mut()
    }
}
