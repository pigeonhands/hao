use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
};

use crate::sync::{self, Lock, ReadGuard, RidCell, Shared, WeakShared, WriteGuard};

#[cfg_attr(feature = "reflect", derive(facet::Facet))]
struct EntryInner<T> {
    rid: RidCell,
    value: Lock<T>,
}

/// A shared handle to a metadata entry (a type, method, field, ...).
///
/// Cloning an `Entry` is cheap and produces another handle to the same value.
/// Entries compare and hash by identity.
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct Entry<T>(Shared<EntryInner<T>>);

impl<T> Entry<T> {
    pub fn new(value: T) -> Self {
        Self::with_rid(0, value)
    }

    pub(crate) fn with_rid(rid: u32, value: T) -> Self {
        Self(Shared::new(EntryInner {
            rid: RidCell::new(rid),
            value: Lock::new(value),
        }))
    }

    /// Borrows the value. Panics if it is currently mutably borrowed.
    #[inline(always)]
    pub fn get(&self) -> ReadGuard<'_, T> {
        sync::read(&self.0.value)
    }

    /// Mutably borrows the value. Panics if it is currently borrowed.
    #[inline(always)]
    pub fn get_mut(&self) -> WriteGuard<'_, T> {
        sync::write(&self.0.value)
    }

    #[inline(always)]
    pub fn try_get(&self) -> Option<ReadGuard<'_, T>> {
        sync::try_read(&self.0.value)
    }

    #[inline(always)]
    pub fn try_get_mut(&self) -> Option<WriteGuard<'_, T>> {
        sync::try_write(&self.0.value)
    }

    /// Replaces the value, returning the old one.
    pub fn replace(&self, value: T) -> T {
        std::mem::replace(&mut *self.get_mut(), value)
    }

    /// The row this entry was loaded from, or 0 if it was created after loading.
    #[inline(always)]
    pub fn rid(&self) -> u32 {
        self.0.rid.get()
    }

    #[inline(always)]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Shared::ptr_eq(&self.0, &other.0)
    }

    pub fn downgrade(&self) -> WeakEntry<T> {
        WeakEntry(Shared::downgrade(&self.0))
    }

    #[inline(always)]
    pub(crate) fn key(&self) -> usize {
        Shared::as_ptr(&self.0) as *const () as usize
    }
}

impl<T: Default> Entry<T> {
    pub(crate) fn placeholder(rid: u32) -> Self {
        Self::with_rid(rid, T::default())
    }

    pub(crate) fn clear(&self) {
        if let Some(mut v) = self.try_get_mut() {
            *v = T::default();
        }
    }
}

impl<T: Default> Default for Entry<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for Entry<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for Entry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> Eq for Entry<T> {}

impl<T> Hash for Entry<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}

impl<T> Debug for Entry<T>
where
    T: EntryName,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<T>()
            .rsplit("::")
            .next()
            .unwrap_or("?");
        match self.try_get() {
            Some(v) => write!(f, "{}({:?})", name, v.entry_name()),
            None => write!(f, "{}(<borrowed>)", name),
        }
    }
}

/// Short name used when debug printing an [`Entry`]. Keeps debug output
/// finite since entries form cycles.
pub trait EntryName {
    fn entry_name(&self) -> String;
}

/// A non-owning [`Entry`] handle, used for back references like a method's declaring type.
#[cfg_attr(feature = "reflect", derive(facet::Facet))]
pub struct WeakEntry<T>(WeakShared<EntryInner<T>>);

impl<T> WeakEntry<T> {
    pub fn new() -> Self {
        Self(WeakShared::new())
    }

    pub fn upgrade(&self) -> Option<Entry<T>> {
        self.0.upgrade().map(Entry)
    }
}

impl<T> Default for WeakEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for WeakEntry<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: EntryName> Debug for WeakEntry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.upgrade() {
            Some(e) => write!(f, "Weak({:?})", e),
            None => write!(f, "Weak(<dropped>)"),
        }
    }
}
