use loom::{
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
    thread,
};
use std::ops::Deref;

/// This object operates on an homogeneous piece of memory.
/// as one would expect, this object is threadsafe. And is written for learning purposes
/// Internally it uses an binary tree structure, this means you can expect :
/// * insert O=Log(n)
/// * pop O= 1
///
pub struct LockFreeBinaryTree<T, const N: usize> {
    storage: [T; N],
    head: AtomicUsize,
}

//unsafe impl<T, const N: usize> Sync for LockFreeBinaryTree<T, N> {
//}

impl<T: Default + Copy + Sync + Send + std::fmt::Debug, const N: usize> LockFreeBinaryTree<T, N> {
    pub fn new() -> Self {
        Self {
            storage: [T::default(); N],
            head: AtomicUsize::new(0),
        }
    }

    fn mut_ptr(&self) -> *mut [T; N] {
        &self.storage as *const _ as *mut _
    }

    fn as_ptr(&self) -> *const [T; N] {
        &self.storage as *const _ 
    }

    pub fn push(&self, val: T) -> bool {
        let index = self.head.fetch_add(1, Ordering::Relaxed);
        if index >= N {
            // no more space left!
            return false;
        }

        unsafe {
            let storage = &mut *self.mut_ptr();
            storage[index] = val;
        }
        return true;
    }

    pub fn pop(&self) -> Option<T> {
        // self.head.fetch_max()
        // head can be empty, if we are in push operation
        todo!();
    }
}

#[test]
fn test_push() {
    loom::model(|| {
        const thread_n: usize = loom::MAX_THREADS - 1;
        let queue: Arc<LockFreeBinaryTree<usize, thread_n>> = Arc::new(LockFreeBinaryTree::new());
        let threads: Vec<_> = (0..thread_n)
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

        unsafe {
            let storage = &*queue.deref().mut_ptr();
        }
    });
}
