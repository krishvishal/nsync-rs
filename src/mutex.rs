use std::cell::UnsafeCell;
use std::fmt::{self};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::atomic::Ordering;

use crate::ffi;

/// A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available.
pub struct Mutex<T: ?Sized> {
    pub(super) _inner: UnsafeCell<ffi::nsync_mu>,
    poison: std::sync::atomic::AtomicBool,
    data: UnsafeCell<T>,
}

impl<T: ?Sized> Mutex<T> {
    pub(super) fn is_poisoned(&self, order: Ordering) -> bool {
        self.poison.load(order)
    }
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> UnwindSafe for Mutex<T> {}
impl<T> RefUnwindSafe for Mutex<T> {}

/// An RAII implementation of a "scoped lock" of a mutex.
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    pub(super) lock: &'a Mutex<T>,
    pub(super) poison: std::sync::atomic::Ordering,
    // !Send
    pub(super) _marker: PhantomData<*const ()>,
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // If a panic occurred, mark the mutex as poisoned
        if std::thread::panicking() {
            self.lock.poison.store(true, self.poison);
        }

        unsafe {
            ffi::nsync_mu_unlock(self.lock._inner.get());
        }
    }
}

impl<'a, T: ?Sized + 'a> MutexGuard<'a, T> {
    pub(super) fn new(lock: &'a Mutex<T>) -> LockResult<MutexGuard<'a, T>> {
        let is_poisoned = lock.poison.load(std::sync::atomic::Ordering::Relaxed);
        let guard = MutexGuard {
            lock,
            poison: std::sync::atomic::Ordering::Relaxed,
            _marker: PhantomData,
        };

        if is_poisoned {
            Err(PoisonError::new(guard)) // Still return the guard, but wrapped in an error
        } else {
            Ok(guard)
        }
    }
}

unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}
/// A type of error which can be returned whenever a lock is acquired.
#[derive(Clone)]
pub struct PoisonError<T> {
    guard: T,
}

impl<T> PoisonError<T> {
    /// Creates a new `PoisonError`.
    pub fn new(guard: T) -> PoisonError<T> {
        PoisonError { guard }
    }
    /// Consumes this error, returning the underlying guard.
    pub fn into_inner(self) -> T {
        self.guard
    }
    /// Reaches into this error, returning a reference to the underlying guard.
    pub fn get_ref(&self) -> &T {
        &self.guard
    }
    /// Reaches into this error, returning a mutable reference to the underlying guard.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

pub type LockResult<T> = Result<T, PoisonError<T>>;
pub type TryLockResult<T> = Result<T, TryLockError<T>>;

impl<T> fmt::Display for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "poisoned lock: another task failed inside")
    }
}
impl<T> std::error::Error for PoisonError<T> {}

/// An enumeration of possible errors associated with a [`TryLockResult`] which
/// can occur while trying to acquire a lock, from the [`try_lock`] method on a
/// [`Mutex`] or the [`try_read`] and [`try_write`] methods on an [`RwLock`].
///
/// [`try_lock`]: Mutex::try_lock
/// [`try_read`]: RwLock::try_read
/// [`try_write`]: RwLock::try_write
pub enum TryLockError<T> {
    /// The lock could not be acquired because another thread failed while holding
    /// the lock.
    Poisoned(PoisonError<T>),
    /// The lock could not be acquired at this time because the operation would
    /// otherwise block.
    WouldBlock,
}

impl<T> fmt::Debug for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PoisonError").finish_non_exhaustive()
    }
}

impl<T> fmt::Debug for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::Poisoned(..) => "Poisoned(..)".fmt(f),
            TryLockError::WouldBlock => "WouldBlock".fmt(f),
        }
    }
}

impl<T> fmt::Display for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::Poisoned(..) => write!(f, "poisoned lock: another task failed inside"),
            TryLockError::WouldBlock => {
                write!(f, "try_lock failed because the operation would block")
            }
        }
    }
}

impl<T> std::error::Error for TryLockError<T> {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            TryLockError::Poisoned(ref p) => Some(p),
            _ => None,
        }
    }
}

impl<T> From<PoisonError<T>> for TryLockError<T> {
    fn from(err: PoisonError<T>) -> TryLockError<T> {
        TryLockError::Poisoned(err)
    }
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    pub fn new(t: T) -> Mutex<T> {
        let mut mu = MaybeUninit::<ffi::nsync_mu>::uninit();
        unsafe {
            ffi::nsync_mu_init(mu.as_mut_ptr());
            Mutex {
                _inner: UnsafeCell::new(mu.assume_init()),
                poison: std::sync::atomic::AtomicBool::new(false),
                data: UnsafeCell::new(t),
            }
        }
    }

    /// Acquires a mutex, blocking the current thread until it is able to do so.
    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        unsafe {
            ffi::nsync_mu_lock(self._inner.get());
        }
        MutexGuard::new(self)
    }

    /// Attempts to acquire this lock.
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        unsafe {
            let ret = ffi::nsync_mu_trylock(self._inner.get());
            if ret == 0 {
                Err(TryLockError::WouldBlock)
            } else {
                match MutexGuard::new(self) {
                    Ok(guard) => Ok(guard),
                    Err(e) => Err(TryLockError::Poisoned(e)),
                }
            }
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> LockResult<T>
    where
        T: Sized,
    {
        let is_poisoned = self.poison.load(std::sync::atomic::Ordering::Relaxed);
        let data = self.data.into_inner();

        if is_poisoned {
            Err(PoisonError::new(data))
        } else {
            Ok(data)
        }
    }

    /// Returns a mutable reference to the underlying data.
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        let is_poisoned = self.poison.load(std::sync::atomic::Ordering::Relaxed);
        let data = self.data.get_mut();

        if is_poisoned {
            Err(PoisonError::new(data))
        } else {
            Ok(data)
        }
    }
}

/// A reader-writer lock
pub struct RwLock<T: ?Sized> {
    inner: UnsafeCell<ffi::nsync_mu>,
    poison: std::sync::atomic::AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
    // !Send
    _marker: PhantomData<*const ()>,
}

pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
    poison: std::sync::atomic::Ordering,
    // !Send
    _marker: PhantomData<*const ()>,
}

unsafe impl<T: ?Sized + Sync> Sync for RwLockReadGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for RwLockWriteGuard<'_, T> {}

impl<T> RwLock<T> {
    pub fn new(t: T) -> RwLock<T> {
        let mut mu = MaybeUninit::<ffi::nsync_mu>::uninit();
        unsafe {
            ffi::nsync_mu_init(mu.as_mut_ptr());
            RwLock {
                inner: UnsafeCell::new(mu.assume_init()),
                poison: std::sync::atomic::AtomicBool::new(false),
                data: UnsafeCell::new(t),
            }
        }
    }

    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        unsafe {
            ffi::nsync_mu_rlock(self.inner.get());
        }
        RwLockReadGuard::new(self)
    }

    pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
        unsafe {
            let ret = ffi::nsync_mu_rtrylock(self.inner.get());
            if ret == 0 {
                Err(TryLockError::WouldBlock)
            } else {
                match RwLockReadGuard::new(self) {
                    Ok(guard) => Ok(guard),
                    Err(e) => Err(TryLockError::Poisoned(e)),
                }
            }
        }
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        unsafe {
            ffi::nsync_mu_lock(self.inner.get());
        }
        RwLockWriteGuard::new(self)
    }

    pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
        unsafe {
            let ret = ffi::nsync_mu_trylock(self.inner.get());
            if ret == 0 {
                Err(TryLockError::WouldBlock)
            } else {
                match RwLockWriteGuard::new(self) {
                    Ok(guard) => Ok(guard),
                    Err(e) => Err(TryLockError::Poisoned(e)),
                }
            }
        }
    }
}

impl<'a, T: ?Sized> RwLockReadGuard<'a, T> {
    fn new(lock: &'a RwLock<T>) -> LockResult<RwLockReadGuard<'a, T>> {
        let is_poisoned = lock.poison.load(std::sync::atomic::Ordering::Relaxed);
        let guard = RwLockReadGuard {
            lock,
            _marker: PhantomData,
        };

        if is_poisoned {
            Err(PoisonError::new(guard))
        } else {
            Ok(guard)
        }
    }
}

impl<'a, T: ?Sized> RwLockWriteGuard<'a, T> {
    fn new(lock: &'a RwLock<T>) -> LockResult<RwLockWriteGuard<'a, T>> {
        let is_poisoned = lock.poison.load(std::sync::atomic::Ordering::Relaxed);
        let guard = RwLockWriteGuard {
            lock,
            poison: std::sync::atomic::Ordering::Relaxed,
            _marker: PhantomData,
        };

        if is_poisoned {
            Err(PoisonError::new(guard))
        } else {
            Ok(guard)
        }
    }
}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            ffi::nsync_mu_runlock(self.lock.inner.get());
        }
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.lock.poison.store(true, self.poison);
        }

        unsafe {
            ffi::nsync_mu_unlock(self.lock.inner.get());
        }
    }
}
