use std::ops::Deref;
use std::{
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    sync::Arc,
    thread,
};

/// Part of the [LockFreeBinaryTree]. Used to build the binary tree with
/// TODO: we need to have a thread-safe reference count somewhere to make sure
/// that a thread cannot modify the pointers while others are reading them.
struct Node<T> {
    pub data: AtomicPtr<T>,
    pub left: AtomicPtr<Node<T>>,
    pub right: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    pub fn leaky<U: Into<Box<T>>>(item: U) -> *mut Self {
        let left = AtomicPtr::new(std::ptr::null_mut::<Node<T>>());
        let right = AtomicPtr::new(std::ptr::null_mut::<Node<T>>());

        let raw = Box::into_raw(item.into());
        let boxed = Box::from(Self {
            data: AtomicPtr::new(raw),
            left,
            right,
        });

        Box::into_raw(boxed)
    }
}

/// structure that guards the immutability lifetime of the contained node. As long an instance of
/// this guard exists no other threads can modify the node.
pub struct DerefGuard<'a, T> {
    phantom: std::marker::PhantomData<&'a T>,
    node: *const Node<T>,
}

impl<'a, T> DerefGuard<'a, T> {
    fn new(node: *const Node<T>) -> Self {
        Self {
            phantom: std::marker::PhantomData::default(),
            node,
        }
    }
}

impl<'a, T> Deref for DerefGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.node).data.load(Ordering::Acquire) }
    }
}

pub struct LockFreeBinaryTree<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T: Send + std::cmp::Ord> LockFreeBinaryTree<T> {
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    pub fn push<U: Into<Box<T>>>(&self, item: U) -> bool {
        let new_node = Node::leaky(item);
        loop {
            let current = self.head.load(Ordering::Relaxed);
            if current.is_null() {
                match self.head.compare_exchange(
                    current,
                    new_node,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    //other thread was quicker, proceed with adding it
                    Err(_) => continue,
                }
            } else {
                unsafe {
                    let data = (&*new_node).data.load(Ordering::Acquire);
                    let _found_node = self.relaxed_search(current, &*data);
                    todo!();
                }
            }
        }
        return true;
    }

    /// Find the given element in the container and return a thread-safe
    /// immutable reference to this element, If the item does not exist, return
    /// the item closest.
    ///
    /// # Arguments
    ///
    /// * 'item'    item to find in the container
    ///
    /// # Returns
    ///
    /// A immutable reference to the found item wrapped in a guard. This guard
    /// makes sure no writes happen as long as the guard object exists.
    pub fn find<'a>(&self, item: &T) -> DerefGuard<'a, T> {
        let (node, _) = self.relaxed_search(self.head.load(Ordering::Acquire), item);
        DerefGuard::new(node)
    }

    pub fn delete(&self, item: &T) -> bool {
        todo!()
    }

    /// This function walks over the tree to find the node that is closest to the
    /// given value. Note: The tree can get modified while we are walking over it. 
    ///
    /// # Arguments
    ///
    /// * 'current' the root node to start searching with. Cannot be null!
    /// * 'value' the value to find
    ///
    /// # Return
    ///
    /// Pointer to the value closest to the given value.
    fn relaxed_search(&self, current: *const Node<T>, to_find: &T) -> (*const Node<T>, std::cmp::Ordering) {
        assert!(!current.is_null());

        unsafe {
            let node_data = (*current).data.load(Ordering::Acquire);
            // data pointers cannot be invalid!
            assert!(!node_data.is_null());

            let ordering = (&*node_data).cmp(to_find);
            let new_current = match ordering {
                std::cmp::Ordering::Less => (*current).left.load(Ordering::Relaxed),
                std::cmp::Ordering::Greater => (*current).right.load(Ordering::Relaxed),
                std::cmp::Ordering::Equal => std::ptr::null_mut(),
            };

            if new_current.is_null() {
                return (current, ordering);
            }

        (new_current, ordering)
        }
    }
}

#[test]
fn test_push() {
    loom::model(|| {
        const THREAD_N: usize = loom::MAX_THREADS - 1;
        let queue: Arc<LockFreeBinaryTree<usize>> = Arc::new(LockFreeBinaryTree::new());
        let threads: Vec<_> = (0..THREAD_N)
            .map(|n| {
                let clone = queue.clone();
                thread::spawn(move || {
                    assert!(clone.push(n));
                })
            })
            .collect();

        for handle in threads {
            handle.join().unwrap();
        }
    });
}

#[test]
fn push_test() {
    let tree = LockFreeBinaryTree::<usize>::new();
    assert!(tree.push(123));
    unsafe {
        let head = &*tree.head.load(Ordering::Relaxed);
        assert_eq!(123, *head.data.load(Ordering::Relaxed));
    }
}
