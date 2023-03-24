#![allow(unused)]

use bitflags::bitflags;
use std::borrow::Borrow;
use std::cell::Cell;
use std::ops::Deref;
use std::ptr::NonNull;

use crate::value::object::Object;

use self::handle::GcNode;
use self::handle::Handle;

pub mod handle;
pub mod persistent;
pub mod trace;

pub struct Gc {
    /// The very first node of this [`Gc`]
    head: Option<NonNull<GcNode<dyn Object>>>,
    /// The last-inserted node of this [`Gc`]
    tail: Option<NonNull<GcNode<dyn Object>>>,
    node_count: usize,
}

impl Default for Gc {
    fn default() -> Self {
        Self::new()
    }
}

impl Gc {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
            node_count: 0,
        }
    }

    fn add(&mut self, value: Box<GcNode<dyn Object>>) -> Handle<dyn Object> {
        let ptr = NonNull::new(Box::into_raw(value)).unwrap();

        // insert head if this is the very first node
        if self.head.is_none() {
            self.head = Some(ptr);
        }

        // set the `next` pointer of the current last element in the list to this pointer
        if let Some(tail) = &mut self.tail {
            unsafe {
                tail.as_mut().next = Some(ptr);
            }
        }

        self.tail = Some(ptr);
        self.node_count += 1;

        unsafe { Handle::from_raw(ptr) }
    }

    /// # Safety
    /// Calling this function while there are unmarked, live [`Handle`]s is Undefined Behavior.
    /// Any unmarked node is deallocated during a sweep cycle.
    pub unsafe fn sweep(&mut self) {
        // The last valid pointer that was found
        let mut previous = None;
        let mut cur = self.head;

        while let Some(ptr) = cur {
            let GcNode {
                flags,
                refcount,
                next,
                ..
                // value,
            } = unsafe { ptr.as_ref() };

            cur = *next;

            if !flags.is_marked() && refcount.get() == 0 {
                // Reference did not get marked during mark phase
                // Deallocate and unlink!

                // If this node is the head (i.e. oldest/first node) or there is no head,
                // set it to the next node.
                if self.head.map_or(true, |p| p == ptr) {
                    self.head = *next;
                }

                // If this node is the tail (i.e. newest/most recently added node) or there is no tail,
                // set it to the last valid node.
                if self.tail.map_or(true, |p| p == ptr) {
                    self.tail = previous;
                }

                // Update last valid pointer to the next pointer
                if let Some(mut previous) = previous {
                    unsafe { previous.as_mut().next = *next };
                }

                // Deallocate node.
                unsafe { drop(Box::from_raw(ptr.as_ptr())) };

                // One less node now.
                self.node_count -= 1;
            } else {
                // Node still live
                flags.unmark();
                previous = Some(ptr);
            }
        }
    }
}

impl Drop for Gc {
    fn drop(&mut self) {
        let mut curr = self.head;
        while let Some(node) = curr {
            let next = unsafe { node.as_ref().next };
            curr = next;

            unsafe {
                drop(Box::from_raw(node.as_ptr()));
            }
        }
    }
}

macro_rules! register_gc {
    ($gc:expr, $val:expr) => {{
        let value = $val;
        let node = GcNode {
            flags: Default::default(),
            refcount: Default::default(),
            next: None,
            value,
        };
        $gc.add(Box::new(node))
    }};
}

/// # Safety
/// Implementors must provide a "correct" into_handle method
/// by returning a valid [`Handle`] living in the given linked list.
pub unsafe trait IntoHandle {
    fn into_handle(self, gc: &mut Gc) -> Handle<dyn Object>;
}

unsafe impl<T: Object + 'static> IntoHandle for T {
    fn into_handle(self, gc: &mut Gc) -> Handle<dyn Object> {
        register_gc!(gc, self)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Display;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn gc_works() {
        unsafe {
            let mut gc = Gc::new();

            assert!(gc.node_count == 0);
            assert!(gc.head.is_none());
            assert!(gc.tail.is_none());

            let h1 = register_gc!(gc, 123.0);

            assert!(gc.head == NonNull::new(h1.as_ptr()));
            assert!(gc.tail == NonNull::new(h1.as_ptr()));
            assert!((*h1.as_ptr()).next.is_none());
            assert!(!(*h1.as_ptr()).flags.is_marked());
            assert!(gc.node_count == 1);

            let h2 = register_gc!(gc, Rc::from("hi"));

            assert!(gc.head == NonNull::new(h1.as_ptr()));
            assert!(gc.tail == NonNull::new(h2.as_ptr()));
            assert!((*h1.as_ptr()).next == NonNull::new(h2.as_ptr()));
            assert!(!(*h2.as_ptr()).flags.is_marked());
            assert!(gc.node_count == 2);

            (*h1.as_ptr()).flags.mark();
            (*h2.as_ptr()).flags.mark();

            assert!((*h1.as_ptr()).flags.is_marked());
            assert!((*h2.as_ptr()).flags.is_marked());

            gc.sweep();

            // nothing should have changed after GC sweep since all nodes were marked
            // they should be unmarked now though
            assert!(gc.head == NonNull::new(h1.as_ptr()));
            assert!(gc.tail == NonNull::new(h2.as_ptr()));
            assert!((*h1.as_ptr()).next == NonNull::new(h2.as_ptr()));
            assert!(!(*h1.as_ptr()).flags.is_marked());
            assert!(!(*h2.as_ptr()).flags.is_marked());
            assert!(gc.node_count == 2);

            // add a third node now
            let h3 = register_gc!(gc, true);

            assert!(gc.head == NonNull::new(h1.as_ptr()));
            assert!(gc.tail == NonNull::new(h3.as_ptr()));
            assert!((*h1.as_ptr()).next == NonNull::new(h2.as_ptr()));
            assert!((*h2.as_ptr()).next == NonNull::new(h3.as_ptr()));
            assert!(!(*h3.as_ptr()).flags.is_marked());
            assert!(gc.node_count == 3);

            // only mark second node
            (*h2.as_ptr()).flags.mark();

            gc.sweep();

            // only one node is left: h2
            assert!(gc.node_count == 1);
            assert!(gc.head == NonNull::new(h2.as_ptr()));
            assert!(gc.tail == NonNull::new(h2.as_ptr()));

            // final sweep
            gc.sweep();

            // nothing left.
            assert!(gc.node_count == 0);
            assert!(gc.head.is_none());
            assert!(gc.tail.is_none());

            // lastly, test if Gc::drop works correctly. run under miri to see possible leaks
            register_gc!(gc, Rc::from("test"));
        }
    }
}
