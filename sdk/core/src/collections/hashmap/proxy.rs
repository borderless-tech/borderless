use std::borrow::Borrow;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use super::cache::Cell;

pub struct Proxy<'a, V> {
    pub(super) value_ptr: RefCell<Rc<Cell<V>>>,
    pub(super) _back_ref: PhantomData<&'a V>, // <- prevents the tree from being borrowed mutably, while a proxy object exists
}

impl<'a, V> AsRef<V> for Proxy<'a, V> {
    fn as_ref(&self) -> &V {
        // TODO - check if this causes UB !
        let p = unsafe { &mut *self.value_ptr.as_ptr() };
        &p.value
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
