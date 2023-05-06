use std::{fmt::Debug, cell::{RefCell, Ref}, rc::Rc, ops::{DerefMut, Deref}, mem::MaybeUninit};

#[derive(Debug)]
pub (crate) struct MaybeUnsetEntry<T:Sized> {
    value: MaybeUninit<T>,
    is_set: bool
}

impl<T:Sized> MaybeUnsetEntry<T> {
    pub fn new_unset() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            is_set: false
        }
    }

    pub fn is_set(&self) -> bool {
        self.is_set
    }

    pub fn set_value(&mut self, value: T) {
        if self.is_set() {
            unsafe{ self.value.assume_init_drop(); }
        }
        self.value.write(value);
        self.is_set = true;
    }

}

impl<T:Sized> Drop for MaybeUnsetEntry<T> {
    fn drop(&mut self) {
        if self.is_set() {
            unsafe { self.value.assume_init_drop(); }
        }
    }
}

impl<T> AsRef<T> for MaybeUnsetEntry<T> {
    fn as_ref(&self) -> &T {
        assert!(self.is_set());
        unsafe { self.value.assume_init_ref() }
    }
}

impl<T> AsMut<T> for MaybeUnsetEntry<T> {
    fn as_mut(&mut self) -> &mut T {
        assert!(self.is_set());
        unsafe { self.value.assume_init_mut() }
    }
}

impl<T:Sized> Deref for MaybeUnsetEntry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T:Sized> DerefMut for MaybeUnsetEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}


#[derive(Debug, Copy, Clone)]
pub struct RowEntry<T> {
    value: T,
    row: u32
}

impl<T> RowEntry<T> {
    pub fn new(value: T, row: u32) -> Self {
        Self { value, row }
    }
    pub fn row(&self) -> u32 {
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




#[derive(Clone)]
pub struct Ptr<T>(Rc<RefCell<RowEntry<MaybeUnsetEntry<T>>>>);
pub(crate) type EntList<T> = Vec<Ptr<T>>;

impl<T> Ptr<T> {
    pub fn new_unset(row: u32) -> Self {
        Self(Rc::new(RefCell::new(RowEntry::new(MaybeUnsetEntry::new_unset(), row))))
    }

    pub fn is_set(&self) -> bool {
        self.0.borrow().value.is_set()
    }

    pub fn is_refrenced(&self) -> bool {
        Rc::strong_count(&self.0) > 1
    }

    pub fn set_value(&self, value: T) {
        let mut val_ref = self.0.borrow_mut();
        val_ref.value.set_value(value);
    }

    pub fn value(&self) -> Ref<T> {
        let r = self.0.borrow();
        Ref::map(r, |x| x.value.as_ref())
    }

    pub fn clone_value(&self) -> T 
    where T: Clone {
        self.0.borrow().value.clone()
    }
}

impl<T: Debug> Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0.as_ref().borrow();
        if v.value.is_set() {
            v.value.as_ref().fmt(f)
        }else{
            write!(f, "Ptr::new_unset()")
        }
    }
}
