use crate::mutex::{LockResult, MutexGuard};
use crate::time::{Duration, Time};
use crate::{PoisonError, ffi};
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::time::Duration as StdDuration;

/// A Condition Variable
pub struct Condvar {
    _inner: UnsafeCell<ffi::nsync_cv>,
}

unsafe impl Send for Condvar {}
unsafe impl Sync for Condvar {}

/// A type indicating whether a timed wait on a condition variable returned
/// due to a time out or not.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    /// Returns `true` if the wait was known to have timed out.
    pub fn timed_out(&self) -> bool {
        self.0
    }
}

impl Condvar {
    /// Creates a new condition variable which is ready to be waited on and notified.
    pub fn new() -> Condvar {
        let mut cv = MaybeUninit::<ffi::nsync_cv>::uninit();
        unsafe {
            ffi::nsync_cv_init(cv.as_mut_ptr());
            Condvar {
                _inner: UnsafeCell::new(cv.assume_init()),
            }
        }
    }

    /// Blocks the current thread until this condition variable receives a notification.
    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> LockResult<MutexGuard<'a, T>> {
        let mutex = guard.lock;
        // DON'T drop the guard, nsync expects the mutex to be held
        // The wait function will unlock it internally
        unsafe {
            // Pass the locked mutex to nsync_cv_wait
            ffi::nsync_cv_wait(self._inner.get(), mutex._inner.get());
            // nsync_cv_wait returns with the mutex locked again
        }
        std::mem::forget(guard);
        MutexGuard::new(mutex)
    }

    /// Waits on this condition variable for a notification, timing out after a specified duration.
    pub fn wait_timeout<'a, T>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: StdDuration,
    ) -> LockResult<(MutexGuard<'a, T>, WaitTimeoutResult)> {
        let mutex = guard.lock;
        let deadline = Time::now() + Duration::from(dur);

        let result = unsafe {
            // Pass the locked mutex - don't drop the guard first
            ffi::nsync_cv_wait_with_deadline(
                self._inner.get(),
                mutex._inner.get(),
                deadline.as_raw(),
                std::ptr::null_mut(),
            )
        };
        std::mem::forget(guard);

        let timed_out = result != 0;
        // The mutex is already re-locked by nsync_cv_wait_with_deadline
        let is_poisoned = mutex.is_poisoned(std::sync::atomic::Ordering::Relaxed);
        let guard = MutexGuard {
            lock: mutex,
            poison: std::sync::atomic::Ordering::Relaxed,
            _marker: PhantomData,
        };
        if is_poisoned {
            Err(PoisonError::new((guard, WaitTimeoutResult(timed_out))))
        } else {
            Ok((guard, WaitTimeoutResult(timed_out)))
        }
    }

    /// Wakes up one blocked thread on this condvar.
    pub fn notify_one(&self) {
        unsafe {
            ffi::nsync_cv_signal(self._inner.get());
        }
    }

    /// Wakes up all blocked threads on this condvar.
    pub fn notify_all(&self) {
        unsafe {
            ffi::nsync_cv_broadcast(self._inner.get());
        }
    }
}

impl Default for Condvar {
    fn default() -> Self {
        Self::new()
    }
}
