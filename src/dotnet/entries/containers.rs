use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::{Debug, Display},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Debug)]
pub(crate) struct MaybeUnsetEntry<T: Sized> {
    value: MaybeUninit<RowEntry<T>>,
    is_set: bool,
}

impl<T: Sized> MaybeUnsetEntry<T> {
    pub fn new_unset() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            is_set: false,
        }
    }

    pub fn assume_init_value(mut self) -> T {
        assert!(self.is_set());
        self.is_set = false;
        let value = std::mem::replace(&mut self.value, MaybeUninit::uninit());
        unsafe { value.assume_init().value }
    }

    pub fn is_set(&self) -> bool {
        self.is_set
    }

    pub fn set_value(&mut self, index: u32, value: T) {
        if self.is_set() {
            unsafe {
                self.value.assume_init_drop();
            }
        }
        self.value.write(RowEntry::new(value, index));
        self.is_set = true;
    }
}

impl<T: Sized> Drop for MaybeUnsetEntry<T> {
    fn drop(&mut self) {
        if self.is_set() {
            unsafe {
                self.value.assume_init_drop();
            }
        }
    }
}

impl<T> AsRef<RowEntry<T>> for MaybeUnsetEntry<T> {
    fn as_ref(&self) -> &RowEntry<T> {
        assert!(self.is_set());
        unsafe { self.value.assume_init_ref() }
    }
}

impl<T> AsMut<RowEntry<T>> for MaybeUnsetEntry<T> {
    fn as_mut(&mut self) -> &mut RowEntry<T> {
        assert!(self.is_set());
        unsafe { self.value.assume_init_mut() }
    }
}

impl<T: Sized> Deref for MaybeUnsetEntry<T> {
    type Target = RowEntry<T>;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Sized> DerefMut for MaybeUnsetEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RowEntry<T> {
    value: T,
    row: u32,
}

impl<T> RowEntry<T> {
    pub fn new(value: T, index: u32) -> Self {
        Self { value, row: index }
    }
    pub fn entry_index(&self) -> u32 {
        self.row
    }
}

impl<T> Deref for RowEntry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for RowEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Display for RowEntry<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// Ptr should only ever be used as an internal type
pub(crate) struct Ptr<T>(pub Rc<RefCell<MaybeUnsetEntry<T>>>);
pub(crate) type EntList<T> = Vec<Ptr<T>>;

impl<T> Ptr<T> {
    pub fn new_unset() -> Self {
        Self(Rc::new(RefCell::new(MaybeUnsetEntry::new_unset())))
    }

    pub fn is_set(&self) -> bool {
        self.0.borrow().is_set()
    }

    pub fn _is_refrenced(&self) -> bool {
        Rc::strong_count(&self.0) > 1
    }

    pub fn set_value(&self, index: u32, value: T) {
        let mut val_ref = self.0.borrow_mut();
        val_ref.set_value(index, value);
    }

    pub fn value(&self) -> Ref<RowEntry<T>> {
        let r = self.0.borrow();
        Ref::map(r, |x| x.as_ref())
    }

    pub fn value_mut(&self) -> RefMut<T> {
        let r = self.0.borrow_mut();
        RefMut::map(r, |x| &mut x.as_mut().value)
    }

    pub fn try_value(&self) -> Option<Ref<RowEntry<T>>> {
        let r = self.0.try_borrow();
        r.ok().map(|r| Ref::map(r, |x| x.as_ref()))
    }

    pub fn try_value_mut(&self) -> Option<RefMut<T>> {
        let r = self.0.try_borrow_mut();
        r.ok().map(|r| RefMut::map(r, |x| &mut x.as_mut().value))
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Debug> Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0.as_ref().borrow();
        if v.is_set() {
            v.as_ref().fmt(f)
        } else {
            write!(f, "Ptr::new_unset()")
        }
    }
}
