use std::{cell::Cell, ops::Deref, ptr::NonNull};

pub struct InnerHandle<T: ?Sized> {
    pub(crate) marked: Cell<bool>,
    pub(crate) value: Box<T>,
}

impl<T: ?Sized> InnerHandle<T> {
    pub fn mark(&self) {
        self.marked.set(true);
    }

    pub unsafe fn unmark(&self) {
        self.marked.set(false);
    }
}

#[derive(Debug)]
pub struct Handle<T: ?Sized>(NonNull<InnerHandle<T>>);

impl<T: ?Sized> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T: ?Sized> Handle<T> {
    pub unsafe fn new(ptr: NonNull<InnerHandle<T>>) -> Self {
        Handle(ptr)
    }
}

impl<T: ?Sized> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.0.as_ref().value }
    }
}
