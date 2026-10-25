//! Shared ownership primitives. `Rc`/`RefCell` by default, `Arc`/`RwLock` with the `sync` feature.

#[cfg(not(feature = "sync"))]
mod imp {
    use std::cell::{Cell, Ref, RefCell, RefMut};

    pub type Shared<T> = std::rc::Rc<T>;
    pub type WeakShared<T> = std::rc::Weak<T>;
    pub type Lock<T> = RefCell<T>;
    pub type ReadGuard<'a, T> = Ref<'a, T>;
    pub type WriteGuard<'a, T> = RefMut<'a, T>;

    #[inline(always)]
    pub fn read<T>(lock: &Lock<T>) -> ReadGuard<'_, T> {
        lock.borrow()
    }

    #[inline(always)]
    pub fn write<T>(lock: &Lock<T>) -> WriteGuard<'_, T> {
        lock.borrow_mut()
    }

    #[inline(always)]
    pub fn try_read<T>(lock: &Lock<T>) -> Option<ReadGuard<'_, T>> {
        lock.try_borrow().ok()
    }

    #[inline(always)]
    pub fn try_write<T>(lock: &Lock<T>) -> Option<WriteGuard<'_, T>> {
        lock.try_borrow_mut().ok()
    }

    #[derive(Debug, Default)]
    pub struct RidCell(Cell<u32>);

    impl RidCell {
        pub fn new(v: u32) -> Self {
            Self(Cell::new(v))
        }
        pub fn get(&self) -> u32 {
            self.0.get()
        }
        pub fn set(&self, v: u32) {
            self.0.set(v)
        }
    }
}

#[cfg(feature = "sync")]
mod imp {
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard,
    };

    pub type Shared<T> = std::sync::Arc<T>;
    pub type WeakShared<T> = std::sync::Weak<T>;
    pub type Lock<T> = RwLock<T>;
    pub type ReadGuard<'a, T> = RwLockReadGuard<'a, T>;
    pub type WriteGuard<'a, T> = RwLockWriteGuard<'a, T>;

    #[inline(always)]
    pub fn read<T>(lock: &Lock<T>) -> ReadGuard<'_, T> {
        lock.read().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline(always)]
    pub fn write<T>(lock: &Lock<T>) -> WriteGuard<'_, T> {
        lock.write().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline(always)]
    pub fn try_read<T>(lock: &Lock<T>) -> Option<ReadGuard<'_, T>> {
        lock.try_read().ok()
    }

    #[inline(always)]
    pub fn try_write<T>(lock: &Lock<T>) -> Option<WriteGuard<'_, T>> {
        lock.try_write().ok()
    }

    #[derive(Debug, Default)]
    pub struct RidCell(AtomicU32);

    impl RidCell {
        pub fn new(v: u32) -> Self {
            Self(AtomicU32::new(v))
        }
        pub fn get(&self) -> u32 {
            self.0.load(Ordering::Relaxed)
        }
        pub fn set(&self, v: u32) {
            self.0.store(v, Ordering::Relaxed)
        }
    }
}

pub use imp::*;
