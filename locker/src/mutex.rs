use std::cell::UnsafeCell;

use crate::exclusive_lock::{ExclusiveGuard, RawExclusiveGuard, RawExclusiveLock};

cfg_if::cfg_if! {
    if #[cfg(feature = "extra")] {
        pub mod global;
        pub mod spin;
        pub mod tagged;
        pub mod local_simple;
        pub mod local_tagged;
        pub mod local_splittable;

        #[cfg(feature = "parking_lot_core")]
        pub mod simple;
        #[cfg(feature = "parking_lot_core")]
        pub mod splittable;
    }
}

pub unsafe trait RawMutex: crate::RawLockInfo + RawExclusiveLock {}
#[repr(C)]
pub struct Mutex<L, T: ?Sized> {
    lock: L,
    value: UnsafeCell<T>,
}

impl<L: RawMutex, T: Default> Default for Mutex<L, T> {
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
    }
}

unsafe impl<L: Send + RawMutex, T: Send> Send for Mutex<L, T> {}
unsafe impl<L: Sync + RawMutex, T: Send> Sync for Mutex<L, T> {}

impl<L, T> Mutex<L, T> {
    /// # Safety
    ///
    /// You must pass `RawUniueLock::INIT` as lock
    #[inline]
    pub const unsafe fn from_raw_parts(lock: L, value: T) -> Self {
        Self {
            lock,
            value: UnsafeCell::new(value),
        }
    }

    #[inline]
    pub fn into_raw_parts(self) -> (L, T) {
        (self.lock, self.value.into_inner())
    }

    #[inline]
    pub fn into_mutex(self) -> crate::mutex::Mutex<L, T> {
        let (lock, value) = self.into_raw_parts();
        unsafe { crate::mutex::Mutex::from_raw_parts(lock, value) }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<L, T: ?Sized> Mutex<L, T> {
    #[inline]
    #[allow(clippy::transmute_ptr_to_ptr)]
    pub fn as_rwlock(&self) -> &crate::rwlock::RwLock<L, T> {
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    #[allow(clippy::transmute_ptr_to_ptr)]
    pub fn as_rwlock_mut(&mut self) -> &mut crate::rwlock::RwLock<L, T> {
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub const fn raw(&self) -> &L {
        &self.lock
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "nightly")] {
            #[inline]
            pub const unsafe fn raw_mut(&mut self) -> &mut L {
                &mut self.lock
            }

            #[inline]
            pub const fn get_mut(&mut self) -> &mut T {
                unsafe { &mut *self.value.get() }
            }
        } else {
            #[inline]
            pub unsafe fn raw_mut(&mut self) -> &mut L {
                &mut self.lock
            }

            #[inline]
            pub fn get_mut(&mut self) -> &mut T {
                unsafe { &mut *self.value.get() }
            }
        }
    }
}

impl<L: RawMutex, T> Mutex<L, T> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "nightly")] {
            #[inline]
            pub const fn new(value: T) -> Self {
                unsafe { Self::from_raw_parts(L::INIT, value) }
            }
        } else {
            #[inline]
            pub fn new(value: T) -> Self {
                unsafe { Self::from_raw_parts(L::INIT, value) }
            }
        }
    }
}

impl<L: RawMutex, T: ?Sized> Mutex<L, T>
where
    L::ExclusiveGuardTraits: crate::Inhabitted,
{
    #[inline]
    pub fn lock(&self) -> ExclusiveGuard<'_, L, T> {
        unsafe {
            ExclusiveGuard::from_raw_parts(RawExclusiveGuard::new(&self.lock), self.value.get())
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<ExclusiveGuard<'_, L, T>> {
        unsafe {
            Some(ExclusiveGuard::from_raw_parts(
                RawExclusiveGuard::try_new(&self.lock)?,
                self.value.get(),
            ))
        }
    }
}
